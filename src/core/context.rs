use std::marker::PhantomData;

use crate::config::{Config, ContextSetting};
use crate::crypto::{ArrayLike, Crypto};

use tonic::transport::channel::Channel;
use tonic::transport::channel::Endpoint;

use std::future::Future;
use super::client::GrpcClientBehaviour;
use super::evm::EvmClient;
use super::executor::ExecutorClient;
// use super::controller::ControllerClient;
use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;

use super::wallet::Wallet;
use super::wallet::MultiCryptoAccount;

#[cfg(test)]
use super::controller::MockControllerClient;
#[cfg(test)]
use super::executor::MockExecutorClient;
#[cfg(test)]
use super::evm::MockEvmClient;

#[cfg(test)]
pub fn mock_context() -> Context<MockControllerClient, MockExecutorClient, MockEvmClient> {
    todo!()

}

pub struct Context<Co, Ex, Ev> {
    /// Those gRPC client are connected lazily.
    pub controller: Co,
    pub executor: Ex,
    pub evm: Ev,

    pub wallet: Wallet,

    pub config: Config,
    pub current_setting: ContextSetting,

    pub rt: CancelableRuntime,
}

impl<Co, Ex, Ev> Context<Co, Ex, Ev> {
    pub fn from_config(config: Config) -> Result<Self>
    where
        Co: GrpcClientBehaviour,
        Ex: GrpcClientBehaviour,
        Ev: GrpcClientBehaviour,
    {
        let rt = CancelableRuntime(tokio::runtime::Runtime::new()?);
        let wallet = Wallet::open(&config.data_dir)?;
        let default_context_setting = config.context_settings.get(&config.default_context)
            // TODO: log warning and use default context setting
            .ok_or_else(|| anyhow!("missing default context setting"))?
            .clone();
        // connect_lazy must run in async environment.
        let (controller, executor, evm) = rt.block_on(async {
            let co = Co::connect_lazy(&default_context_setting.controller_addr)?;
            let ex = Ex::connect_lazy(&default_context_setting.executor_addr)?;
            let ev = Ev::connect_lazy(&default_context_setting.executor_addr)?;
            anyhow::Ok((co, ex, ev))
        })??;

        Ok(Self {
            controller,
            executor,
            evm,
            wallet,
            config,
            current_setting: default_context_setting,
            rt,
        })
    }

    pub fn current_account(&self) -> Result<&MultiCryptoAccount> {
        let id = &self.current_setting.account_id;
        let current = self.wallet.get(id).ok_or_else(|| anyhow!("current account `{}` not found", id))?;
        current.unlocked().with_context(|| format!("cannot get current account `{}` ", id))
    }

    pub fn current_controller_addr(&self) -> &str {
        &self.current_setting.controller_addr
    }

    pub fn current_executor_addr(&self) -> &str {
        &self.current_setting.executor_addr
    }

    pub fn switch_context(&mut self, setting: ContextSetting) -> Result<()>
    where
        Co: GrpcClientBehaviour,
        Ex: GrpcClientBehaviour,
        Ev: GrpcClientBehaviour,
    {
        if self.current_setting == setting {
            return Ok(());
        }

        let (controller, executor, evm) = self.rt.block_on(async {
            let co = Co::connect_lazy(&setting.controller_addr)?;
            let ex = Ex::connect_lazy(&setting.executor_addr)?;
            let ev = Ev::connect_lazy(&setting.executor_addr)?;
            anyhow::Ok((co, ex, ev))
        })??;
        self.controller = controller;
        self.executor = executor;
        self.evm = evm;
        self.current_setting = setting;

        Ok(())
    }

    pub fn switch_context_to(&mut self, context_name: &str) -> Result<()>
    where
        Co: GrpcClientBehaviour,
        Ex: GrpcClientBehaviour,
        Ev: GrpcClientBehaviour,
    {
        let setting = self.config.context_settings.get(context_name)
            .ok_or_else(|| anyhow!("context`{}` not found", context_name))?
            .clone();
        self.switch_context(setting)
    }
} 


#[derive(Debug, thiserror::Error)]
#[error("Canceled")]
pub struct Canceled;

