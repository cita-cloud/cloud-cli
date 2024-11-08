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

mod cmd;
mod config;
mod core;
mod crypto;
mod display;
mod legacy;
mod types;
mod utils;

use anyhow::Result;
use rustyline::error::ReadlineError;
use std::fs;

use crate::{
    config::{Config, CLOUD_CLI_DATA_DIR_NAME},
    core::{
        context::Context, controller::ControllerClient, evm::EvmClient, executor::ExecutorClient,
        wallet::Account, wallet::Wallet,
    },
    crypto::SmCrypto,
    utils::init_local_utc_offset,
};

#[macro_use]
extern crate serde_derive;

fn main() -> Result<()> {
    // This should be called without any other concurrent running threads.
    init_local_utc_offset();

    let data_dir = {
        let home = home::home_dir().expect("cannot find home dir");
        home.join(CLOUD_CLI_DATA_DIR_NAME)
    };
    let is_init = !data_dir.exists();
    let is_legacy = data_dir.is_file();

    if is_legacy {
        const LEGACY_FILE_BACKUP_NAME: &str = ".cloud-cli-v0.2.0-legacy";

        println!("Migrating cloud-cli v0.2.0 accounts..");
        println!(
            "Data backup can be found in `{CLOUD_CLI_DATA_DIR_NAME}/{LEGACY_FILE_BACKUP_NAME}`."
        );

        // Displace legacy file
        #[allow(clippy::redundant_clone)]
        let legacy_file = data_dir.clone();
        let tmp_legacy_file_backup = {
            let mut p = legacy_file.clone();
            p.set_file_name(LEGACY_FILE_BACKUP_NAME);
            p
        };
        fs::rename(&legacy_file, &tmp_legacy_file_backup)?;
        let legacy_file_backup = data_dir.join(LEGACY_FILE_BACKUP_NAME);

        // Move legacy file to data dir
        let mut config = Config::open(&data_dir)?;
        fs::rename(&tmp_legacy_file_backup, &legacy_file_backup)?;

        // Load legacy accounts
        let (default_account_name, accounts) =
            legacy::load_info_from_legacy_wallet::<SmCrypto, _>(&legacy_file_backup)?;

        // Import legacy accounts
        config
            .context_settings
            .get_mut("default")
            .unwrap()
            .account_name = default_account_name;
        config.save()?;
        let mut wallet = Wallet::open(&data_dir)?;
        for (name, account) in accounts {
            wallet.save(name, account)?;
        }

        println!("Successfully migrated.");
    }

    let config = Config::open(data_dir)?;
    let mut ctx: Context<ControllerClient, ExecutorClient, EvmClient> =
        Context::from_config(config)?;

    if is_init {
        let default_account = Account::<SmCrypto>::generate();
        ctx.wallet
            .save("default".into(), default_account)
            .expect("cannot save default account");
    }

    let cldi = cmd::cldi_cmd();
    let m = cldi.get_matches();

    cldi.exec_with(&m, &mut ctx).inspect_err(|e| {
        if let Some(e) = e.downcast_ref::<clap::Error>() {
            e.exit();
        }
    })?;

    // Enter interactive mode if no subcommand provided
    if m.subcommand().is_none() {
        // TODO: put editor into context
        loop {
            let line = ctx.editor.readline("cldi> ");
            match line {
                Ok(line) => {
                    let _ = ctx.editor.add_history_entry(&line);

                    let args = match shell_words::split(&line) {
                        Ok(args) => args,
                        Err(e) => {
                            println!("parse error: `{e}`");
                            continue;
                        }
                    };
                    let input = std::iter::once(cldi.get_name().into()).chain(args);
                    if let Err(e) = cldi.exec_from(input, &mut ctx) {
                        println!("{e:?}");
                    }
                }
                Err(ReadlineError::Eof) => break,
                Err(ReadlineError::Interrupted) => println!("press CTRL-D to exit"),
                Err(e) => {
                    println!("readline error {e}");
                    break;
                }
            }
        }
    }

    Ok(())
}
