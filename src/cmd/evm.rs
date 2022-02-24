use clap::App;
use clap::Arg;

use crate::crypto::ArrayLike;
use crate::sdk::evm::EvmBehaviour;
use crate::sdk::evm::EvmBehaviourExt;
use crate::sdk::executor::ExecutorBehaviour;
use crate::utils::{parse_addr, parse_data, parse_hash};

use super::*;
use crate::sdk::context::Context;
use prost::Message;

use crate::display::Display;
use crate::utils::hex;

pub fn get_receipt<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-receipt")
        .about("Get receipt by tx_hash")
        .arg(
            Arg::new("tx_hash")
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
        .arg(Arg::new("addr").required(true).validator(parse_addr))
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
        .arg(Arg::new("addr").required(true).validator(parse_addr))
        .handler(|_cmd, m, ctx| {
            let addr = parse_addr(m.value_of("addr").unwrap())?;

            let balance = ctx.rt.block_on(ctx.evm.get_balance(addr))??;
            println!("{}", balance.display());
            Ok(())
        })
}

pub fn get_tx_count<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-tx-count")
        .about("Get the transaction count of the address")
        .arg(Arg::new("addr").required(true).validator(parse_addr))
        .handler(|_cmd, m, ctx| {
            let addr = parse_addr(m.value_of("addr").unwrap())?;

            let count = ctx.rt.block_on(ctx.evm.get_tx_count(addr))??;
            println!("{}", count.display());
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

pub fn store_contract_abi<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: EvmBehaviourExt,
{
    Command::<Context<Co, Ex, Ev>>::new("store-contract-abi")
        .about("Store contract ABI")
        .arg(
            Arg::new("addr")
                .short('a')
                .long("addr")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(
            Arg::new("abi")
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        )
        .handler(|_cmd, m, ctx| {
            let contract_addr = parse_addr(m.value_of("addr").unwrap())?;
            let abi = parse_data(m.value_of("abi").unwrap())?;

            let signer = ctx.current_account()?;
            let tx_hash = ctx.rt.block_on(async {
                ctx.controller.store_contract_abi(signer, contract_addr, &abi).await
            })??;
            println!("{}", hex(tx_hash.as_slice()));
            Ok(())
        })
}

pub fn evm_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: EvmBehaviourExt,
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("evm")
        .about("EVM commands")
        .setting(AppSettings::SubcommandRequired)
        .subcommands([
            get_receipt(),
            get_code(),
            get_tx_count(),
            get_balance(),
            get_contract_abi(),
            store_contract_abi(),
        ])
}
