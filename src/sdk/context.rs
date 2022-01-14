use crate::crypto::{ Crypto, ArrayLike };

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
    executor::{CallRequest, CallResponse},
};

use super::{controller::{
    ControllerClient, ControllerBehaviour, SignerBehaviour, RawTransactionSenderBehaviour, NormalTransactionSenderBehaviour, UtxoTransactionSenderBehaviour, UtxoType,
}, wallet::WalletBehaviour, evm::EvmBehaviour, executor::ExecutorBehaviour, account::AccountBehaviour };
use super::executor::ExecutorClient;
use super::evm::EvmClient;
// use super::controller::ControllerClient;
use anyhow::Result;
use super::wallet::Wallet;


pub struct Context<C, Ac, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    pub current_block_number: u64,
    pub system_config: SystemConfig,

    pub wallet: Wa,

    /// Those gRPC client are connected lazily.
    pub controller: Co,
    pub executor: Ex,
    pub evm: Ev,

    pub rt: tokio::runtime::Handle,

    // TODO: re-consider this field. Did I do it right?
    _phantom: std::marker::PhantomData<Ac>,
}

// I miss [Delegation](https://github.com/contactomorph/rfcs/blob/delegation/text/0000-delegation-of-implementation.md)
// Most of the code below is boilerplate, and ambassador doesn't work for generic trait:(
// TODO: write a macro for this

// re-export functionality for Context

#[tonic::async_trait]
impl<C, Ac, Co, Ex, Ev, Wa> ControllerBehaviour<C> for Context<C, Ac, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash> {
        <Co as ControllerBehaviour<C>>::send_raw(&self.controller, raw).await
    }

    async fn get_system_config(&self) -> Result<SystemConfig> {
        <Co as ControllerBehaviour<C>>::get_system_config(&self.controller).await
    }

    async fn get_block_number(&self, for_pending: bool) -> Result<u64> {
        <Co as ControllerBehaviour<C>>::get_block_number(&self.controller, for_pending).await
    }
    async fn get_block_hash(&self, block_number: u64) -> Result<C::Hash> {
        <Co as ControllerBehaviour<C>>::get_block_hash(&self.controller, block_number).await
    }

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock> {
        <Co as ControllerBehaviour<C>>::get_block_by_number(&self.controller, block_number).await
    }

    async fn get_block_by_hash(&self, hash: C::Hash) -> Result<CompactBlock> {
        <Co as ControllerBehaviour<C>>::get_block_by_hash(&self.controller, hash).await
    }

    async fn get_tx(&self, tx_hash: C::Hash) -> Result<RawTransaction> {
        <Co as ControllerBehaviour<C>>::get_tx(&self.controller, tx_hash).await
    }

    async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<u64> {
        <Co as ControllerBehaviour<C>>::get_tx_index(&self.controller, tx_hash).await
    }

    async fn get_tx_block_number(&self, tx_hash: C::Hash) -> Result<u64> {
        <Co as ControllerBehaviour<C>>::get_tx_block_number(&self.controller, tx_hash).await
    }

    async fn get_peer_count(&self) -> Result<u64> {
        <Co as ControllerBehaviour<C>>::get_peer_count(&self.controller).await
    }

    async fn get_peers_info(&self) -> Result<Vec<NodeInfo>> {
        <Co as ControllerBehaviour<C>>::get_peers_info(&self.controller).await
    }

    async fn add_node(&self, multiaddr: String) -> Result<u32> {
        <Co as ControllerBehaviour<C>>::add_node(&self.controller, multiaddr).await
    }
}


#[tonic::async_trait]
impl<C, Ac, Co, Ex, Ev, Wa> ExecutorBehaviour<C> for Context<C, Ac, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    async fn call(&self, from: C::Address, to: C::Address, payload: Vec<u8>) -> Result<CallResponse> {
        <Ex as ExecutorBehaviour<C>>::call(&self.executor, from, to, payload).await
    }
}

