use super::client::GrpcClientBehaviour;
use super::context::Context;
use super::controller::ControllerBehaviour;
use super::evm::EvmBehaviour;
use super::executor::ExecutorBehaviour;
use crate::config::Config;
use crate::crypto::{Address, Hash, Crypto, SmCrypto};
use crate::proto::blockchain::CompactBlock;
use crate::proto::blockchain::RawTransaction;
use crate::proto::common::TotalNodeInfo;
use crate::proto::controller::SystemConfig;
use crate::proto::evm::Balance;
use crate::proto::evm::ByteAbi;
use crate::proto::evm::ByteCode;
use crate::proto::evm::Nonce;
use crate::proto::evm::Receipt;
use crate::proto::executor::CallResponse;
use crate::core::wallet::Account;
use anyhow::Result;
use mockall::mock;
use tempfile::tempdir;
use tempfile::TempDir;
use tonic::transport::Channel;
use std::time::Duration;

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
    // set up mock context
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
    let mut config = Config::default();
    config.data_dir = test_dir.path().to_path_buf();

    let mut ctx = Context::from_config(config).expect("fail to create test context");

    let default_account = Account::<SmCrypto>::generate();
    ctx.wallet.save("default", default_account).expect("cannot save default account");

    (ctx, test_dir)
}
