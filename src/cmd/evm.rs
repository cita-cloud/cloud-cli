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
pub trait EvmBehaviour {
    async fn get_receipt(&self, hash: Vec<u8>) -> Receipt;
    async fn get_code(&self, address: Vec<u8>) -> ByteCode;
    async fn get_balance(&self, address: Vec<u8>) -> Balance;
    async fn get_transaction_count(&self, address: Vec<u8>) -> Nonce;
    async fn get_abi(&self, address: Vec<u8>) -> ByteAbi;
}


#[tonic::async_trait]
impl EvmBehaviour for Context {
    async fn get_receipt(&self, hash: Vec<u8>) -> Receipt {
        let hash = Hash { hash };
        self.evm
            .clone()
            .get_transaction_receipt(hash)
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_code(&self, address: Vec<u8>) -> ByteCode {
        let addr = Address { address };
        self.evm.clone().get_code(addr).await.unwrap().into_inner()
    }

    async fn get_balance(&self, address: Vec<u8>) -> Balance {
        let addr = Address { address };
        self.evm
            .clone()
            .get_balance(addr)
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_transaction_count(&self, address: Vec<u8>) -> Nonce {
        let addr = Address { address };
        self.evm
            .clone()
            .get_transaction_count(addr)
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_abi(&self, address: Vec<u8>) -> ByteAbi {
        let addr = Address { address };
        self.evm.clone().get_abi(addr).await.unwrap().into_inner()
    }
}
