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

use clap::Arg;

use crate::{
    cmd::Command,
    core::{
        context::Context, controller::ControllerBehaviour, evm::EvmBehaviour, evm::EvmBehaviourExt,
    },
    display::Display,
    utils::{get_block_height_at, parse_addr, parse_hash, parse_position},
};

pub fn get_receipt<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-receipt")
        .about("Get EVM execution receipt by tx_hash")
        .arg(
            Arg::new("tx_hash")
                .help("Transaction hash")
                .takes_value(true)
                .required(true)
                .validator(parse_hash),
        )
        .handler(|_cmd, m, ctx| {
            let tx_hash = parse_hash(m.value_of("tx_hash").unwrap())?;

            let receipt = ctx.rt.block_on(ctx.evm.get_receipt(tx_hash))??;
            println!("{}", receipt.display());
            Ok(())
        })
}

pub fn get_code<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-code")
        .about("Get code by contract address")
        .arg(
            Arg::new("addr")
                .help("Contract address")
                .takes_value(true)
                .required(true)
                .validator(parse_addr),
        )
        .handler(|_cmd, m, ctx| {
            let addr = parse_addr(m.value_of("addr").unwrap())?;

            let byte_code = ctx.rt.block_on(ctx.evm.get_code(addr))??;
            println!("{}", byte_code.display());
            Ok(())
        })
}

pub fn get_balance<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-balance")
        .about("Get balance by account address")
        .arg(
            Arg::new("addr")
                .help("Account address, default to current account")
                .takes_value(true)
                .validator(parse_addr),
        )
        .handler(|_cmd, m, ctx| {
            let addr = match m.value_of("addr") {
                Some(s) => parse_addr(s).unwrap(),
                None => *ctx.current_account()?.address(),
            };

            let balance = ctx.rt.block_on(ctx.evm.get_balance(addr))??;
            println!("{}", balance.display());
            Ok(())
        })
}

pub fn get_account_nonce<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-account-nonce")
        .about("Get the nonce of this account")
        .arg(
            Arg::new("addr")
                .help("Account address, default to current account")
                .takes_value(true)
                .validator(parse_addr),
        )
        .handler(|_cmd, m, ctx| {
            let addr = match m.value_of("addr") {
                Some(s) => parse_addr(s).unwrap(),
                None => *ctx.current_account()?.address(),
            };

            let nonce = ctx.rt.block_on(ctx.evm.get_tx_count(addr))??;
            println!("{}", nonce.display());
            Ok(())
        })
}

pub fn get_contract_abi<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-contract-abi")
        .about("Get the specific contract ABI")
        .arg(
            Arg::new("addr")
                .help("Contract address")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .handler(|_cmd, m, ctx| {
            let addr = parse_addr(m.value_of("addr").unwrap())?;

            let byte_abi = ctx.rt.block_on(ctx.evm.get_abi(addr))??;
            println!("{}", byte_abi.display());
            Ok(())
        })
}

pub fn store_abi<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + Send + Sync,
{
    Command::<Context<Co, Ex, Ev>>::new("store-abi")
        .about("Store EVM contract ABI")
        .arg(
            Arg::new("addr")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(Arg::new("abi").required(true).takes_value(true))
        .arg(
            Arg::new("quota")
                .help("the quota of this tx")
                .short('q')
                .long("quota")
                .takes_value(true)
                .default_value("3000000")
                .validator(str::parse::<u64>),
        )
        .arg(
            Arg::new("valid-until-block")
                .help("this tx is valid until the given block height. `+h` means `<current-height> + h`")
                .long("until")
                .takes_value(true)
                .default_value("+95")
                .validator(parse_position),
        )
        .handler(|_cmd, m, ctx| {
            let tx_hash = ctx.rt.block_on(async {
                let contract_addr = parse_addr(m.value_of("addr").unwrap())?;
                let abi = m.value_of("abi").unwrap();
                let quota = m.value_of("quota").unwrap().parse::<u64>()?;
                let valid_until_block = {
                    let pos = parse_position(m.value_of("valid-until-block").unwrap())?;
                    get_block_height_at(&ctx.controller, pos).await?
                };

                let signer = ctx.current_account()?;
                ctx.controller
                    .store_contract_abi(
                        signer,
                        contract_addr,
                        abi.as_bytes(),
                        quota,
                        valid_until_block,
                    )
                    .await
            })??;
            println!("{}", tx_hash.display());
            Ok(())
        })
}
