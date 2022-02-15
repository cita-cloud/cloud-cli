// mod cli;
// mod client;
mod crypto;
mod display;
mod proto;
mod sdk;
mod utils;
// mod wallet;
mod cmd;
mod config;
// mod interactive;

use rustyline::error::ReadlineError;
// use rustyline::KeyEvent;
// use rustyline::Cmd;
use crate::{
    sdk::context::Context,
    cmd::Command,
    config::Config,
    crypto::{ EthCrypto, SmCrypto },
};

use anyhow::Result;

fn main() -> Result<()> {
    let config = Config {
        controller_addr: "localhost:50005".into(),
        executor_addr: "localhost:50002".into(),
        default_account: None,
        wallet_dir: "d:/cld/cloud-cli/tmp-wallet".into(),
    };

    let mut ctx = sdk::context::from_config::<SmCrypto>(&config).unwrap();

    let cldi = cmd::cldi_cmd();
    let m = cldi.get_matches();
    if m.subcommand().is_some() {
        cldi.exec_with(&m, &mut ctx).map_err(|e|{
            if let Some(e) = e.downcast_ref::<clap::Error>() {
                e.exit();
            }
            e
        })?;
    } else {
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
                    println!("readline error {}", e)
                }
            }
        }
    }

    Ok(())
}
