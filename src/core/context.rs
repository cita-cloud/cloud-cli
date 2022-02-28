use std::marker::PhantomData;

use crate::config::{Config, ContextSetting};
use crate::crypto::{ArrayLike, Crypto};

use tonic::transport::channel::Channel;
use tonic::transport::channel::Endpoint;

use super::client::GrpcClientBehaviour;
use super::evm::EvmClient;
use super::executor::ExecutorClient;
use std::future::Future;
// use super::controller::ControllerClient;
use anyhow::anyhow;
use anyhow::Context as _;
use anyhow::Result;

use super::wallet::MultiCryptoAccount;
use super::wallet::Wallet;

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

        let default_context_setting = config
            .context_settings
            .get(&config.default_context)
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
        let current = self
            .wallet
            .get(id)
            .ok_or_else(|| anyhow!("current account `{}` not found", id))?;
        current
            .unlocked()
            .with_context(|| format!("cannot get current account `{}` ", id))
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
        let setting = self
            .config
            .context_settings
            .get(context_name)
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
