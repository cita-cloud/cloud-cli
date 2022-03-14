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

use anyhow::Result;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::transport::Endpoint;

use super::{controller::ControllerClient, evm::EvmClient, executor::ExecutorClient};

#[tonic::async_trait]
pub trait GrpcClientBehaviour: Sized {
    fn from_channel(ch: Channel) -> Self;

    async fn connect(addr: &str) -> Result<Self> {
        let addr = format!("http://{addr}");
        let ch = Endpoint::from_shared(addr)?.connect().await?;
        Ok(Self::from_channel(ch))
    }

    // TODO: maybe add async.
    // Endpoint::connect_lazy, although no async fn, does require running in a async runtime
    fn connect_lazy(addr: &str) -> Result<Self> {
        let addr = format!("http://{addr}");
        let ch = Endpoint::from_shared(addr)?.connect_lazy();
        Ok(Self::from_channel(ch))
    }

    async fn connect_timeout(addr: &str, dur: Duration) -> Result<Self> {
        let addr = format!("http://{addr}");
        let ch = Endpoint::from_shared(addr)?.timeout(dur).connect().await?;
        Ok(Self::from_channel(ch))
    }
}

#[tonic::async_trait]
impl GrpcClientBehaviour for ControllerClient {
    fn from_channel(ch: Channel) -> Self {
        Self::new(ch)
    }
}

#[tonic::async_trait]
impl GrpcClientBehaviour for ExecutorClient {
    fn from_channel(ch: Channel) -> Self {
        Self::new(ch)
    }
}

#[tonic::async_trait]
impl GrpcClientBehaviour for EvmClient {
    fn from_channel(ch: Channel) -> Self {
        Self::new(ch)
    }
}
