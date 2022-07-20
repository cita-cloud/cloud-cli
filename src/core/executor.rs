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

use anyhow::{Context, Result};
use tonic::transport::Channel;

use crate::crypto::{Address, ArrayLike};
use cita_cloud_proto::executor::{CallRequest, CallResponse};

pub type ExecutorClient =
    cita_cloud_proto::executor::executor_service_client::ExecutorServiceClient<Channel>;

#[tonic::async_trait]
pub trait ExecutorBehaviour {
    async fn call(
        &self,
        from: Address,
        to: Address,
        data: Vec<u8>,
        height: u64,
    ) -> Result<CallResponse>;
}

#[tonic::async_trait]
impl ExecutorBehaviour for ExecutorClient {
    async fn call(
        &self,
        from: Address,
        to: Address,
        data: Vec<u8>,
        height: u64,
    ) -> Result<CallResponse> {
        let req = CallRequest {
            from: from.to_vec(),
            to: to.to_vec(),
            // This is `executor_evm` specific calling convention.
            // `executor_chaincode` uses args[0] for payload.
            // But since no one uses chaincode, we may just use the evm's convention.
            method: data,
            args: Vec::new(),
            height,
        };

        ExecutorClient::call(&mut self.clone(), req)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to do executor gRPC call")
    }
}