pub struct CancelableRuntime(tokio::runtime::Runtime);

impl CancelableRuntime {
    pub fn block_on<F: Future>(&self, future: F) -> Result<F::Output, Canceled> {
        self.0.block_on(async {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => Err(Canceled),
                res = future => Ok(res),
            }
        })
    }
}

// // I miss [Delegation](https://github.com/contactomorph/rfcs/blob/delegation/text/0000-delegation-of-implementation.md)
// // Most of the code below is boilerplate, and ambassador doesn't work for generic trait:(
// // TODO: write a macro for this

// // re-export functionality for Context

// #[tonic::async_trait]
// impl<C, Co, Ex, Ev, Wa> ControllerBehaviour<C> for Context<Co, Ex, Ev, Wa>
// where
//     C: Crypto,
//     Co: ControllerBehaviour<C> + Send + Sync,
//     Context<Co, Ex, Ev, Wa>: Send + Sync,
// {
//     async fn send_raw(&self, raw: RawTransaction) -> Result<Hash> {
//         <Co as ControllerBehaviour<C>>::send_raw(&self.controller, raw).await
//     }

//     async fn get_system_config(&self) -> Result<SystemConfig> {
//         <Co as ControllerBehaviour<C>>::get_system_config(&self.controller).await
//     }

//     async fn get_block_number(&self, for_pending: bool) -> Result<u64> {
//         <Co as ControllerBehaviour<C>>::get_block_number(&self.controller, for_pending).await
//     }
//     async fn get_block_hash(&self, block_number: u64) -> Result<Hash> {
//         <Co as ControllerBehaviour<C>>::get_block_hash(&self.controller, block_number).await
//     }

//     async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock> {
//         <Co as ControllerBehaviour<C>>::get_block_by_number(&self.controller, block_number).await
//     }

//     async fn get_block_by_hash(&self, hash: Hash) -> Result<CompactBlock> {
//         <Co as ControllerBehaviour<C>>::get_block_by_hash(&self.controller, hash).await
//     }

//     async fn get_tx(&self, tx_hash: Hash) -> Result<RawTransaction> {
//         <Co as ControllerBehaviour<C>>::get_tx(&self.controller, tx_hash).await
//     }

//     async fn get_tx_index(&self, tx_hash: Hash) -> Result<u64> {
//         <Co as ControllerBehaviour<C>>::get_tx_index(&self.controller, tx_hash).await
//     }

//     async fn get_tx_block_number(&self, tx_hash: Hash) -> Result<u64> {
//         <Co as ControllerBehaviour<C>>::get_tx_block_number(&self.controller, tx_hash).await
//     }

//     async fn get_peer_count(&self) -> Result<u64> {
//         <Co as ControllerBehaviour<C>>::get_peer_count(&self.controller).await
//     }

//     async fn get_peers_info(&self) -> Result<TotalNodeInfo> {
//         <Co as ControllerBehaviour<C>>::get_peers_info(&self.controller).await
//     }

//     async fn add_node(&self, multiaddr: String) -> Result<u32> {
//         <Co as ControllerBehaviour<C>>::add_node(&self.controller, multiaddr).await
//     }
// }

// #[tonic::async_trait]
// impl<C, Co, Ex, Ev, Wa> ExecutorBehaviour<C> for Context<Co, Ex, Ev, Wa>
// where
//     C: Crypto,
//     Ex: ExecutorBehaviour<C> + Send + Sync,
//     Context<Co, Ex, Ev, Wa>: Send + Sync,
// {
//     async fn call(
//         &self,
//         from: Address,
//         to: Address,
//         payload: Vec<u8>,
//     ) -> Result<CallResponse> {
//         <Ex as ExecutorBehaviour<C>>::call(&self.executor, from, to, payload).await
//     }
// }

// #[tonic::async_trait]
// impl<C, Co, Ex, Ev, Wa> EvmBehaviour<C> for Context<Co, Ex, Ev, Wa>
// where
//     C: Crypto,
//     Ev: EvmBehaviour<C> + Send + Sync,
//     Context<Co, Ex, Ev, Wa>: Send + Sync,
// {
//     async fn get_receipt(&self, hash: Hash) -> Result<Receipt> {
//         <Ev as EvmBehaviour<C>>::get_receipt(&self.evm, hash).await
//     }

