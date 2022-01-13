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

use super::controller::ControllerClient;
use super::executor::ExecutorClient;
use super::evm::EvmClient;
// use super::controller::ControllerClient;


#[derive(Clone)]
pub struct Context<C: Crypto> {
    pub system_config: SystemConfig,

    /// Those gRPC client are connected lazily.
    pub controller: ControllerClient,
    pub executor: ExecutorClient,
    pub evm: EvmClient,

    pub rt: tokio::runtime::Handle,
}

impl<C: Crypto> Context<C> {
    pub fn new() -> Self {
        todo!()
    }

    pub fn with_account(self) -> Self {
        todo!()
    }

    pub fn with_wallet(self) -> Self {
        todo!()
    }

    pub fn with_system_config(self) -> Self {
        todo!()
    }

    pub fn with_controller(self) -> Self {
        todo!()
    }

    pub fn with_evm(self) -> Self {
        todo!()
    }

    // pub fn account(&self) -> &Account {
    //     todo!()
    // }

    // pub fn wallet(&self) -> &Wallet {
    //     todo!()
    // }

    // pub fn system_config(&self) -> &SystemConfig {
    //     todo!()
    // }

}
