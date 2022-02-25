use clap::App;
use clap::Arg;
use serde_json::json;

use crate::config::CryptoType;
use crate::crypto::EthCrypto;
use crate::crypto::SmCrypto;
use crate::core::wallet::Account;
use crate::core::wallet::MaybeLocked;
use crate::core::wallet::MultiCryptoAccount;
use crate::utils::{parse_addr, parse_data};

use super::*;
use crate::core::context::Context;
use prost::Message;
use crate::display::Display;

use crate::crypto::{ArrayLike, Crypto};
use crate::utils::hex;

pub fn generate_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
{
    Command::<Context<Co, Ex, Ev>>::new("generate-key")
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
            let account: MultiCryptoAccount = match ctx.current_setting.crypto_type {
                CryptoType::Sm => Account::<SmCrypto>::generate().into(),
                CryptoType::Eth => Account::<EthCrypto>::generate().into(),
            };

            let maybe_locked: MaybeLocked = if let Some(pw) = pw {
                account.lock(pw.as_bytes()).into()
            } else {
                account.into()
            };
            // TODO: don't display secret key
            let output = json!(maybe_locked);

            ctx.wallet.save(id.into(), maybe_locked)?;

            println!("{output}");

            Ok(())
        })
}

pub fn list_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
{
    Command::<Context<Co, Ex, Ev>>::new("list")
        .aliases(&["ls", "l"])
        .about("list keys")
        .handler(|_cmd, _m, ctx| {
            let keys = ctx.wallet.list().map(|(id, account)| {
                json!({
                    "id": id,
                    "address": hex(account.address()),
                    "pubkey": hex(account.public_key()),
                    "is_locked": account.is_locked(),
                })
            }).collect::<Vec<_>>();

            let output = serde_json::to_string_pretty(&keys)?;
            println!("{}", output);

            Ok(())
        })
}

pub fn export_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
{
    Command::<Context<Co, Ex, Ev>>::new("export")
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

            let maybe_locked = ctx.wallet.get(id)
                .ok_or_else(|| anyhow!("account `{}` not found", id))?;

            let json = if let Some(pw) = pw {
                let unlocked = maybe_locked.unlock(pw.as_bytes())?;
                json!(unlocked)
            } else {
                let unlocked = maybe_locked.unlocked()?;
                json!(unlocked)
            };

            let output = serde_json::to_string_pretty(&json)?;
            println!("{}", output);

            Ok(())
        })
}

pub fn use_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
{
    Command::<Context<Co, Ex, Ev>>::new("use-key")
        .about("unlock a key to be used as default")
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

            if let Some(pw) = pw {
                ctx.wallet.unlock(id, pw.as_bytes())?;
            }
            ctx.wallet.get(id).ok_or_else(|| anyhow!("account `{}` not found", id))?;
            ctx.current_setting.account_id = id.into();

            Ok(())
        })
}

pub fn key_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
{
    Command::<Context<Co, Ex, Ev>>::new("key")
        .about("Key commands")
        .setting(AppSettings::SubcommandRequired)
        .subcommands([
            generate_key().name("generate"),
            list_key().name("list"),
            export_key().name("export"),
            use_key().name("use"),
        ])
}
