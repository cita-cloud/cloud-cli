use crate::crypto::Crypto;

use tonic::transport::channel::Channel;

use crate::proto::{
    blockchain::{
        raw_transaction::Tx, CompactBlock, RawTransaction, Transaction as CloudTransaction,
        UnverifiedTransaction, UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction,
        Witness,
    },
    common::{Address, Empty, Hash, NodeInfo, NodeNetInfo},
    controller::{
        BlockNumber, Flag, SystemConfig,
        TransactionIndex,
    },
    evm::{
        Balance, ByteAbi, ByteCode, Nonce,
        Receipt,
    },
    executor::{CallRequest},
};

use super::controller::{
    ControllerClient, ControllerBehaviour,
};
use super::executor::ExecutorClient;
use super::evm::EvmClient;
// use super::controller::ControllerClient;
use anyhow::Result;
use super::wallet::Wallet;


// #[derive(Clone)]
pub struct Context {
    pub system_config: SystemConfig,
    pub wallet: Wallet,

    /// Those gRPC client are connected lazily.
    pub controller: ControllerClient,
    pub executor: ExecutorClient,
    pub evm: EvmClient,

    pub rt: tokio::runtime::Handle,
}

// I miss [Delegation](https://github.com/contactomorph/rfcs/blob/delegation/text/0000-delegation-of-implementation.md)
// Most of the code below is boilerplate.
// TODO: write a macro for this or use ambassador

#[tonic::async_trait]
impl<C: Crypto> ControllerBehaviour<C> for Context {
    async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash> {
        <ControllerClient as ControllerBehaviour<C>>::send_raw(&self.controller, raw).await
    }

    async fn get_system_config(&self) -> Result<SystemConfig> {
        <ControllerClient as ControllerBehaviour<C>>::get_system_config(&self.controller).await
    }

    async fn get_block_number(&self, for_pending: bool) -> Result<u64> {
        <ControllerClient as ControllerBehaviour<C>>::get_block_number(&self.controller, for_pending).await
    }
    async fn get_block_hash(&self, block_number: u64) -> Result<C::Hash> {
        <ControllerClient as ControllerBehaviour<C>>::get_block_hash(&self.controller, block_number).await
    }

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock> {
        <ControllerClient as ControllerBehaviour<C>>::get_block_by_number(&self.controller, block_number).await
    }

    async fn get_block_by_hash(&self, hash: C::Hash) -> Result<CompactBlock> {
        <ControllerClient as ControllerBehaviour<C>>::get_block_by_hash(&self.controller, hash).await
    }

    async fn get_tx(&self, tx_hash: C::Hash) -> Result<RawTransaction> {
        <ControllerClient as ControllerBehaviour<C>>::get_tx(&self.controller, tx_hash).await
    }

    async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<u64> {
        <ControllerClient as ControllerBehaviour<C>>::get_tx_index(&self.controller, tx_hash).await
    }

    async fn get_tx_block_number(&self, tx_hash: C::Hash) -> Result<u64> {
        <ControllerClient as ControllerBehaviour<C>>::get_tx_block_number(&self.controller, tx_hash).await
    }

    async fn get_peer_count(&self) -> Result<u64> {
        <ControllerClient as ControllerBehaviour<C>>::get_peer_count(&self.controller).await
    }

    async fn get_peers_info(&self) -> Result<Vec<NodeInfo>> {
        <ControllerClient as ControllerBehaviour<C>>::get_peers_info(&self.controller).await
    }

    async fn add_node(&self, multiaddr: String) -> Result<u32> {
        <ControllerClient as ControllerBehaviour<C>>::add_node(&self.controller, multiaddr).await
    }
}

