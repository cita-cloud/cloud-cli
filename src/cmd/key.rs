use clap::App;
use clap::Arg;

use crate::sdk::wallet::MaybeLockedAccount;
use crate::utils::{parse_addr, parse_data};

use super::*;
use crate::sdk::context::Context;
use prost::Message;

use crate::crypto::{ArrayLike, Crypto};
use crate::sdk::wallet::WalletBehaviour;
use crate::utils::hex;

pub fn generate_key<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Wa: WalletBehaviour<C>,
{
    Command::new("generate")
        .aliases(&["gen", "g"])
        .about("generate a new key")
        .arg(
            Arg::new("id")
                .long("id")
                .short('u')
                .help("The ID for the new generated key")
                .required(true)
                .takes_value(true), // TODO: add validator
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("passowrd")
                .help("The password to encrypt the key")
                .takes_value(true), // TODO: add validator
        )
        .handler(|_cmd, m, ctx| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("password");
            let account = ctx.rt.handle().clone().block_on(async {
                let wallet = &mut ctx.wallet;

                wallet.generate_account(id, pw).await?;
                if let Some(pw) = pw {
                    // TODO: maybe auto unlock generated account?
                    wallet.unlock_account(id, pw).await?;
                }
                wallet.get_account(id).await
            })?;

            let addr = hex(account.address().as_slice());
            println!("key `{id}` generated, address: {addr}");

            Ok(())
        })
}

pub fn list_key<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Wa: WalletBehaviour<C>,
{
    Command::new("list")
        .aliases(&["ls", "l"])
        .about("list keys")
        .handler(|_cmd, _m, ctx| {
            let id_and_accounts = ctx.rt.block_on(ctx.wallet.list_account());

            for (id, account) in id_and_accounts {
                // TODO: impl crate::display::Display
                match account {
                    MaybeLockedAccount::Locked(..) => {
                        println!("id `{id}`: locked");
                    }
                    MaybeLockedAccount::Unlocked(unlocked) => {
                        let addr = hex(unlocked.address().as_slice());
                        let pk = hex(unlocked.public_key().as_slice());
                        println!("id: `{id}`\naddress: {addr}\npubkey: {pk}");
                    }
                }
                println!();
            }
            Ok(())
        })
}

pub fn export_key<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Wa: WalletBehaviour<C>,
{
    Command::new("export")
        .about("export key")
        .arg(
            Arg::new("id")
                .long("id")
                .short('u')
                .help("The ID of the key")
                .required(true)
                .takes_value(true), // TODO: add validator
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("passowrd")
                .help("The password to decrypt the key")
                .takes_value(true), // TODO: add validator
        )
        .handler(|_cmd, m, ctx| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("password");
            ctx.rt.handle().clone().block_on(async {
                let wallet = &mut ctx.wallet;

                if let Some(pw) = pw {
                    wallet.unlock_account(id, pw)
                        .await
                        .context("failed to export account, please check your password")?;
                }

                let account = wallet.get_account(id).await.context("failed to get key")?;

                let addr = hex(account.address().as_slice());
                let pk = hex(account.public_key().as_slice());
                let sk = hex(account.expose_secret_key().as_slice());
                println!("id: `{id}`\naddress: {addr}\npubkey: {pk}\nprivkey: {sk}");

                anyhow::Ok(())
            })?;

            Ok(())
        })
}

pub fn use_key<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Wa: WalletBehaviour<C>,
{
    Command::new("use-key")
        .about("select a key as default")
        .arg(
            Arg::new("id")
                .help("The ID of the key")
                .required(true)
                .takes_value(true), // TODO: add validator
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("passowrd")
                .help("The password to unlock the key if neccessary")
                .takes_value(true), // TODO: add validator
        )
        .handler(|_cmd, m, ctx| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("password");
            ctx.rt.block_on(async {
                let wallet = &mut ctx.wallet;

                if let Some(pw) = pw {
                    wallet.unlock_account(id, pw)
                        .await
                        .context("failed to export account, please check your password")?;
                }

                wallet.set_current_account(id).await?;

                anyhow::Ok(())
            })?;

            Ok(())
        })
}

pub fn key_cmd<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Wa: WalletBehaviour<C>,
{
    Command::new("key")
        .about("key commands")
        .setting(AppSettings::SubcommandRequired)
        .subcommands([
            generate_key(),
            list_key(),
            export_key(),
            use_key().name("use"),
        ])
}
