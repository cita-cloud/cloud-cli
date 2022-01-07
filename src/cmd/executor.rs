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
pub trait ExecutorBehaviour {
    async fn call(&self, from: Vec<u8>, to: Vec<u8>, payload: Vec<u8>) -> Vec<u8>;
}

#[tonic::async_trait]
impl ExecutorBehaviour for Context {
    async fn call(&self, from: Vec<u8>, to: Vec<u8>, payload: Vec<u8>) -> Vec<u8> {
        #[cfg(feature = "chaincode")]
        let req = CallRequest {
            from,
            to,
            args: vec![payload],
            ..Default::default()
        };
        #[cfg(feature = "evm")]
        let req = CallRequest {
            from,
            to,
            method: payload,
            args: vec![],
        };

        self.executor
            .clone()
            .call(req)
            .await
            .unwrap()
            .into_inner()
            .value
    }
}
