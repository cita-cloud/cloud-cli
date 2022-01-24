use crate::crypto::{ArrayLike, Crypto};

use tonic::transport::channel::Channel;

use crate::proto::{
    blockchain::{
        raw_transaction::Tx, CompactBlock, RawTransaction, Transaction as CloudNormalTransaction,
        UnverifiedTransaction, UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction,
        Witness,
    },
    common::{Address, Empty, Hash, NodeInfo, NodeNetInfo},
    controller::{BlockNumber, Flag, SystemConfig, TransactionIndex},
    evm::{Balance, ByteAbi, ByteCode, Nonce, Receipt},
    executor::{CallRequest, CallResponse},
};

use super::evm::EvmClient;
use super::executor::ExecutorClient;
use super::{
    account::AccountBehaviour,
    controller::{
        ControllerBehaviour, ControllerClient, NormalTransactionSenderBehaviour,
        RawTransactionSenderBehaviour, SignerBehaviour, UtxoTransactionSenderBehaviour, UtxoType,
    },
    evm::EvmBehaviour,
    evm::EvmBehaviourExt,
    executor::ExecutorBehaviour,
    wallet::WalletBehaviour,
};
// use super::controller::ControllerClient;
use anyhow::Context as _;
use anyhow::Result;

use super::wallet::{
    Wallet, MaybeLockedAccount,
};

pub struct Context<Co, Ex, Ev, Wa>
{
    pub current_block_number: u64,
    pub system_config: SystemConfig,

    pub wallet: Wa,

    /// Those gRPC client are connected lazily.
    pub controller: Co,
    pub executor: Ex,
    pub evm: Ev,

    pub rt: tokio::runtime::Runtime,
}

// I miss [Delegation](https://github.com/contactomorph/rfcs/blob/delegation/text/0000-delegation-of-implementation.md)
// Most of the code below is boilerplate, and ambassador doesn't work for generic trait:(
// TODO: write a macro for this

// re-export functionality for Context

#[tonic::async_trait]
impl<C, Co, Ex, Ev, Wa> ControllerBehaviour<C> for Context<Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C> + Send + Sync,
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
impl<C, Co, Ex, Ev, Wa> ExecutorBehaviour<C> for Context<Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C> + Send + Sync,
{
    async fn call(
        &self,
        from: C::Address,
        to: C::Address,
        payload: Vec<u8>,
    ) -> Result<CallResponse> {
        <Ex as ExecutorBehaviour<C>>::call(&self.executor, from, to, payload).await
    }
}

#[tonic::async_trait]
impl<C, Co, Ex, Ev, Wa> EvmBehaviour<C> for Context<Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C> + Send + Sync,
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

#[tonic::async_trait]
impl<C, Co, Ex, Ev, Wa> WalletBehaviour<C> for Context<Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C> + Send + Sync,
{
    type Locked = Wa::Locked;
    type Unlocked = Wa::Unlocked;

    async fn generate_account(&mut self, id: &str, pw: Option<&str>) -> Result<()> {
        <Wa as WalletBehaviour<C>>::generate_account(&mut self.wallet, id, pw).await
    }

    async fn import_account(&mut self, id: &str, maybe_locked: MaybeLockedAccount<Self::Locked, Self::Unlocked>) -> Result<()> {
        <Wa as WalletBehaviour<C>>::import_account(&mut self.wallet, id, maybe_locked).await
    }

    async fn unlock_account(&mut self, id: &str, pw: &str) -> Result<()> {
        <Wa as WalletBehaviour<C>>::unlock_account(&mut self.wallet, id, pw).await
    }

    async fn delete_account(&mut self, id: &str) -> Result<()> {
        <Wa as WalletBehaviour<C>>::delete_account(&mut self.wallet, id).await
    }

    async fn get_account(&self, id: &str) -> Result<&Self::Unlocked> {
        <Wa as WalletBehaviour<C>>::get_account(&self.wallet, id).await
    }

    async fn list_account(&self) -> Vec<(&str, &MaybeLockedAccount<Self::Locked, Self::Unlocked>)> {
        <Wa as WalletBehaviour<C>>::list_account(&self.wallet).await
    }

    async fn current_account(&self) -> Result<(&str, &Self::Unlocked)> {
        <Wa as WalletBehaviour<C>>::current_account(&self.wallet).await
    }

    async fn set_current_account(&mut self, id: &str) -> Result<()> {
        <Wa as WalletBehaviour<C>>::set_current_account(&mut self.wallet, id).await
    }

    async fn default_account(&self) -> Result<&MaybeLockedAccount<Self::Locked, Self::Unlocked>> {
        <Wa as WalletBehaviour<C>>::default_account(&self.wallet).await
    }

    async fn set_default_account(&mut self, id: &str) -> Result<()> {
        <Wa as WalletBehaviour<C>>::set_default_account(&mut self.wallet, id).await
    }
}

#[tonic::async_trait]
impl<C, Co, Ex, Ev, Wa> RawTransactionSenderBehaviour<C> for Context<Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C> + Send + Sync,
{
    async fn send_raw_tx(&self, raw_tx: CloudNormalTransaction) -> Result<C::Hash> {
        let account = self.current_account().await?.1;
        let raw = account.sign_raw_tx(raw_tx)?;
        self.send_raw(raw).await.context("failed to send raw")
    }

    async fn send_raw_utxo(&self, raw_utxo: CloudUtxoTransaction) -> Result<C::Hash> {
        let account = self.current_account().await?.1;
        let raw = account.sign_raw_utxo(raw_utxo)?;
        self.send_raw(raw).await.context("failed to send raw")
    }
}

#[tonic::async_trait]
impl<C, Co, Ex, Ev, Wa> NormalTransactionSenderBehaviour<C> for Context<Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C> + Send + Sync,
{
    // Use send_raw_tx if you want more control over the tx content
    async fn send_tx(&self, to: C::Address, data: Vec<u8>, value: Vec<u8>) -> Result<C::Hash> {
        let raw_tx = CloudNormalTransaction {
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
impl<C, Co, Ex, Ev, Wa> UtxoTransactionSenderBehaviour<C> for Context<Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C> + Send + Sync,
    Ex: ExecutorBehaviour<C> + Send + Sync,
    Ev: EvmBehaviour<C> + Send + Sync,
    Wa: WalletBehaviour<C> + Send + Sync,
{
    // Use send_raw_utxo if you want more control over the utxo content
    async fn send_utxo(&self, output: Vec<u8>, utxo_type: UtxoType) -> Result<C::Hash> {
        let raw_utxo = {
            let lock_id = utxo_type as u64;
            let system_config = &self.system_config;
            let pre_tx_hash = match utxo_type {
                UtxoType::Admin => &system_config.admin_pre_hash,
                UtxoType::BlockInterval => &system_config.block_interval_pre_hash,
                UtxoType::Validators => &system_config.validators_pre_hash,
                UtxoType::EmergencyBrake => &system_config.emergency_brake_pre_hash,
            }
            .clone();

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
