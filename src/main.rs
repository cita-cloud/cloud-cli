// mod cli;
// mod client;
mod core;
mod crypto;
mod display;
mod proto;
mod utils;
// mod wallet;
mod cmd;
// mod cmd;
mod config;
// mod interactive;

use rustyline::error::ReadlineError;
// use rustyline::KeyEvent;
// use rustyline::Cmd;
use crate::{
    config::Config,
    crypto::{EthCrypto, SmCrypto},
};

use crate::core::context::Context;
use crate::core::{controller::ControllerClient, evm::EvmClient, executor::ExecutorClient};
use anyhow::Context as _;
use anyhow::Result;
use std::{fs, io::Write};


fn main() -> Result<()> {
    let config = {
        let data_dir = {
            let home = home::home_dir().expect("cannot find home dir");
            home.join(".cloud-cli-v0.3.0")
        };
        Config::open(data_dir)?
    };
    let mut ctx: Context<ControllerClient, ExecutorClient, EvmClient> =
        Context::from_config(config)?;

    let cldi = cmd::cldi_cmd();

    let m = cldi.get_matches();
    if m.subcommand().is_some() {
        cldi.exec_with(&m, &mut ctx).map_err(|e| {
            if let Some(e) = e.downcast_ref::<clap::Error>() {
                e.exit();
            }
            e
        })?;
    } else {
        // TODO: simplify this, and fix `cldi -r addr` case
        // TODO: put editor into context
        let mut rl = rustyline::Editor::<()>::new();
        loop {
            let line = rl.readline("cldi> ");
            match line {
                Ok(line) => {
                    rl.add_history_entry(&line);

                    let args = match shell_words::split(&line) {
                        Ok(args) => args,
                        Err(e) => {
                            println!("parse error: `{}`", e);
                            continue;
                        }
                    };
                    let input = std::iter::once(cldi.get_name().into()).chain(args);
                    if let Err(e) = cldi.exec_from(input, &mut ctx) {
                        println!("{:?}", e);
                    }
                }
                Err(ReadlineError::Eof) => break,
                Err(ReadlineError::Interrupted) => println!("press CTRL+D to exit"),
                Err(e) => {
                    println!("readline error {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
