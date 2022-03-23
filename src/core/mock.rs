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

use super::{
    client::GrpcClientBehaviour, context::Context, controller::ControllerBehaviour,
    evm::EvmBehaviour, executor::ExecutorBehaviour,
};
use crate::{
    config::Config,
    core::wallet::Account,
    crypto::{Address, Hash, SmCrypto},
    proto::{
        blockchain::{CompactBlock, RawTransaction},
        common::TotalNodeInfo,
        controller::SystemConfig,
        evm::{Balance, ByteAbi, ByteCode, Nonce, Receipt},
        executor::CallResponse,
    },
};
use anyhow::Result;
use mockall::mock;
use std::time::Duration;
use tempfile::tempdir;
use tempfile::TempDir;
use tonic::transport::Channel;

mock! {
    pub ControllerClient {}

    #[tonic::async_trait]
    impl ControllerBehaviour for ControllerClient {
        async fn send_raw(&self, raw: RawTransaction) -> Result<Hash>;

        async fn get_version(&self) -> Result<String>;
        async fn get_system_config(&self) -> Result<SystemConfig>;

        async fn get_block_number(&self, for_pending: bool) -> Result<u64>;
        async fn get_block_hash(&self, block_number: u64) -> Result<Hash>;

        async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock>;
        async fn get_block_by_hash(&self, hash: Hash) -> Result<CompactBlock>;

        async fn get_tx(&self, tx_hash: Hash) -> Result<RawTransaction>;
        async fn get_tx_index(&self, tx_hash: Hash) -> Result<u64>;
        async fn get_tx_block_number(&self, tx_hash: Hash) -> Result<u64>;

        async fn get_peer_count(&self) -> Result<u64>;
        async fn get_peers_info(&self) -> Result<TotalNodeInfo>;

        async fn add_node(&self, multiaddr: String) -> Result<u32>;
    }

    #[tonic::async_trait]
    impl GrpcClientBehaviour for ControllerClient {
        fn from_channel(ch: Channel) -> Self;
        async fn connect(addr: &str) -> Result<Self>;
        fn connect_lazy(addr: &str) -> Result<Self>;
        async fn connect_timeout(addr: &str, dur: Duration) -> Result<Self> ;
    }

    impl Clone for ControllerClient {
        fn clone(&self) -> Self;
    }
}

mock! {
    pub ExecutorClient {}

    #[tonic::async_trait]
    impl ExecutorBehaviour for ExecutorClient {
        async fn call(
            &self,
            from: Address,
            to: Address,
            data: Vec<u8>,
        ) -> Result<CallResponse>;
    }

    #[tonic::async_trait]
    impl GrpcClientBehaviour for ExecutorClient {
        fn from_channel(ch: Channel) -> Self;
        async fn connect(addr: &str) -> Result<Self>;
        fn connect_lazy(addr: &str) -> Result<Self>;
        async fn connect_timeout(addr: &str, dur: Duration) -> Result<Self> ;
    }

    impl Clone for ExecutorClient {
        fn clone(&self) -> Self;
    }
}

mock! {
    pub EvmClient {}

    #[tonic::async_trait]
    impl EvmBehaviour for EvmClient {
        async fn get_receipt(&self, hash: Hash) -> Result<Receipt>;
        async fn get_code(&self, addr: Address) -> Result<ByteCode>;
        async fn get_balance(&self, addr: Address) -> Result<Balance>;
        async fn get_tx_count(&self, addr: Address) -> Result<Nonce>;
        async fn get_abi(&self, addr: Address) -> Result<ByteAbi>;
    }

    #[tonic::async_trait]
    impl GrpcClientBehaviour for EvmClient {
        fn from_channel(ch: Channel) -> Self;
        async fn connect(addr: &str) -> Result<Self>;
        fn connect_lazy(addr: &str) -> Result<Self>;
        async fn connect_timeout(addr: &str, dur: Duration) -> Result<Self> ;
    }

    impl Clone for EvmClient {
        fn clone(&self) -> Self;
    }
}

/// Returns mock context and temp dir guard.
/// The temp dir guard must be holded to use the mock context.
pub fn context() -> (
    Context<MockControllerClient, MockExecutorClient, MockEvmClient>,
    TempDir,
) {
    // Set up mock context. Note that we don't use the provided impl
    // for the rest connect* methods since that would actually try to connect.
    let mock_ctx = MockControllerClient::from_channel_context();
    mock_ctx.expect().returning(|_| Default::default());
    let mock_ctx = MockControllerClient::connect_context();
    mock_ctx.expect().returning(|_| Ok(Default::default()));
    let mock_ctx = MockControllerClient::connect_lazy_context();
    mock_ctx.expect().returning(|_| Ok(Default::default()));
    let mock_ctx = MockControllerClient::connect_timeout_context();
    mock_ctx.expect().returning(|_, _| Ok(Default::default()));

    let mock_ctx = MockExecutorClient::from_channel_context();
    mock_ctx.expect().returning(|_| Default::default());
    let mock_ctx = MockExecutorClient::connect_context();
    mock_ctx.expect().returning(|_| Ok(Default::default()));
    let mock_ctx = MockExecutorClient::connect_lazy_context();
    mock_ctx.expect().returning(|_| Ok(Default::default()));
    let mock_ctx = MockExecutorClient::connect_timeout_context();
    mock_ctx.expect().returning(|_, _| Ok(Default::default()));

    let mock_ctx = MockEvmClient::from_channel_context();
    mock_ctx.expect().returning(|_| Default::default());
    let mock_ctx = MockEvmClient::connect_context();
    mock_ctx.expect().returning(|_| Ok(Default::default()));
    let mock_ctx = MockEvmClient::connect_lazy_context();
    mock_ctx.expect().returning(|_| Ok(Default::default()));
    let mock_ctx = MockEvmClient::connect_timeout_context();
    mock_ctx.expect().returning(|_, _| Ok(Default::default()));

    let test_dir = tempdir().expect("cannot get temp dir");
    let config = Config {
        data_dir: test_dir.path().to_path_buf(),
        ..Default::default()
    };

    let mut ctx = Context::from_config(config).expect("fail to create test context");

    let default_account = Account::<SmCrypto>::generate();
    ctx.wallet
        .save("default".into(), default_account)
        .expect("cannot save default account");

    (ctx, test_dir)
}
