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
    client::GrpcClientBehaviour,
    context::Context,
    controller::{CompactBlockWithStaterootProof, ControllerBehaviour, ProofWithValidators},
    evm::EvmBehaviour,
    executor::ExecutorBehaviour,
};
use crate::{
    config::Config,
    core::wallet::Account,
    crypto::{Address, Hash, SmCrypto},
};
use anyhow::Result;
use cita_cloud_proto::{
    blockchain::{Block, CompactBlock, RawTransaction},
    common::NodeStatus,
    controller::{BlockNumber, SystemConfig},
    evm::{Balance, ByteAbi, ByteCode, ByteQuota, Nonce, Receipt},
    executor::CallResponse,
};
use mockall::mock;
use tempfile::tempdir;
use tempfile::TempDir;
use tonic::transport::Channel;

mock! {
    pub ControllerClient {}

    #[tonic::async_trait]
    impl ControllerBehaviour for ControllerClient {
        async fn send_raw(&self, raw: RawTransaction) -> Result<Hash>;

        async fn get_system_config(&self) -> Result<SystemConfig>;
        async fn get_system_config_by_number(&self, block_number: u64) -> Result<SystemConfig>;

        async fn get_block_number(&self, for_pending: bool) -> Result<u64>;
        async fn get_block_hash(&self, block_number: u64) -> Result<Hash>;

        async fn get_height_by_hash(&self, hash: Hash) -> Result<BlockNumber>;
        async fn get_compact_block_by_number(&self, block_number: u64) -> Result<CompactBlock>;
        async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlockWithStaterootProof>;
        async fn get_block_detail_by_number(&self, block_number: u64) -> Result<Block>;

        async fn get_tx(&self, tx_hash: Hash) -> Result<RawTransaction>;
        async fn get_tx_index(&self, tx_hash: Hash) -> Result<u64>;
        async fn get_tx_block_number(&self, tx_hash: Hash) -> Result<u64>;

        async fn get_node_status(&self) -> Result<NodeStatus>;

        async fn add_node(&self, multiaddr: String) -> Result<u32>;
        async fn parse_overlord_proof(&self, proof_bytes: Vec<u8>) -> Result<ProofWithValidators>;
    }

    impl Clone for ControllerClient {
        fn clone(&self) -> Self;
    }
}

impl GrpcClientBehaviour for MockControllerClient {
    fn from_channel(_ch: Channel) -> Self {
        MockControllerClient::default()
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
            height: u64
        ) -> Result<CallResponse>;
    }

    impl Clone for ExecutorClient {
        fn clone(&self) -> Self;
    }
}

impl GrpcClientBehaviour for MockExecutorClient {
    fn from_channel(_ch: Channel) -> Self {
        MockExecutorClient::default()
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
        async fn estimate_quota(
            &self,
            from: Vec<u8>,
            to: Vec<u8>,
            method: Vec<u8>,
        ) -> Result<ByteQuota>;
    }

    impl Clone for EvmClient {
        fn clone(&self) -> Self;
    }
}

impl GrpcClientBehaviour for MockEvmClient {
    fn from_channel(_ch: Channel) -> Self {
        MockEvmClient::default()
    }
}

/// Returns mock context and temp dir guard.
/// The temp dir guard must be holded to use the mock context.
pub fn context() -> (
    Context<MockControllerClient, MockExecutorClient, MockEvmClient>,
    TempDir,
) {
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
