use prost::Message;
use crate::context::Context;
use crate::wallet::Account;

use crate::proto::{
    blockchain::{
        raw_transaction::Tx, CompactBlock, RawTransaction, Transaction as CloudTransaction,
        UnverifiedTransaction, UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction,
        Witness,
    },
    common::{Address, Empty, Hash, NodeInfo, NodeNetInfo},
    controller::{
        rpc_service_client::RpcServiceClient as ControllerClient, BlockNumber, Flag, SystemConfig,
        TransactionIndex,
    },
    evm::{
        rpc_service_client::RpcServiceClient as EvmClient, Balance, ByteAbi, ByteCode, Nonce,
        Receipt,
    },
    executor::{executor_service_client::ExecutorServiceClient as ExecutorClient, CallRequest},
};

use crate::crypto::{ hash_data, sign_message };

#[tonic::async_trait]
pub trait ControllerBehaviour {
    async fn send_tx(&self, tx: CloudTransaction) -> Vec<u8> ;
    async fn send_utxo(&self, utxo: CloudUtxoTransaction) -> Vec<u8>;

    async fn get_system_config(&self) -> SystemConfig;

    async fn get_block_number(&self, for_pending: bool) -> u64;
    async fn get_block_hash(&self, block_number: u64) -> Vec<u8>;

    async fn get_block_by_number(&self, block_number: u64) -> CompactBlock;
    async fn get_block_by_hash(&self, hash: Vec<u8>) -> CompactBlock;

    async fn get_tx(&self, tx_hash: Vec<u8>) -> RawTransaction;
    async fn get_tx_index(&self, tx_hash: Vec<u8>) -> TransactionIndex;
    async fn get_tx_block_number(&self, tx_hash: Vec<u8>) -> BlockNumber;

    async fn get_peer_count(&self) -> u64;
    async fn get_peers_info(&self) -> Vec<NodeInfo>;

    async fn add_node(&self, address: String) -> u32;
}

impl Context {
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

    async fn send_raw(&self, raw: RawTransaction) -> Vec<u8> {
        self.controller
            .clone()
            .send_raw_transaction(raw)
            .await
            .unwrap()
            .into_inner()
            .hash
    }
}

#[tonic::async_trait]
impl ControllerBehaviour for Context {
    async fn send_tx(&self, tx: CloudTransaction) -> Vec<u8> {
        let raw = self.prepare_raw_tx(tx);
        self.send_raw(raw).await
    }

    async fn send_utxo(&self, utxo: CloudUtxoTransaction) -> Vec<u8> {
        let raw = self.prepare_raw_utxo(utxo);
        self.send_raw(raw).await
    }

    async fn get_system_config(&self) -> SystemConfig {
        self.controller
            .clone()
            .get_system_config(Empty {})
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_block_number(&self, for_pending: bool) -> u64 {
        let flag = Flag { flag: for_pending };
        self.controller
            .clone()
            .get_block_number(flag)
            .await
            .unwrap()
            .into_inner()
            .block_number
    }

    async fn get_block_hash(&self, block_number: u64) -> Vec<u8> {
        self.controller
            .clone()
            .get_block_hash(BlockNumber { block_number })
            .await
            .unwrap()
            .into_inner()
            .hash
    }

    async fn get_block_by_number(&self, block_number: u64) -> CompactBlock {
        self.controller
            .clone()
            .get_block_by_number(BlockNumber { block_number })
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_block_by_hash(&self, hash: Vec<u8>) -> CompactBlock {
        self.controller
            .clone()
            .get_block_by_hash(Hash { hash })
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_tx(&self, tx_hash: Vec<u8>) -> RawTransaction {
        self.controller
            .clone()
            .get_transaction(Hash { hash: tx_hash })
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_tx_index(&self, tx_hash: Vec<u8>) -> TransactionIndex {
        self.controller
            .clone()
            .get_transaction_index(Hash { hash: tx_hash })
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_tx_block_number(&self, tx_hash: Vec<u8>) -> BlockNumber {
        self.controller
            .clone()
            .get_transaction_block_number(Hash { hash: tx_hash })
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_peer_count(&self) -> u64 {
        self.controller
            .clone()
            .get_peer_count(Empty {})
            .await
            .unwrap()
            .into_inner()
            .peer_count
    }

    async fn get_peers_info(&self) -> Vec<NodeInfo> {
        self.controller
            .clone()
            .get_peers_info(Empty {})
            .await
            .unwrap()
            .into_inner()
            .nodes
    }

    async fn add_node(&self, address: String) -> u32 {
        self.controller
            .clone()
            .add_node(NodeNetInfo { multi_address: address, origin: 0 })
            .await
            .unwrap()
            .into_inner()
            .code
    }
}
