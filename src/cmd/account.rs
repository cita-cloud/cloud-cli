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

pub fn generate_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("generate-account")
        .aliases(&["gen", "g"])
        .about("generate a new account")
        .arg(
            Arg::new("name")
                .help("The name for the new generated account, default to account address")
                .long("name")
                .takes_value(true),
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("passowrd")
                .help("The password to encrypt the account")
                .takes_value(true),
        )
        .arg(
            Arg::new("crypto-type")
                .help("The crypto type for the generated account. [default: <current-context-crypto-type>]")
                .long("crypto")
                .possible_values(["SM", "ETH"])
                .ignore_case(true)
                .validator(CryptoType::from_str)
        )
        .handler(|_cmd, m, ctx| {
            let name = m.value_of("name").map(str::to_string);
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

            let name = name.unwrap_or_else(|| hex(maybe_locked.address()));
            ctx.wallet.save(name, maybe_locked)?;

            println!("{output}");
            Ok(())
        })
}

pub fn list_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("list")
        .aliases(&["ls", "l"])
        .about("list accounts")
        .handler(|_cmd, _m, ctx| {
            let accounts = ctx
                .wallet
                .list()
                .map(|(name, account)| {
                    json!({
                        "name": name,
                        "address": hex(account.address()),
                        "pubkey": hex(account.public_key()),
                        "is_locked": account.is_locked(),
                        "crypto_type": account.crypto_type(),
                    })
                })
                .collect::<Vec<_>>();

            let output = serde_json::to_string_pretty(&accounts)?;
            println!("{}", output);

            Ok(())
        })
}

pub fn import_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("import")
        .about("import account")
        .arg(
            Arg::new("secret-key")
                .help("The secret key")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("name")
                .help("The name of the account, default to account address")
                .long("name")
                .takes_value(true),
        )
        .arg(
            Arg::new("password")
                .help("The password to encrypt the account")
                .short('p')
                .long("password")
                .takes_value(true),
        )
        .arg(
            Arg::new("crypto-type")
                .help("The crypto type for the imported account. [default: <current-context-crypto-type>]")
                .long("crypto")
                .possible_values(["SM", "ETH"])
                .ignore_case(true)
                .validator(CryptoType::from_str)
        )
        .handler(|_cmd, m, ctx| {
            let name = m.value_of("name").map(str::to_string);
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

            let name = name.unwrap_or(addr);
            if let Some(pw) = pw {
                let pw = pw.as_bytes();
                let locked = account.lock(pw);

                ctx.wallet.save(name.clone(), locked)?;
                ctx.wallet.unlock(&name, pw)?;
            } else {
                ctx.wallet.save(name, account)?;
            };

            println!("{}", info.display());
            Ok(())
        })
}

pub fn export_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("export")
        .about("export account")
        .arg(
            Arg::new("name")
                .help("The name of the account")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("password")
                .help("The password to decrypt the account")
                .short('p')
                .long("password")
                .takes_value(true),
        )
        .handler(|_cmd, m, ctx| {
            let name = m.value_of("name").unwrap();
            let pw = m.value_of("password");

            let maybe_locked = ctx
                .wallet
                .get(name)
                .ok_or_else(|| anyhow!("account `{}` not found", name))?;

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

pub fn unlock_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("unlock-account")
        .about("unlock a account")
        .arg(
            Arg::new("name")
                .help("The name of the account")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("password")
                .help("The password to unlock the account")
                .short('p')
                .long("password")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("file")
                .help("Unlock the account file in keystore")
                .short('f')
                .long("file"),
        )
        .handler(|_cmd, m, ctx| {
            let name = m.value_of("name").unwrap();
            let pw = m.value_of("password").unwrap();

            if m.is_present("file") {
                ctx.wallet.unlock(name, pw.as_bytes())?;
            } else {
                ctx.wallet.unlock_in_keystore(name, pw.as_bytes())?;
            }
            Ok(())
        })
}

pub fn lock_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("lock-account")
        .about("lock a account")
        .arg(
            Arg::new("name")
                .help("The name of the account")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("password")
                .help("The password to lock the account")
                .short('p')
                .long("password")
                .takes_value(true)
                .required(true),
        )
        .handler(|_cmd, m, ctx| {
            let name = m.value_of("name").unwrap();
            let pw = m.value_of("password").unwrap();

            ctx.wallet.lock(name, pw.as_bytes())?;

            Ok(())
        })
}

pub fn account_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("account")
        .about("Accounts commands")
        .subcommand_required_else_help(true)
        .subcommands([
            generate_account().name("generate"),
            list_account().name("list"),
            import_account().name("import"),
            export_account().name("export"),
            unlock_account().name("unlock"),
            lock_account().name("lock"),
        ])
}
