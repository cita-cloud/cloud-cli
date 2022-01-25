use clap::App;
use clap::Arg;

use crate::crypto::ArrayLike;
use crate::sdk::evm::EvmBehaviourExt;
use crate::utils::{parse_addr, parse_hash, parse_data};

use prost::Message;
use super::*;
use crate::sdk::context::Context;


use crate::proto::{
    blockchain::{
        raw_transaction::Tx, CompactBlock, RawTransaction, Transaction as CloudTransaction,
        UnverifiedTransaction, UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction,
        Witness,
    },
    common::{Address, Empty, Hash, NodeInfo, NodeNetInfo},
    controller::{
        rpc_service_client::RpcServiceClient as ControllerClient, BlockNumber, Flag, SystemConfig,
        TransactionIndex,
    },
    evm::{
        rpc_service_client::RpcServiceClient as EvmClient, Balance, ByteAbi, ByteCode, Nonce,
        Receipt,
    },
    executor::{executor_service_client::ExecutorServiceClient as ExecutorClient, CallRequest},
};

use crate::display::Display;
use crate::utils::hex;


pub fn get_receipt<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    Command::new("get-receipt")
        .about("Get receipt by tx_hash")
        .arg(
            Arg::new("tx_hash")
                .required(true)
                .validator(parse_hash::<C>)
        )
        .handler(|ctx, m| {
            let tx_hash = parse_hash::<C>(m.value_of("tx_hash").unwrap())?;

            let receipt = ctx.rt.block_on(ctx.get_receipt(tx_hash))?;
            println!("{}", receipt.display());
            Ok(())
        })
}

pub fn get_code<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    Command::new("get-code")
        .about("Get code by contract address")
        .arg(
            Arg::new("addr")
                .required(true)
                .validator(parse_addr::<C>)
        )
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;

            let byte_code = ctx.rt.block_on(ctx.get_code(addr))?;
            println!("{}", byte_code.display());
            Ok(())
        })
}

pub fn get_balance<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    Command::new("get-balance")
        .about("Get balance by account address")
        .arg(
            Arg::new("addr")
                .required(true)
                .validator(parse_addr::<C>)
        )
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;

            let balance = ctx.rt.block_on(ctx.get_balance(addr))?;
            println!("{}", balance.display());
            Ok(())
        })
}

pub fn get_tx_count<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    Command::new("get-tx-count")
        .about("Get the transaction count of the address")
        .arg(Arg::new("addr").required(true).validator(parse_addr::<C>))
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;

            let count = ctx.rt.block_on(ctx.get_tx_count(addr))?;
            println!("{}", count.display());
            Ok(())
        })
}

pub fn get_abi<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    Command::new("get-abi")
        .about("Get the specific contract ABI")
        .arg(
            Arg::new("addr")
                .required(true)
                .takes_value(true)
                .validator(parse_addr::<C>),
        )
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;

            let byte_abi = ctx.rt.block_on(ctx.get_abi(addr))?;
            println!("{}", byte_abi.display());
            Ok(())
        })
}

pub fn store_abi<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: EvmBehaviourExt<C>
{
    Command::new("store-abi")
        .about("Store contract ABI")
        .arg(
            Arg::new("addr")
                .short('a')
                .long("addr")
                .required(true)
                .takes_value(true)
                .validator(parse_addr::<C>),
        )
        .arg(
            Arg::new("abi")
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        )
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;
            let abi = parse_data(m.value_of("abi").unwrap())?;

            let tx_hash = ctx.rt.block_on(ctx.store_abi(addr, &abi))?;
            println!("{}", hex(tx_hash.as_slice()));
            Ok(())
        })
}


pub fn evm_cmd<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: EvmBehaviour<C> + EvmBehaviourExt<C>,
{
    Command::new("evm")
        .about("EVM commands")
        .setting(AppSettings::SubcommandRequired)
        .subcommands([
            get_receipt(),
            get_code(),
            get_tx_count(),
            get_balance(),
            get_abi(),
            store_abi(),
        ])
}
