mod admin;
// mod controller;
// mod executor;
// #[cfg(feature = "evm")]
mod evm;
// mod wallet;

use clap::{App, ArgMatches};
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
    wallet::WalletBehaviour,
};

/// Command handler that associated with a command.
pub type CommandHandler<Ac, Co, Ex, Ev, Wa> = fn(&mut Context<Ac, Co, Ex, Ev, Wa>, &mut ArgMatches) -> Result<()>;


/// Command
pub struct Command<Ac, Co, Ex, Ev, Wa>
{
    app: App<'static>,
    handler: Option<CommandHandler<Ac, Co, Ex, Ev, Wa>>,

    subcmds: HashMap<String, Self>,
}


impl<Ac, Co, Ex, Ev, Wa> Command<Ac, Co, Ex, Ev, Wa>
// where
//     C: Crypto,
//     Ac: AccountBehaviour<SigningAlgorithm = C> + Send + Sync,
//     Co: ControllerBehaviour<C> + Send + Sync,
//     Ex: ExecutorBehaviour<C> + Send + Sync,
//     Ev: EvmBehaviour<C> + Send + Sync,
//     Wa: WalletBehaviour<C, Account = Ac> + Send + Sync,
{
    /// Accept an clap App without subcommands.
    /// Subcommands should be passed by using [`Command::subcommand`] or [`Command::subcommands`].
    /// 
    /// # Panics
    /// 
    /// Panic if the clap app has subcommands.
    /// 
    /// [`Command::subcommand`]: Command::subcommand
    /// [`Command::subcommands`]: Command::subcommands
    pub fn new(app_without_subcmds: App<'static>) -> Self {
        assert!(!app_without_subcmds.has_subcommands(), "subcommands should be passed by using Command::subcommands");
        Self {
            app: app_without_subcmds,
            handler: None,
            subcmds: HashMap::new(),
        }
    }

    /// Get name of the underlaying clap App.
    pub fn get_name(&self) -> &str {
        self.app.get_name()
    }

    /// Get matches from the underlaying clap App.
    pub fn get_matches(&self) -> ArgMatches {
        self.app.get_matches()
    }
    
    // TODO: get matches from


    /// Command handler is for handling leaf command(that has no subcommands) or modifying context for subcommands.
    /// It should not handle any subcommands. Subcommand has its own handler, which will be called after.
    /// 
    /// Default to no-op.
    pub fn handler(mut self, handler: CommandHandler<Ac, Co, Ex, Ev, Wa>) -> Self {
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
    pub fn exec(&self, context: &mut Context<Ac, Co, Ex, Ev, Wa>, mut m: ArgMatches) -> Result<()> {
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
}

