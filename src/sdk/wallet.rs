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

use crate::crypto::{ArrayLike, Crypto};

use serde::{Deserialize, Serialize};

use super::account::Account;
use super::account::AccountBehaviour;
use anyhow::Result;
use anyhow::Context;

// TODO: use async?
pub trait WalletBehaviour<C: Crypto> {
    type Account: AccountBehaviour<SigningAlgorithm = C>;

    fn import_account(&self, id: &str, account: Self::Account, pw: Option<&str>) -> Result<()>;
    fn unlock_account(&self, id: &str, pw: Option<&str>) -> Result<Self::Account>;
    fn delete_account(&self, id: &str) -> Result<Self::Account>;

    fn list_account_id(&self) -> Vec<String>;
}


struct LockedAccount<C: Crypto> {
    address: C::Address,
    encrypted_sk: Vec<u8>,
}

impl<C: Crypto> LockedAccount<C> {
    fn unlock(&self, pw: Option<&str>) -> Result<Account<C>> {
        todo!()
    }
}

enum MaybeLockedAccount<C: Crypto> {
    Locked(LockedAccount<C>),
    Unlocked(Account<C>),
}

impl<C: Crypto> From<LockedAccount<C>> for MaybeLockedAccount<C> {
    fn from(locked: LockedAccount<C>) -> Self {
        Self::Locked(locked)
    }
}

impl<C: Crypto> From<Account<C>> for MaybeLockedAccount<C> {
    fn from(unlocked: Account<C>) -> Self {
        Self::Unlocked(unlocked)
    }
}

pub struct Wallet {


}

impl<C: Crypto> WalletBehaviour<C> for Wallet {
    type Account = Account<C>;

    fn import_account(&self, id: &str, account: Self::Account, pw: Option<&str>) -> Result<()>;
    fn unlock_account(&self, id: &str, pw: Option<&str>) -> Result<Self::Account>;
    fn delete_account(&self, id: &str) -> Result<Self::Account>;

    fn list_account_id(&self) -> Vec<String>;
}
