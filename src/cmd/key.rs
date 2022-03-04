use std::str::FromStr;

use anyhow::anyhow;
use clap::Arg;
use serde_json::json;

use crate::{
    cmd::Command,
    config::CryptoType,
    core::{
        context::Context,
        wallet::{Account, MaybeLocked, MultiCryptoAccount},
    },
    crypto::{EthCrypto, SmCrypto},
    display::Display,
    utils::{hex, parse_sk},
};

pub fn generate_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("generate-key")
        .aliases(&["gen", "g"])
        .about("generate a new key")
        .arg(
            Arg::new("id")
                .help("The ID for the new generated key")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("passowrd")
                .help("The password to encrypt the key")
                .takes_value(true),
        )
        .arg(
            Arg::new("crypto-type")
                .help("The crypto type for the generated key. [default: <current-context-crypto-type>]")
                .long("crypto")
                .possible_values(["SM", "ETH"])
                .ignore_case(true)
                .validator(CryptoType::from_str)
        )
        .handler(|_cmd, m, ctx| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("password");
            let crypto_type = m.value_of("crypto-type")
                .map(|s| s.parse::<CryptoType>().unwrap())
                .unwrap_or(ctx.current_setting.crypto_type);
            let account: MultiCryptoAccount = match crypto_type {
                CryptoType::Sm => Account::<SmCrypto>::generate().into(),
                CryptoType::Eth => Account::<EthCrypto>::generate().into(),
            };

            let maybe_locked: MaybeLocked = if let Some(pw) = pw {
                account.lock(pw.as_bytes()).into()
            } else {
                account.into()
            };
            // TODO: don't display secret key
            let output = serde_json::to_string_pretty(&maybe_locked)?;

            ctx.wallet.save(id.into(), maybe_locked)?;

            println!("{output}");
            Ok(())
        })
}

pub fn list_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("list")
        .aliases(&["ls", "l"])
        .about("list keys")
        .handler(|_cmd, _m, ctx| {
            let keys = ctx
                .wallet
                .list()
                .map(|(id, account)| {
                    json!({
                        "id": id,
                        "address": hex(account.address()),
                        "pubkey": hex(account.public_key()),
                        "is_locked": account.is_locked(),
                        "crypto_type": account.crypto_type(),
                    })
                })
                .collect::<Vec<_>>();

            let output = serde_json::to_string_pretty(&keys)?;
            println!("{}", output);

            Ok(())
        })
}

pub fn import_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("import")
        .about("import key")
        .arg(
            Arg::new("id")
                .help("The ID of the key")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("password")
                .help("The password to decrypt the key")
                .short('p')
                .long("passowrd")
                .takes_value(true),
        )
        .arg(
            Arg::new("secret-key")
                .help("The secret key")
                .short('k')
                .long("sk")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("crypto-type")
                .help("The crypto type for the generated key. [default: <current-context-crypto-type>]")
                .long("crypto")
                .possible_values(["SM", "ETH"])
                .ignore_case(true)
                .validator(CryptoType::from_str)
        )
        .handler(|_cmd, m, ctx| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("password");
            let sk = m.value_of("secret-key").unwrap();
            let crypto_type = m.value_of("crypto-type")
                .map(|s| s.parse::<CryptoType>().unwrap())
                .unwrap_or(ctx.current_setting.crypto_type);

            let account: MultiCryptoAccount = match crypto_type {
                CryptoType::Sm => {
                    let sk = parse_sk::<SmCrypto>(sk)
                        .map_err(|e| anyhow!("invalid secret key for crypto type SM: {}", e))?;
                    Account::<SmCrypto>::from_secret_key(sk).into()
                }
                CryptoType::Eth => {
                    let sk = parse_sk::<EthCrypto>(sk)
                        .map_err(|e| anyhow!("invalid secret key for crypto type ETH: {}", e))?;
                    Account::<EthCrypto>::from_secret_key(sk).into()
                }
            };

            let addr = hex(account.address());
            let pubkey = hex(account.public_key());
            let info = json!({
                "address": addr,
                "pubkey": pubkey,
            });

            let maybe_locked: MaybeLocked = if let Some(pw) = pw {
                account.lock(pw.as_bytes()).into()
            } else {
                account.into()
            };
            ctx.wallet.save(id.into(), maybe_locked)?;

            println!("{}", info.display());
            Ok(())
        })
}

pub fn export_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("export")
        .about("export key")
        .arg(
            Arg::new("id")
                .help("The ID of the key")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("passowrd")
                .help("The password to decrypt the key")
                .takes_value(true),
        )
        .handler(|_cmd, m, ctx| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("password");

            let maybe_locked = ctx
                .wallet
                .get(id)
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

pub fn unlock_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("unlock-key")
        .about("unlock a key")
        .arg(
            Arg::new("id")
                .help("The ID of the key")
                .takes_value(true)
                .required(true), // TODO: add validator
        )
        .arg(
            Arg::new("password")
                .help("The password to unlock the key")
                .short('p')
                .long("passowrd")
                .takes_value(true)
                .required(true), // TODO: add validator
        )
        .arg(
            Arg::new("file")
                .help("Unlock the account file in keystore")
                .short('f')
                .long("file"),
        )
        .handler(|_cmd, m, ctx| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("password").unwrap();

            if m.is_present("file") {
                ctx.wallet.unlock(id, pw.as_bytes())?;
            } else {
                ctx.wallet.unlock_in_keystore(id, pw.as_bytes())?;
            }
            Ok(())
        })
}

pub fn lock_key<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("lock-key")
        .about("lock a key")
        .arg(
            Arg::new("id")
                .help("The ID of the key")
                .takes_value(true)
                .required(true), // TODO: add validator
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("passowrd")
                .help("The password to lock the key")
                .takes_value(true)
                .required(true), // TODO: add validator
        )
        .handler(|_cmd, m, ctx| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("password").unwrap();

            ctx.wallet.lock(id, pw.as_bytes())?;

            Ok(())
        })
}

pub fn key_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("key")
        .about("Key commands")
        .subcommand_required_else_help(true)
        .subcommands([
            generate_key().name("generate"),
            list_key().name("list"),
            import_key().name("import"),
            export_key().name("export"),
            unlock_key().name("unlock"),
            lock_key().name("lock"),
        ])
}
