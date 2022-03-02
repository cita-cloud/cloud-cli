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
