// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::{anyhow, ensure, Context as _, Result};
use rustyline::DefaultEditor;
use std::{future::Future, time::Duration};

use super::{
    client::GrpcClientBehaviour,
    wallet::{MultiCryptoAccount, Wallet},
};
use crate::config::{Config, ContextSetting};

pub struct Context<Co, Ex, Ev> {
    /// Those gRPC client are connected lazily.
    pub controller: Co,
    pub executor: Ex,
    pub evm: Ev,

    pub wallet: Wallet,

    pub config: Config,
    pub current_setting: ContextSetting,

    // rustyline::Editor, used for interactive cmd.
    pub editor: DefaultEditor,
    pub rt: CtrlCSignalCapturedRuntime,
}

impl<Co, Ex, Ev> Context<Co, Ex, Ev> {
    pub fn from_config(config: Config) -> Result<Self>
    where
        Co: GrpcClientBehaviour,
        Ex: GrpcClientBehaviour,
        Ev: GrpcClientBehaviour,
    {
        let rt = CtrlCSignalCapturedRuntime(tokio::runtime::Runtime::new()?);
        let editor = rustyline::DefaultEditor::new()?;
        let wallet = Wallet::open(&config.data_dir)?;

        let default_context_setting = config
            .context_settings
            .get(&config.default_context)
            .cloned()
            .unwrap_or_else(|| {
                println!(
                    "The configured default context setting `{}` is missing.",
                    config.default_context
                );
                println!("Using a local default context..");
                ContextSetting::default()
            });
        // connect_lazy must run in async environment.
        let (controller, executor, evm) = rt.block_on(async {
            let co = Co::connect_lazy(
                &default_context_setting.controller_addr,
                Duration::from_secs(default_context_setting.connect_timeout),
            )?;
            let ex = Ex::connect_lazy(
                &default_context_setting.executor_addr,
                Duration::from_secs(default_context_setting.connect_timeout),
            )?;
            let ev = Ev::connect_lazy(
                &default_context_setting.executor_addr,
                Duration::from_secs(default_context_setting.connect_timeout),
            )?;
            anyhow::Ok((co, ex, ev))
        })??;

        Ok(Self {
            controller,
            executor,
            evm,
            wallet,
            config,
            current_setting: default_context_setting,
            editor,
            rt,
        })
    }

    pub fn current_account(&self) -> Result<&MultiCryptoAccount> {
        let current_name = &self.current_setting.account_name;
        let current = self
            .wallet
            .get(current_name)
            // Specify that it's the **current** account not found
            .map_err(|_| anyhow!("current account `{}` not found", current_name))?;

        ensure!(
            current.crypto_type() == self.current_setting.crypto_type,
            "current account `{}`'s crypto type `{}` mismatched with target chain's crypto type `{}`",
            current_name,
            current.crypto_type(),
            self.current_setting.crypto_type,
        );

        current
            .unlocked()
            .with_context(|| format!("cannot get current account `{current_name}` "))
    }

    pub fn current_controller_addr(&self) -> &str {
        &self.current_setting.controller_addr
    }

    pub fn current_executor_addr(&self) -> &str {
        &self.current_setting.executor_addr
    }

    pub fn get_context_setting(&self, setting_name: &str) -> Result<&ContextSetting> {
        self.config
            .context_settings
            .get(setting_name)
            .ok_or_else(|| anyhow!("context`{}` not found", setting_name))
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
            let co = Co::connect_lazy(
                &setting.controller_addr,
                Duration::from_secs(setting.connect_timeout),
            )?;
            let ex = Ex::connect_lazy(
                &setting.executor_addr,
                Duration::from_secs(setting.connect_timeout),
            )?;
            let ev = Ev::connect_lazy(
                &setting.executor_addr,
                Duration::from_secs(setting.connect_timeout),
            )?;
            anyhow::Ok((co, ex, ev))
        })??;
        self.controller = controller;
        self.executor = executor;
        self.evm = evm;
        self.current_setting = setting;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Canceled")]
pub struct Canceled;

pub struct CtrlCSignalCapturedRuntime(tokio::runtime::Runtime);

impl CtrlCSignalCapturedRuntime {
    pub fn block_on<F: Future>(&self, future: F) -> Result<F::Output, Canceled> {
        self.0.block_on(async {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => Err(Canceled),
                res = future => Ok(res),
            }
        })
    }
}
