// mod cli;
// mod client;
mod crypto;
mod display;
mod proto;
mod sdk;
mod utils;
// mod wallet;
// mod cmd;
mod cmd;
mod config;
// mod interactive;

use rustyline::error::ReadlineError;
// use rustyline::KeyEvent;
// use rustyline::Cmd;
use crate::{
    config::Config,
    crypto::{ EthCrypto, SmCrypto },
};


use anyhow::Result;
use std::{fs, io::Write};


// FIXME
fn load_config() -> Result<Config> {
    let data_dir = {
        let home = home::home_dir().expect("cannot find home dir");
        home.join(".cloud-cli-v0.3.0")
    };
    if data_dir.exists() && data_dir.is_file() {
        todo!("migrate old wallet")
    } else {
        fs::create_dir_all(&data_dir)?;
    }

    let config: Config = {
        let path = data_dir.join("config.toml");
        if path.exists() {
            let s = fs::read_to_string(path)?;
            toml::from_str(&s)?
        } else {
            let mut f = fs::File::options()
                .create_new(true)
                .write(true)
                .open(path)?;
            
            let default_config = Config::default();
            let content = toml::to_string_pretty(&default_config)?;

            f.write_all(content.as_bytes())?;
            default_config
        }

    };

    Ok(config)
}

fn main() -> Result<()> {
    let config = load_config()?;
    let mut ctx = sdk::context::from_config::<SmCrypto>(config).unwrap();

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
        // TODO: simplify this, and fix `cldi -r addr` case
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
