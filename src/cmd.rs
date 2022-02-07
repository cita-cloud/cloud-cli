mod admin;
mod rpc;
// // mod executor;
// // #[cfg(feature = "evm")]
mod evm;
// mod wallet;
mod key;

use crate::crypto::Crypto;
use crate::sdk::context::Context;
use clap::AppFlags;
use clap::AppSettings;
use clap::{App, Arg, ArgMatches};
use std::collections::HashMap;
use std::ffi::OsString;
use tonic::transport::Endpoint;

use anyhow::{anyhow, bail, ensure, Context as _, Result};

use crate::sdk::{
    account::AccountBehaviour, admin::AdminBehaviour, controller::ControllerBehaviour,
    evm::EvmBehaviour, evm::EvmBehaviourExt, executor::ExecutorBehaviour, wallet::WalletBehaviour,
    controller::ControllerClient, executor::ExecutorClient, evm::EvmClient,
    wallet::Wallet
};

use crate::interactive::interactive;

// TODO: Use Box<dyn Fn(&mut Context<Co, Ex, Ev, Wa>, &mut ArgMatches) -> Result<()>> for handler.

/// Command handler that associated with a command.
pub type CommandHandler<Co, Ex, Ev, Wa> =
    fn(&mut Context<Co, Ex, Ev, Wa>, &mut ArgMatches) -> Result<()>;

/// Command
// #[derive(Clone)]
pub struct Command<'help, Co, Ex, Ev, Wa> {
    app: App<'help>,
    handler: Option<CommandHandler<Co, Ex, Ev, Wa>>,
    enable_interactive: bool,

    subcmds: HashMap<String, Self>,
}

impl<'help, Co, Ex, Ev, Wa> Command<'help, Co, Ex, Ev, Wa> {
    /// Create a new command.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            app: App::new(name),
            handler: None,
            enable_interactive: false,
            subcmds: HashMap::new(),
        }
    }

    /// (Re)Sets this command's app name.
    pub fn name(mut self, name: &str) -> Self {
        self.app = self.app.name(name);
        self
    }

    pub fn alias<S: Into<&'help str>>(mut self, name: S) -> Self {
        self.app = self.app.alias(name);
        self
    }

    pub fn aliases(mut self, names: &[&'help str]) -> Self {
        self.app = self.app.aliases(names);
        self
    }

    pub fn about<O: Into<Option<&'help str>>>(mut self, about: O) -> Self {
        self.app = self.app.about(about);
        self
    }

    pub fn setting<F: Into<AppFlags>>(mut self, setting: F) -> Self {
        self.app = self.app.setting(setting);
        self
    }

    pub fn arg<A: Into<Arg<'help>>>(mut self, a: A) -> Self {
        self.app = self.app.arg(a);
        self
    }

    /// Command handler is for handling a leaf command(that has no subcommands) or modifying the Context/ArgMatches for subcommands.
    /// After processed by the handler, Context and subcommand's ArgMatches will be handled by the subcommand(if any).
    ///
    /// Default to no-op.
    pub fn handler(mut self, handler: CommandHandler<Co, Ex, Ev, Wa>) -> Self {
        self.handler.replace(handler);
        self
    }

    /// Add subcommand for this Command.
    pub fn subcommand(mut self, subcmd: Self) -> Self {
        let subcmd_name = subcmd.get_name().to_owned();

        self.app = self.app.subcommand(subcmd.app.clone());
        self.subcmds.insert(subcmd_name, subcmd);

        self
    }

    /// Same as [`subcommand`], but accept multiple subcommands.
    ///
    /// [`Command::subcommand`]: Command::subcommand
    pub fn subcommands<I>(self, subcmds: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        // just a fancy loop!
        subcmds
            .into_iter()
            .fold(self, |this, subcmd| this.subcommand(subcmd))
    }

    pub fn interactive(mut self, enable: bool) -> Self {
        self.enable_interactive = enable;
        self
    }

    pub fn exec(&mut self, ctx: &mut Context<Co, Ex, Ev, Wa>) -> Result<()> {
        let m = self.app.clone().get_matches();
        self.exec_with(ctx, m)
    }

    /// Execute this command with context and args.
    pub fn exec_with(
        &mut self,
        ctx: &mut Context<Co, Ex, Ev, Wa>,
        mut m: ArgMatches,
    ) -> Result<()> {
        if let Some(handler) = self.handler {
            (handler)(ctx, &mut m)
                .with_context(|| format!("failed to exec command `{}`", self.get_name()))?;
        }
        if let Some((subcmd_name, subcmd_matches)) = m.subcommand() {
            if let Some(handler) = self.subcmds.get_mut(subcmd_name) {
                handler.exec_with(ctx, subcmd_matches.clone())?;
            } else {
                bail!("no subcommand handler for `{}`", subcmd_name);
            }
        } else if self.enable_interactive {
            // avoid recursion
            self.enable_interactive = false;
            interactive::<Co, Ex, Ev, Wa>(self, ctx)?;
        }

        Ok(())
    }

    pub fn exec_from<I, T>(&mut self, ctx: &mut Context<Co, Ex, Ev, Wa>, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let m = self.app.clone().try_get_matches_from(iter)?;
        self.exec_with(ctx, m)
    }

    /// Get name of the underlaying clap App.
    pub fn get_name(&self) -> &str {
        self.app.get_name()
    }

    pub fn get_subcommand(&self, subcmd: &str) -> Option<&Self> {
        self.subcmds.get(subcmd)
    }

    pub fn rename_subcommand(&mut self, old: &str, new: &str) -> Result<()> {
        let old_app = self
            .app
            .find_subcommand_mut(old)
            .ok_or(anyhow!("subcommand no found"))?;
        *old_app = old_app.clone().name(new);
        let old_subcmd = self.subcmds.remove(old).expect("subcommand no found");
        self.subcmds.insert(new.into(), old_subcmd.name(new));

        Ok(())
    }

    /// Get matches from the underlaying clap App.
    pub fn get_matches(&self) -> ArgMatches {
        self.app.clone().get_matches()
    }

    // TODO: get matches from

    pub fn get_all_aliases(&self) -> impl Iterator<Item = &str> + '_ {
        self.app.get_all_aliases()
    }
}

