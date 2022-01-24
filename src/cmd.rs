mod admin;
mod controller;
// // mod executor;
// // #[cfg(feature = "evm")]
mod evm;
// mod wallet;
mod account;

use clap::{App, Arg, ArgMatches};
use clap::AppFlags;
use clap::AppSettings;
use std::collections::HashMap;
use crate::sdk::context::Context;
use crate::crypto::Crypto;

use anyhow::{
    bail, ensure, Context as _, Result
};

use crate::sdk::{
    admin::AdminBehaviour,
    account::AccountBehaviour,
    controller::ControllerBehaviour,
    executor::ExecutorBehaviour,
    evm::EvmBehaviour,
    evm::EvmBehaviourExt,
    wallet::WalletBehaviour,
};

/// Command handler that associated with a command.
pub type CommandHandler<Co, Ex, Ev, Wa> = fn(&mut Context<Co, Ex, Ev, Wa>, &mut ArgMatches) -> Result<()>;


/// Command
#[derive(Clone)]
pub struct Command<'help, Co, Ex, Ev, Wa>
{
    app: App<'help>,
    handler: Option<CommandHandler<Co, Ex, Ev, Wa>>,

    subcmds: HashMap<String, Self>,
}


impl<'help, Co, Ex, Ev, Wa> Command<'help, Co, Ex, Ev, Wa>
{
    /// Create a new command.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            app: App::new(name),
            handler: None,
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

    pub fn setting<F: Into<AppFlags>>(mut self, setting: F) -> Self
    {
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
        I: IntoIterator<Item = Self>
    {
        // just a fancy loop!
        subcmds.into_iter().fold(self, |this, subcmd| this.subcommand(subcmd))
    }

    /// Execute this command with context and args.
    pub fn exec(&self, context: &mut Context<Co, Ex, Ev, Wa>, mut m: ArgMatches) -> Result<()> {
        if let Some(handler) = self.handler {
            (handler)(context, &mut m).with_context(|| format!("failed to exec command `{}`", self.get_name()))?;
        }
        if let Some((subcmd_name, subcmd_matches)) = m.subcommand() {
            if let Some(handler) = self.subcmds.get(subcmd_name) {
                handler.exec(context, subcmd_matches.clone()).with_context(|| format!("failed to exec subcommand `{}`", subcmd_name))?;
            } else {
                bail!("no subcommand handler for `{}`", subcmd_name);
            }
        }
        Ok(())
    }

    /// Get name of the underlaying clap App.
    pub fn get_name(&self) -> &str {
        self.app.get_name()
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


pub fn all_cmd<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: ControllerBehaviour<C> + ExecutorBehaviour<C> + AdminBehaviour<C> + EvmBehaviour<C> + EvmBehaviourExt<C> + WalletBehaviour<C>,
{
    Command::new("cldi")
        .about("The command line interface to interact with `CITA-Cloud v6.3.0`.")
        .setting(AppSettings::SubcommandRequired)
        .subcommands([
            account::account_cmd(),
            admin::admin_cmd(),
            controller::controller_cmd(),
            evm::evm_cmd(),
        ])
}

