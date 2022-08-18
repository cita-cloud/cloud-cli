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

mod account;
mod admin;
mod bench;
mod cldi;
mod context;
mod ethabi;
mod evm;
mod rpc;
mod watch;

use anyhow::{anyhow, bail, Result};
use clap::{AppSettings, Arg, ArgMatches, ColorChoice};
use std::collections::HashMap;
use std::ffi::OsString;

pub use cldi::cldi_cmd;

type HandleFn<'help, Ctx> =
    dyn Fn(&Command<'help, Ctx>, &ArgMatches, &mut Ctx) -> Result<()> + 'help;
pub struct Command<'help, Ctx: 'help> {
    cmd: clap::Command<'help>,

    handler: Box<HandleFn<'help, Ctx>>,

    subcmds: HashMap<String, Self>,
}

impl<'help, Ctx: 'help> Command<'help, Ctx> {
    /// Create a new command.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            cmd: clap::Command::new(name),
            handler: Box::new(Self::dispatch_subcmd),
            subcmds: HashMap::new(),
        }
    }

    /// (Re)Sets this command's app name.
    pub fn name(mut self, name: &str) -> Self {
        self.cmd = self.cmd.name(name);
        self
    }

    pub fn alias<S: Into<&'help str>>(mut self, name: S) -> Self {
        self.cmd = self.cmd.alias(name);
        self
    }

    pub fn aliases(mut self, names: &[&'help str]) -> Self {
        self.cmd = self.cmd.aliases(names);
        self
    }

    pub fn about<O: Into<Option<&'help str>>>(mut self, about: O) -> Self {
        self.cmd = self.cmd.about(about);
        self
    }

    pub fn version<S: Into<&'help str>>(mut self, ver: S) -> Self {
        self.cmd = self.cmd.version(ver);
        self
    }

    pub fn author<S: Into<&'help str>>(mut self, author: S) -> Self {
        self.cmd = self.cmd.author(author);
        self
    }

    pub fn color(mut self, color: ColorChoice) -> Self {
        self.cmd = self.cmd.color(color);
        self
    }

    pub fn global_setting(mut self, setting: AppSettings) -> Self {
        self.cmd = self.cmd.global_setting(setting);
        self
    }

    #[allow(dead_code)]
    pub fn display_order(mut self, ord: usize) -> Self {
        self.cmd = self.cmd.display_order(ord);
        self
    }

    // https://docs.rs/clap/3.1.2/clap/enum.AppSettings.html#variant.SubcommandRequiredElseHelp
    pub fn subcommand_required_else_help(mut self, yes: bool) -> Self {
        self.cmd = self
            .cmd
            .subcommand_required(yes)
            .arg_required_else_help(yes);
        self
    }

    pub fn arg<A: Into<Arg<'help>>>(mut self, a: A) -> Self {
        self.cmd = self.cmd.arg(a);
        self
    }

    pub fn handler<H>(mut self, handler: H) -> Self
    where
        H: Fn(&Self, &ArgMatches, &mut Ctx) -> Result<()> + 'help,
    {
        self.handler = Box::new(handler);
        self
    }

    /// Add subcommand for this Command.
    pub fn subcommand(mut self, subcmd: Self) -> Self {
        let subcmd_name = subcmd.get_name().to_owned();

        self.cmd = self.cmd.subcommand(subcmd.cmd.clone());
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

    pub fn with_completions_subcmd(self) -> Self {
        let completions_without_handler = Self::new("completions")
            .about("Generate completions for current shell. Add the output script to `.profile` or `.bashrc` etc. to make it effective.")
            .arg(
                Arg::new("shell")
                    .required(true)
                    .possible_values(&[
                        "bash",
                        "zsh",
                        "powershell",
                        "fish",
                        "elvish",
                    ])
                    .validator(|s| s.parse::<clap_complete::Shell>()),
            );

        let cmd_for_completions = self
            .cmd
            .clone()
            .subcommand(completions_without_handler.cmd.clone());
        let completions = completions_without_handler.handler(move |_cmd, m, _ctx| {
            let shell: clap_complete::Shell = m.value_of("shell").unwrap().parse().unwrap();
            let mut stdout = std::io::stdout();
            let bin_name = cmd_for_completions.get_name();
            clap_complete::generate(
                shell,
                &mut cmd_for_completions.clone(),
                bin_name,
                &mut stdout,
            );
            Ok(())
        });

        self.subcommand(completions)
    }

    #[allow(dead_code)]
    pub fn exec(&self, ctx: &mut Ctx) -> Result<()> {
        let m = self.cmd.clone().get_matches();
        self.exec_with(&m, ctx)
    }

    /// Execute this command with context and args.
    pub fn exec_with(&self, m: &ArgMatches, ctx: &mut Ctx) -> Result<()> {
        (self.handler)(self, m, ctx)
    }

    pub fn exec_from<I, T>(&self, iter: I, ctx: &mut Ctx) -> Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let m = self.cmd.clone().try_get_matches_from(iter)?;
        self.exec_with(&m, ctx)
    }

    pub fn dispatch_subcmd(&self, m: &ArgMatches, ctx: &mut Ctx) -> Result<()> {
        if let Some((subcmd_name, subcmd_matches)) = m.subcommand() {
            if let Some(subcmd) = self.subcmds.get(subcmd_name) {
                subcmd.exec_with(subcmd_matches, ctx)?;
            } else {
                // TODO: this may be an unreachable branch.
                bail!("no subcommand handler for `{}`", subcmd_name);
            }
        }
        Ok(())
    }

    /// Get name of the underlaying clap App.
    pub fn get_name(&self) -> &str {
        self.cmd.get_name()
    }

    #[allow(dead_code)]
    pub fn get_clap_command(&self) -> &clap::Command<'help> {
        &self.cmd
    }

    #[allow(dead_code)]
    pub fn get_subcommand(&self, subcmd: &str) -> Option<&Self> {
        self.subcmds.get(subcmd)
    }

    #[allow(dead_code)]
    pub fn rename_subcommand(&mut self, old: &str, new: &str) -> Result<()> {
        let old_app = self
            .cmd
            .find_subcommand_mut(old)
            .ok_or_else(|| anyhow!("subcommand not found"))?;
        *old_app = old_app.clone().name(new);
        let old_subcmd = self.subcmds.remove(old).expect("subcommand not found");
        self.subcmds.insert(new.into(), old_subcmd.name(new));

        Ok(())
    }

    /// Get matches from the underlaying clap App.
    pub fn get_matches(&self) -> ArgMatches {
        self.cmd.clone().get_matches()
    }

    // TODO: get matches from

    #[allow(dead_code)]
    pub fn get_all_aliases(&self) -> impl Iterator<Item = &str> + '_ {
        self.cmd.get_all_aliases()
    }
}