#[tonic::async_trait]
impl<C, Ac, Co, Ex, Ev, Wa> EvmBehaviour<C> for Context<C, Ac, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    async fn get_receipt(&self, hash: C::Hash) -> Result<Receipt> {
        <Ev as EvmBehaviour<C>>::get_receipt(&self.evm, hash).await
    }

    async fn get_code(&self, addr: C::Address) -> Result<ByteCode> {
        <Ev as EvmBehaviour<C>>::get_code(&self.evm, addr).await
    }

    async fn get_balance(&self, addr: C::Address) -> Result<Balance> {
        <Ev as EvmBehaviour<C>>::get_balance(&self.evm, addr).await
    }

    async fn get_tx_count(&self, addr: C::Address) -> Result<Nonce> {
        <Ev as EvmBehaviour<C>>::get_tx_count(&self.evm, addr).await
    }

    async fn get_abi(&self, addr: C::Address) -> Result<ByteAbi> {
        <Ev as EvmBehaviour<C>>::get_abi(&self.evm, addr).await
    }
}


impl<C, Ac, Co, Ex, Ev, Wa> WalletBehaviour<C> for Context<C, Ac, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    type Account = Ac;

    fn generate_account(&self, id: &str) -> Self::Account {
        <Wa as WalletBehaviour<C>>::generate_account(&self.wallet, id)
    }

    fn import_account(&self, id: &str, sk: C::SecretKey) {
        <Wa as WalletBehaviour<C>>::import_account(&self.wallet, id, sk)
    }

    fn export_account(&self, id: &str) -> Option<&Self::Account> {
        <Wa as WalletBehaviour<C>>::export_account(&self.wallet, id)
    }

    fn delete_account(&self, id: &str) -> Option<Self::Account> {
        <Wa as WalletBehaviour<C>>::delete_account(&self.wallet, id)
    }

    fn current_account(&self) -> &Self::Account {
        <Wa as WalletBehaviour<C>>::current_account(&self.wallet)
    }

    // TODO: better API
    fn list_account(&self) -> Vec<(&str, &Self::Account)> {
        <Wa as WalletBehaviour<C>>::list_account(&self.wallet)
    }
}

impl<C, Ac, Co, Ex, Ev, Wa> SignerBehaviour<C> for Context<C, Ac, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    fn sign_raw_tx(&self, tx: CloudTransaction) -> RawTransaction {
        let account = <Self as WalletBehaviour<C>>::current_account(self);
        account.sign_raw_tx(tx)
    }

    fn sign_raw_utxo(&self, utxo: CloudUtxoTransaction) -> RawTransaction {
        let account = <Self as WalletBehaviour<C>>::current_account(self);
        account.sign_raw_utxo(utxo)
    }
}


#[tonic::async_trait]
impl<C, Ac, Co, Ex, Ev, Wa> NormalTransactionSenderBehaviour<C> for Context<C, Ac, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    async fn send_tx(
        &self,
        to: C::Address,
        data: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<C::Hash> {
        let raw_tx = CloudTransaction {
            version: self.system_config.version,
            to: to.to_vec(),
            data,
            value,
            nonce: rand::random::<u64>().to_string(),
            quota: 3_000_000,
            valid_until_block: self.current_block_number + 95,
            chain_id: self.system_config.chain_id.clone(),
        };

        <Self as RawTransactionSenderBehaviour<C>>::send_raw_tx(&self, raw_tx).await
    }
}

#[tonic::async_trait]
impl<C, Ac, Co, Ex, Ev, Wa> UtxoTransactionSenderBehaviour<C> for Context<C, Ac, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    async fn send_utxo(
        &self,
        output: Vec<u8>,
        utxo_type: UtxoType,
    ) -> Result<C::Hash> {
        let raw_utxo = {
            let lock_id = utxo_type as u64;
            let system_config = &self.system_config;
            let pre_tx_hash = match utxo_type {
                UtxoType::Admin => &system_config.admin_pre_hash,
                UtxoType::BlockInterval => &system_config.block_interval_pre_hash,
                UtxoType::Validators => &system_config.validators_pre_hash,
                UtxoType::EmergencyBrake => &system_config.emergency_brake_pre_hash,
            }.clone();

            CloudUtxoTransaction {
                version: system_config.version,
                pre_tx_hash,
                output,
                lock_id,
            }
        };

        <Self as RawTransactionSenderBehaviour<C>>::send_raw_utxo(&self, raw_utxo).await
    }
}
