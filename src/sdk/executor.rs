use prost::Message;
// use crate::context::Context;
// use crate::wallet::Account;
use anyhow::Context;

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
    executor::{executor_service_client::ExecutorServiceClient, CallRequest, CallResponse},
};

use crate::crypto::ArrayLike;
use crate::crypto::Crypto;
use anyhow::Result;
use tonic::transport::Channel;

pub type ExecutorClient =
    crate::proto::executor::executor_service_client::ExecutorServiceClient<Channel>;

#[tonic::async_trait]
pub trait ExecutorBehaviour<C: Crypto> {
    async fn call(
        &self,
        from: C::Address,
        to: C::Address,
        data: Vec<u8>,
    ) -> Result<CallResponse>;
}

#[tonic::async_trait]
impl<C: Crypto> ExecutorBehaviour<C> for ExecutorClient {
    async fn call(
        &self,
        from: C::Address,
        to: C::Address,
        data: Vec<u8>,
    ) -> Result<CallResponse> {
        let req = CallRequest {
            from: from.to_vec(),
            to: to.to_vec(),
            // This is `executor_evm` specific calling convention.
            // `executor_chaincode` uses args[0] for payload.
            // But since no one uses chaincode, we may just use the evm's convention.
            method: data,
            args: vec![],
        };

        ExecutorClient::call(&mut self.clone(), req)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to do executor gRpc call")
    }
}
