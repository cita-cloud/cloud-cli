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
                .long("password")
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
            let pw = m.value_of("password").map(str::as_bytes);
            let crypto_type = m.value_of("crypto-type")
                .map(|s| s.parse::<CryptoType>().unwrap())
                .unwrap_or(ctx.current_setting.crypto_type);
            let account: MultiCryptoAccount = match crypto_type {
                CryptoType::Sm => Account::<SmCrypto>::generate().into(),
                CryptoType::Eth => Account::<EthCrypto>::generate().into(),
            };

            let maybe_locked: MaybeLocked = if let Some(pw) = pw {
                account.lock(pw).into()
            } else {
                account.into()
            };
            // TODO: don't display secret key
            let output = serde_json::to_string_pretty(&maybe_locked)?;

            let name = name.unwrap_or_else(|| hex(maybe_locked.address()));
            ctx.wallet.save(name.clone(), maybe_locked)?;
            // Make generated account usable without having to unlock it.
            if let Some(pw) = pw {
                ctx.wallet.unlock(&name, pw)?;
            }

            println!("{output}");
            Ok(())
        })
}

pub fn list_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("list")
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
            let pw = m.value_of("password").map(str::as_bytes);
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
            let pw = m.value_of("password").map(str::as_bytes);

            let maybe_locked = ctx.wallet.get(name)?;

            let json = if let Some(pw) = pw {
                let unlocked = maybe_locked.unlock(pw)?;
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
        .about("unlock account in keystore")
        .arg(
            Arg::new("name")
                .help("The name of the account")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("password")
                .help("The password of the account")
                .short('p')
                .long("password")
                .takes_value(true)
                .required(true),
        )
        .handler(|_cmd, m, ctx| {
            let name = m.value_of("name").unwrap();
            let pw = m.value_of("password").unwrap().as_bytes();

            ctx.wallet.unlock_in_keystore(name, pw)?;

            Ok(())
        })
}

pub fn lock_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("lock-account")
        .about("lock account in keystore")
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
            let pw = m.value_of("password").unwrap().as_bytes();

            ctx.wallet.lock(name, pw)?;

            Ok(())
        })
}

pub fn delete_account<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("delete-account")
        .about("delete account")
        .arg(
            Arg::new("name")
                .help("The name of the account")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("yes")
                .help("Don't ask for confirmation")
                .short('y')
                .long("yes"),
        )
        .handler(|_cmd, m, ctx| {
            let name = m.value_of("name").unwrap();
            // If account doesn't exit, report it now.
            ctx.wallet.get(name)?;
            if !m.is_present("yes") {
                let prompt = format!("Are you sure to delete the account `{name}`? (y/n) ");
                loop {
                    match ctx
                        .editor
                        .readline(&prompt)
                        .map(|s| s.trim().to_ascii_lowercase())
                    {
                        Ok(s) if s == "yes" || s == "y" => break,
                        Ok(s) if s == "no" || s == "n" => return Ok(()),
                        // Ask again.
                        Ok(_) => (),
                        // Exits silently.
                        _ => return Ok(()),
                    };
                }
            }
            ctx.wallet.remove(name)?;
            println!("account `{name}` deleted");

            Ok(())
        })
}

pub fn account_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("account")
        .about("Account commands")
        .subcommand_required_else_help(true)
        .subcommands([
            generate_account()
                .name("generate")
                .aliases(&["gen", "g", "create"]),
            list_account().name("list").aliases(&["ls", "l"]),
            import_account().name("import"),
            export_account().name("export"),
            unlock_account().name("unlock"),
            lock_account().name("lock"),
            delete_account()
                .name("delete")
                .aliases(&["del", "rm", "remove"]),
        ])
}

#[cfg(test)]
mod tests {
    use crate::cmd::cldi_cmd;
    use crate::core::mock::context;

    #[test]
    fn test_account_subcmds() {
        let cldi_cmd = cldi_cmd();
        let (mut ctx, _temp_dir) = context();

        cldi_cmd
            .exec_from(["cldi", "account", "list"], &mut ctx)
            .unwrap();
        cldi_cmd
            .exec_from(["cldi", "account", "generate", "--name", "test"], &mut ctx)
            .unwrap();
        cldi_cmd
            .exec_from(["cldi", "account", "export", "test"], &mut ctx)
            .unwrap();
        cldi_cmd
            .exec_from(["cldi", "account", "delete", "test", "--yes"], &mut ctx)
            .unwrap();
    }
}
