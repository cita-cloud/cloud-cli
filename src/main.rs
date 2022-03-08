mod cmd;
mod config;
mod core;
mod crypto;
mod display;
mod proto;
mod utils;

use anyhow::Result;
use crypto::SmCrypto;
use rustyline::error::ReadlineError;

use crate::{
    config::Config,
    core::{
        context::Context, controller::ControllerClient, evm::EvmClient, executor::ExecutorClient,
        wallet::Account,
    },
    utils::init_local_utc_offset,
};

fn main() -> Result<()> {
    // This should be called without any other concurrent running threads.
    init_local_utc_offset();

    let data_dir = {
        let home = home::home_dir().expect("cannot find home dir");
        home.join(".cloud-cli-v0.3.0")
    };
    let is_init = !data_dir.exists();

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

    cldi.exec_with(&m, &mut ctx).map_err(|e| {
        if let Some(e) = e.downcast_ref::<clap::Error>() {
            e.exit();
        }
        e
    })?;

    // Enter interactive mode if no subcommand provided
    if m.subcommand().is_none() {
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
