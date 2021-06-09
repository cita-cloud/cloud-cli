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

use tokio::sync::OnceCell;

use prost::Message;

use tonic::transport::channel::Channel;
use tonic::transport::channel::Endpoint;
use tonic::Request;

use cita_cloud_proto::executor::executor_service_client::ExecutorServiceClient as ExecutorClient;

use cita_cloud_proto::executor::CallRequest;

use cita_cloud_proto::blockchain::{
    CompactBlock, Transaction as CloudTransaction, UnverifiedTransaction,
    UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction, Witness,
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

use crate::crypto::hash_data;
use crate::crypto::sign_message;

use crate::wallet::Account;

pub struct Client {
    controller: ControllerClient<Channel>,
    executor: ExecutorClient<Channel>,

    #[cfg(feature = "evm")]
    evm: EvmClient<Channel>,

    account: Account,

    sys_config: OnceCell<SystemConfig>,
}

impl Client {
    pub fn new(account: Account, controller_addr: &str, executor_addr: &str) -> Self {
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
            account,
            sys_config: OnceCell::new(),
        }
    }

    async fn sys_config(&self) -> &SystemConfig {
        let mut controller = self.controller.clone();
        let get_sys_config = || async move {
            controller
                .get_system_config(Empty {})
                .await
                .unwrap()
                .into_inner()
        };

        self.sys_config.get_or_init(get_sys_config).await
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
        let normal_tx = self.build_normal_tx(to, data, value).await;
        let raw_tx = self.prepare_raw_tx(normal_tx);
        self.send_raw(raw_tx).await
    }

    async fn send_raw(&self, raw: RawTransaction) -> Vec<u8> {
        self.controller
            .clone()
            .send_raw_transaction(raw)
            .await
            .unwrap()
            .into_inner()
            .hash
    }

    async fn send_raw_utxo(&mut self, raw: RawTransaction) -> Vec<u8> {
        // invalidate current sys_config
        self.sys_config.take();
        self.controller
            .clone()
            .send_raw_transaction(raw)
            .await
            .unwrap()
            .into_inner()
            .hash
    }

    async fn build_normal_tx(
        &self,
        to: Vec<u8>,
        data: Vec<u8>,
        value: Vec<u8>,
    ) -> CloudTransaction {
        // get start block number
        let start_block_number = self.get_block_number(false).await;
        let sys_config = self.sys_config().await;
        let nonce = rand::random::<u64>().to_string();
        CloudTransaction {
            version: sys_config.version,
            to,
            nonce,
            quota: 3_000_000,
            valid_until_block: start_block_number + 99,
            data,
            value,
            chain_id: sys_config.chain_id.to_vec(),
        }
    }

    fn prepare_raw_tx(&self, tx: CloudTransaction) -> RawTransaction {
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
        let Account { addr, keypair } = &self.account;
        let signature = sign_message(&keypair.0, &keypair.1, &tx_hash);

        // build raw tx
        let raw_tx = {
            let witness = Witness {
                signature,
                sender: addr.to_vec(),
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

    pub async fn set_block_interval(&mut self, block_interval: u64) -> Vec<u8> {
        let utxo = self.build_set_block_interval_utxo(block_interval).await;
        let raw = self.prepare_raw_utxo(utxo);
        self.send_raw_utxo(raw).await
    }

    pub async fn emergency_brake(&mut self, switch: bool) -> Vec<u8> {
        let utxo = self.build_emergency_brake_utxo(switch).await;
        let raw = self.prepare_raw_utxo(utxo);
        self.send_raw_utxo(raw).await
    }

    pub async fn update_admin(&mut self, admin_addr: Vec<u8>) -> Vec<u8> {
        let utxo = self.build_update_admin_utxo(admin_addr).await;
        let raw = self.prepare_raw_utxo(utxo);
        self.send_raw_utxo(raw).await
    }

    pub async fn update_validators(&mut self, validators: &[Vec<u8>]) -> Vec<u8> {
        let utxo = self.build_update_validators_utxo(&validators).await;
        let raw = self.prepare_raw_utxo(utxo);
        self.send_raw_utxo(raw).await
    }

    async fn build_set_block_interval_utxo(&self, block_interval: u64) -> CloudUtxoTransaction {
        let output = block_interval.to_be_bytes().to_vec();
        let sys_config = self.sys_config().await;

        CloudUtxoTransaction {
            version: sys_config.version,
            pre_tx_hash: sys_config.block_interval_pre_hash.clone(),
            output,
            lock_id: 1003,
        }
    }

    async fn build_emergency_brake_utxo(&self, switch: bool) -> CloudUtxoTransaction {
        let output = if switch { vec![0] } else { vec![] };
        let sys_config = self.sys_config().await;

        CloudUtxoTransaction {
            version: sys_config.version,
            pre_tx_hash: sys_config.emergency_brake_pre_hash.clone(),
            output,
            lock_id: 1005,
        }
    }

    async fn build_update_admin_utxo(&self, admin_addr: Vec<u8>) -> CloudUtxoTransaction {
        let output = admin_addr;
        let sys_config = self.sys_config().await;

        CloudUtxoTransaction {
            version: sys_config.version,
            pre_tx_hash: sys_config.admin_pre_hash.clone(),
            output,
            lock_id: 1002,
        }
    }

    async fn build_update_validators_utxo(&self, validators: &[Vec<u8>]) -> CloudUtxoTransaction {
        let output = validators.concat();
        let sys_config = self.sys_config().await;

        CloudUtxoTransaction {
            version: sys_config.version,
            pre_tx_hash: sys_config.validators_pre_hash.clone(),
            output,
            lock_id: 1004,
        }
    }

    fn prepare_raw_utxo(&self, utxo: CloudUtxoTransaction) -> RawTransaction {
        // calc utxo hash
        let utxo_hash = {
            // build utxo bytes
            let utxo_bytes = {
                let mut buf = Vec::with_capacity(utxo.encoded_len());
                utxo.encode(&mut buf).unwrap();
                buf
            };
            hash_data(utxo_bytes.as_slice())
        };

        // sign utxo hash
        let Account { addr, keypair } = &self.account;
        let signature = sign_message(&keypair.0, &keypair.1, &utxo_hash);

        // build raw utxo
        let raw_utxo = {
            let witness = Witness {
                signature,
                sender: addr.to_vec(),
            };

            let unverified_utxo = UnverifiedUtxoTransaction {
                transaction: Some(utxo),
                transaction_hash: utxo_hash,
                witnesses: vec![witness],
            };

            RawTransaction {
                tx: Some(Tx::UtxoTx(unverified_utxo)),
            }
        };

        raw_utxo
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
        self.controller
            .clone()
            .get_block_by_number(BlockNumber { block_number })
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