//     async fn get_code(&self, addr: Address) -> Result<ByteCode> {
//         <Ev as EvmBehaviour<C>>::get_code(&self.evm, addr).await
//     }

//     async fn get_balance(&self, addr: Address) -> Result<Balance> {
//         <Ev as EvmBehaviour<C>>::get_balance(&self.evm, addr).await
//     }

//     async fn get_tx_count(&self, addr: Address) -> Result<Nonce> {
//         <Ev as EvmBehaviour<C>>::get_tx_count(&self.evm, addr).await
//     }

//     async fn get_abi(&self, addr: Address) -> Result<ByteAbi> {
//         <Ev as EvmBehaviour<C>>::get_abi(&self.evm, addr).await
//     }
// }

// #[tonic::async_trait]
// impl<C, Co, Ex, Ev, Wa> WalletBehaviour<C> for Context<Co, Ex, Ev, Wa>
// where
//     C: Crypto,
//     Wa: WalletBehaviour<C> + Send + Sync,
//     Context<Co, Ex, Ev, Wa>: Send + Sync,
// {
//     type Locked = Wa::Locked;
//     type Unlocked = Wa::Unlocked;

//     async fn generate_account(&mut self, id: &str, pw: Option<&str>) -> Result<()> {
//         <Wa as WalletBehaviour<C>>::generate_account(&mut self.wallet, id, pw).await
//     }

//     async fn import_account(
//         &mut self,
//         id: &str,
//         maybe_locked: MaybeLockedAccount<Self::Locked, Self::Unlocked>,
//     ) -> Result<()> {
//         <Wa as WalletBehaviour<C>>::import_account(&mut self.wallet, id, maybe_locked).await
//     }

//     async fn unlock_account(&mut self, id: &str, pw: &str) -> Result<()> {
//         <Wa as WalletBehaviour<C>>::unlock_account(&mut self.wallet, id, pw).await
//     }

//     async fn delete_account(&mut self, id: &str) -> Result<()> {
//         <Wa as WalletBehaviour<C>>::delete_account(&mut self.wallet, id).await
//     }

//     async fn get_account(&self, id: &str) -> Result<&Self::Unlocked> {
//         <Wa as WalletBehaviour<C>>::get_account(&self.wallet, id).await
//     }

//     async fn list_account(&self) -> Vec<(&str, &MaybeLockedAccount<Self::Locked, Self::Unlocked>)> {
//         <Wa as WalletBehaviour<C>>::list_account(&self.wallet).await
//     }

//     async fn current_account(&self) -> Result<(&str, &Self::Unlocked)> {
//         <Wa as WalletBehaviour<C>>::current_account(&self.wallet).await
//     }

//     async fn set_current_account(&mut self, id: &str) -> Result<()> {
//         <Wa as WalletBehaviour<C>>::set_current_account(&mut self.wallet, id).await
//     }
// }

// #[tonic::async_trait]
// impl<C, Co, Ex, Ev, Wa> RawTransactionSenderBehaviour<C> for Context<Co, Ex, Ev, Wa>
// where
//     C: Crypto,
//     Co: ControllerBehaviour<C> + Send + Sync,
//     Wa: WalletBehaviour<C> + Send + Sync,
//     Context<Co, Ex, Ev, Wa>: Send + Sync,
// {
//     async fn send_raw_tx(&self, raw_tx: CloudNormalTransaction) -> Result<Hash> {
//         let account = self.current_account().await?.1;
//         let raw = account.sign_raw_tx(raw_tx)?;
//         self.send_raw(raw).await.context("failed to send raw")
//     }

//     async fn send_raw_utxo(&self, raw_utxo: CloudUtxoTransaction) -> Result<Hash> {
//         let account = self.current_account().await?.1;
//         let raw = account.sign_raw_utxo(raw_utxo)?;
//         self.send_raw(raw).await.context("failed to send raw")
//     }
// }

