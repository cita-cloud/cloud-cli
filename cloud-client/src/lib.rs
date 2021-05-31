// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Those addtional lets make the code more readable.
#![allow(clippy::let_and_return)]

mod crypto;
mod display;

use prost::Message;
use tonic::transport::channel::Channel;
use tonic::transport::channel::Endpoint;
use tonic::Request;

use cita_cloud_proto::executor::executor_service_client::ExecutorServiceClient as ExecutorClient;

use cita_cloud_proto::executor::CallRequest;

use cita_cloud_proto::blockchain::{
    CompactBlock, Transaction as CloudTransaction, UnverifiedTransaction, Witness,
};
use cita_cloud_proto::common::Address;
use cita_cloud_proto::common::Empty;
use cita_cloud_proto::common::Hash;
use cita_cloud_proto::controller::{
    raw_transaction::Tx, rpc_service_client::RpcServiceClient as ControllerClient, BlockNumber,
    Flag, RawTransaction, SystemConfig,
};

use cita_cloud_proto::evm::{
    rpc_service_client::RpcServiceClient as EvmClient, Balance, ByteAbi, ByteCode, Receipt,
};

use crypto::generate_keypair;
use crypto::hash_data;
use crypto::pk2address;
use crypto::sign_message;

use tokio::sync::OnceCell;

pub use display::Display;

// TODO: use better name
struct Wallet {
    account_addr: Vec<u8>,
    keypair: (Vec<u8>, Vec<u8>),
    chain_id: Vec<u8>,
}

pub struct Client {
    controller: ControllerClient<Channel>,
    executor: ExecutorClient<Channel>,

    #[cfg(feature = "evm")]
    evm: EvmClient<Channel>,

    // This wallet's init requires controller available,
    // but sometimes we may just want to test executor.
    // So initializing this only when required.
    wallet: OnceCell<Wallet>,
}

impl Client {
    pub fn new(controller_addr: &str, executor_addr: &str) -> Self {
        let controller = {
            let addr = format!("http://{}", controller_addr);
            let channel = Endpoint::from_shared(addr).unwrap().connect_lazy().unwrap();
            ControllerClient::new(channel)
        };
        let executor = {
            let addr = format!("http://{}", executor_addr);
            let channel = Endpoint::from_shared(addr).unwrap().connect_lazy().unwrap();
            ExecutorClient::new(channel)
        };

        #[cfg(feature = "evm")]
        let evm = {
            // use the same addr as executor
            let addr = format!("http://{}", executor_addr);
            let channel = Endpoint::from_shared(addr).unwrap().connect_lazy().unwrap();
            EvmClient::new(channel)
        };

        Self {
            controller,
            executor,
            #[cfg(feature = "evm")]
            evm,
            wallet: OnceCell::new(),
        }
    }

    async fn wallet(&self) -> &Wallet {
        let mut controller = self.controller.clone();
        let init_wallet = || async move {
            // generate key pair for signing tx
            let keypair = generate_keypair();
            let account_addr = pk2address(&keypair.0);

            let chain_id = {
                // get system config
                let sys_config = {
                    let request = Request::new(Empty {});
                    controller
                        .get_system_config(request)
                        .await
                        .unwrap()
                        .into_inner()
                };
                sys_config.chain_id
            };

            Wallet {
                keypair,
                account_addr,
                chain_id,
            }
        };

        self.wallet.get_or_init(init_wallet).await
    }

    pub async fn call(&self, from: Vec<u8>, to: Vec<u8>, payload: Vec<u8>) -> Vec<u8> {
        let req = {
            #[cfg(feature = "chaincode")]
            let call_req = CallRequest {
                from,
                to,
                args: vec![payload],
                ..Default::default()
            };
            #[cfg(feature = "evm")]
            let call_req = CallRequest {
                from,
                to,
                method: payload,
                args: vec![],
            };
            Request::new(call_req)
        };

        self.executor
            .clone()
            .call(req)
            .await
            .unwrap()
            .into_inner()
            .value
    }

