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

use crate::crypto::{ ArrayLike, Crypto };

use serde::{Deserialize, Serialize};

use super::account::AccountBehaviour;

pub trait WalletBehaviour<C: Crypto> {
    type Account: AccountBehaviour<SigningAlgorithm = C>;

    fn generate_account(&self, id: &str) -> Self::Account;
    fn import_account(&self, id: &str, sk: C::SecretKey);
    fn export_account(&self, id: &str) -> Option<&Self::Account>;
    fn delete_account(&self, id: &str) -> Option<Self::Account>;

    // TODO: better API
    fn list_account(&self) -> Vec<String>;
}
