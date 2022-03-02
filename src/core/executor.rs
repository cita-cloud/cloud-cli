use anyhow::{Context, Result};
use tonic::transport::Channel;

use crate::{
    crypto::{Address, ArrayLike},
    proto::executor::{CallRequest, CallResponse},
};

pub type ExecutorClient =
    crate::proto::executor::executor_service_client::ExecutorServiceClient<Channel>;

#[tonic::async_trait]
pub trait ExecutorBehaviour {
    async fn call(&self, from: Address, to: Address, data: Vec<u8>) -> Result<CallResponse>;
}

#[tonic::async_trait]
impl ExecutorBehaviour for ExecutorClient {
    async fn call(&self, from: Address, to: Address, data: Vec<u8>) -> Result<CallResponse> {
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