    pub async fn send(&self, to: Vec<u8>, data: Vec<u8>, value: Vec<u8>) -> Vec<u8> {
        let normal_tx = self.prepare_normal_tx(to, data, value).await;
        self.controller
            .clone()
            .send_raw_transaction(normal_tx)
            .await
            .unwrap()
            .into_inner()
            .hash
    }

    async fn prepare_normal_tx(
        &self,
        to: Vec<u8>,
        data: Vec<u8>,
        value: Vec<u8>,
    ) -> RawTransaction {
        // build tx
        // get start block number
        let start_block_number = {
            let request = Request::new(Flag { flag: false });
            self.controller
                .clone()
                .get_block_number(request)
                .await
                .unwrap()
                .into_inner()
                .block_number
        };

        let Wallet {
            account_addr,
            keypair,
            chain_id,
        } = self.wallet().await;

        let tx = {
            let nonce = rand::random::<u64>().to_string();
            CloudTransaction {
                version: 0,
                to,
                nonce,
                quota: 3_000_000,
                valid_until_block: start_block_number + 99,
                data,
                value,
                chain_id: chain_id.clone(),
            }
        };

        // calc tx hash
        let tx_hash = {
            // build tx bytes
            let tx_bytes = {
                let mut buf = Vec::with_capacity(tx.encoded_len());
                tx.encode(&mut buf).unwrap();
                buf
            };
            hash_data(tx_bytes.as_slice())
        };

        // sign tx hash
        let signature = sign_message(&keypair.0, &keypair.1, &tx_hash).unwrap();

        // build raw tx
        let raw_tx = {
            let witness = Witness {
                signature,
                sender: account_addr.clone(),
            };

            let unverified_tx = UnverifiedTransaction {
                transaction: Some(tx),
                transaction_hash: tx_hash,
                witness: Some(witness),
            };

            RawTransaction {
                tx: Some(Tx::NormalTx(unverified_tx)),
            }
        };

        raw_tx
    }

    pub async fn get_system_config(&self) -> SystemConfig {
        self.controller
            .clone()
            .get_system_config(Empty {})
            .await
            .unwrap()
            .into_inner()
    }

    pub async fn get_block_number(&self, for_pending: bool) -> u64 {
        let flag = Flag { flag: for_pending };
        self.controller
            .clone()
            .get_block_number(flag)
            .await
            .unwrap()
            .into_inner()
            .block_number
    }

    pub async fn get_block_by_number(&self, block_number: u64) -> CompactBlock {
        let block_number = BlockNumber { block_number };
        self.controller
            .clone()
            .get_block_by_number(block_number)
            .await
            .unwrap()
            .into_inner()
    }

    pub async fn get_peer_count(&self) -> u64 {
        self.controller
            .clone()
            .get_peer_count(Empty {})
            .await
            .unwrap()
            .into_inner()
            .peer_count
    }

    pub async fn get_tx(&self, tx_hash: Vec<u8>) -> RawTransaction {
        let tx_hash = Hash { hash: tx_hash };
        self.controller
            .clone()
            .get_transaction(tx_hash)
            .await
            .unwrap()
            .into_inner()
    }
}

#[cfg(feature = "evm")]
impl Client {
    pub async fn get_receipt(&self, hash: Vec<u8>) -> Receipt {
        let hash = Hash { hash };
        self.evm
            .clone()
            .get_transaction_receipt(hash)
            .await
            .unwrap()
            .into_inner()
    }

    pub async fn get_code(&self, address: Vec<u8>) -> ByteCode {
        let addr = Address { address };
        self.evm.clone().get_code(addr).await.unwrap().into_inner()
    }

    pub async fn get_balance(&self, address: Vec<u8>) -> Balance {
        let addr = Address { address };
        self.evm
            .clone()
            .get_balance(addr)
            .await
            .unwrap()
            .into_inner()
    }

    pub async fn get_abi(&self, address: Vec<u8>) -> ByteAbi {
        let addr = Address { address };
        self.evm.clone().get_abi(addr).await.unwrap().into_inner()
    }
}
