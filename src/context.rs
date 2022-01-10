use clap::App;
use clap::ArgMatches;

use crate::client::Client;
use crate::wallet::Wallet;
use crate::wallet::Account;
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
        rpc_service_client::RpcServiceClient as ControllerClient, BlockNumber, Flag, SystemConfig,
        TransactionIndex,
    },
    evm::{
        rpc_service_client::RpcServiceClient as EvmClient, Balance, ByteAbi, ByteCode, Nonce,
        Receipt,
    },
    executor::{executor_service_client::ExecutorServiceClient as ExecutorClient, CallRequest},
};


#[derive(Clone)]
pub struct Context<C: Crypto> {
    pub account: Account<C>,
    pub wallet: Wallet<C>,

    pub system_config: SystemConfig,

    /// Those gRPC client are connected lazily.
    pub controller: ControllerClient<Channel>,
    pub executor: ExecutorClient<Channel>,
    #[cfg(feature = "evm")]
    pub evm: EvmClient<Channel>,

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
