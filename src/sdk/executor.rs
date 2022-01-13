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

use crate::crypto::Crypto;
use anyhow::Result;

#[tonic::async_trait]
pub trait ExecutorBehaviour {
    type Address;

    async fn call(&self, from: Address, to: Address, payload: Vec<u8>) -> Result<Vec<u8>>;
}

#[tonic::async_trait]
impl<C: Crypto> ExecutorBehaviour for Context<C> {
    type Address = C::Address;

    async fn call(&self, from: Self::Address, to: Self::Address, payload: Vec<u8>) -> Result<Vec<u8>> {
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
            .map(|resp| resp.into_inner().value)
            .context("failed to do executor call")
    }
}