pub fn all_cmd<'help, C: Crypto>() -> Command<'help, ControllerClient, ExecutorClient, EvmClient, Wallet<C>>
{
    Command::new("cldi")
        .about("The command line interface to interact with `CITA-Cloud v6.3.0`.")
        .arg(
            Arg::new("controller-addr")
                .help("controller address")
                .short('r')
                .takes_value(true)
                // TODO: add validator
        )
        .arg(
            Arg::new("executor-addr")
                .help("executor address")
                .short('e')
                .takes_value(true)
                // TODO: add validator
        )
        .interactive(true)
        .handler(|ctx, m| {
            let rt = ctx.rt.handle().clone();
            rt.block_on(async {
                if let Some(controller_addr) = m.value_of("controller-addr") {
                    let controller = {
                        let addr = format!("http://{controller_addr}");
                        let channel = Endpoint::from_shared(addr)?.connect_lazy();
                        ControllerClient::new(channel)
                    };
                    ctx.controller = controller;
                }

                if let Some(executor_addr) = m.value_of("executor-addr") {
                    let executor = {
                        let addr = format!("http://{executor_addr}");
                        let channel = Endpoint::from_shared(addr)?.connect_lazy();
                        ExecutorClient::new(channel)
                    };

                    let evm = {
                        let addr = format!("http://{executor_addr}");
                        let channel = Endpoint::from_shared(addr).unwrap().connect_lazy();
                        EvmClient::new(channel)
                    };

                    ctx.executor = executor;
                    ctx.evm = evm;
                }
                anyhow::Ok(())
            })
        })
        .subcommands([
            key::key_cmd(),
            admin::admin_cmd(),
            // TODO: figure out why I have to specify `C` for this cmd
            rpc::rpc_cmd::<C, _, _, _, _>(),
            evm::evm_cmd(),
        ])
}