// #[tonic::async_trait]
// impl<C, Co, Ex, Ev, Wa> NormalTransactionSenderBehaviour<C> for Context<Co, Ex, Ev, Wa>
// where
//     C: Crypto,
//     Co: ControllerBehaviour<C> + Send + Sync,
//     Wa: WalletBehaviour<C> + Send + Sync,
//     Context<Co, Ex, Ev, Wa>: Send + Sync,
// {
//     // Use send_raw_tx if you want more control over the tx content
//     async fn send_tx(&self, to: Address, data: Vec<u8>, value: Vec<u8>) -> Result<Hash> {
//         let (current_block_number, system_config) =
//             tokio::try_join!(self.get_block_number(false), self.get_system_config())
//                 .context("failed to fetch chain status")?;

//         let raw_tx = CloudNormalTransaction {
//             version: system_config.version,
//             to: to.to_vec(),
//             data,
//             value,
//             nonce: rand::random::<u64>().to_string(),
//             quota: 3_000_000,
//             valid_until_block: current_block_number + 95,
//             chain_id: system_config.chain_id.clone(),
//         };

//         <Self as RawTransactionSenderBehaviour<C>>::send_raw_tx(&self, raw_tx).await
//     }
// }

// #[tonic::async_trait]
// impl<C, Co, Ex, Ev, Wa> UtxoTransactionSenderBehaviour<C> for Context<Co, Ex, Ev, Wa>
// where
//     C: Crypto,
//     Co: ControllerBehaviour<C> + Send + Sync,
//     Wa: WalletBehaviour<C> + Send + Sync,
//     Context<Co, Ex, Ev, Wa>: Send + Sync,
// {
//     // Use send_raw_utxo if you want more control over the utxo content
//     async fn send_utxo(&self, output: Vec<u8>, utxo_type: UtxoType) -> Result<Hash> {
//         let system_config = self
//             .get_system_config()
//             .await
//             .context("failed to get system config")?;
//         let raw_utxo = {
//             let lock_id = utxo_type as u64;
//             let pre_tx_hash = match utxo_type {
//                 UtxoType::Admin => &system_config.admin_pre_hash,
//                 UtxoType::BlockInterval => &system_config.block_interval_pre_hash,
//                 UtxoType::Validators => &system_config.validators_pre_hash,
//                 UtxoType::EmergencyBrake => &system_config.emergency_brake_pre_hash,
//             }
//             .clone();

//             CloudUtxoTransaction {
//                 version: system_config.version,
//                 pre_tx_hash,
//                 output,
//                 lock_id,
//             }
//         };

//         <Self as RawTransactionSenderBehaviour<C>>::send_raw_utxo(&self, raw_utxo).await
//     }
// }

// pub fn from_config<C: Crypto>(
//     config: Config,
// ) -> Result<Context<ControllerClient, ExecutorClient, EvmClient, Wallet>> {
//     // let rt = CancelableRuntime(tokio::runtime::Runtime::new()?);

//     // let (controller, executor, evm, wallet) = rt.block_on(async {
//     //     // Although connect_lazy isn't async, they still must be in an async context.
//     //     let controller = {
//     //         let addr = format!("http://{}", config.controller_addr);
//     //         let channel = Endpoint::from_shared(addr)?.connect_lazy();
//     //         ControllerClient::new(channel)
//     //     };

//     //     let executor = {
//     //         let addr = format!("http://{}", config.executor_addr);
//     //         let channel = Endpoint::from_shared(addr)?.connect_lazy();
//     //         ExecutorClient::new(channel)
//     //     };

//     //     let evm = {
//     //         // use the same addr as executor
//     //         let addr = format!("http://{}", config.executor_addr);
//     //         let channel = Endpoint::from_shared(addr).unwrap().connect_lazy();
//     //         EvmClient::new(channel)
//     //     };

//     //     let wallet = Wallet::open(&config.data_dir).await?;

//     //     anyhow::Ok((controller, executor, evm, wallet))
//     // })??;

//     // let this = Context {
//     //     controller,
//     //     executor,
//     //     evm,
//     //     wallet,

//     //     config,

//     //     rt,
//     // };

//     // Ok(this)
//     todo!()
// }
