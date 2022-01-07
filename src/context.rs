use clap::App;
use clap::ArgMatches;

use crate::client::Client;
use crate::wallet::Wallet;
use crate::wallet::Account;

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


pub struct Context {
    pub account: Account,
    pub wallet: Wallet,

    pub system_config: SystemConfig,

    pub controller: ControllerClient<Channel>,
    pub executor: ExecutorClient<Channel>,
    #[cfg(feature = "evm")]
    pub evm: EvmClient<Channel>,

    pub rt: tokio::runtime::Runtime,
}

impl Context {
    fn new() -> Self {
        todo!()
    }
}
