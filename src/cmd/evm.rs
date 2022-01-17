use clap::App;
use clap::Arg;

use crate::utils::{parse_addr, parse_hash};

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


pub fn get_receipt<C, Ac, Co, Ex, Ev, Wa>() -> Command<Ac, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Ac, Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    let app = App::new("get-receipt")
        .about("Get receipt by tx_hash")
        .arg(
            Arg::new("tx_hash")
                .required(true)
                .validator(parse_hash::<C>)
        );

    Command::new(app)
        .handler(|ctx, m| {
            let tx_hash = parse_hash::<C>(m.value_of("tx_hash").unwrap())?;

            let receipt = ctx.rt.block_on(ctx.get_receipt(tx_hash))?;
            println!("{}", receipt.display());
            Ok(())
        })
}

pub fn get_code<C, Ac, Co, Ex, Ev, Wa>() -> Command<Ac, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Ac, Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    let app = App::new("get-code")
        .about("Get code by contract address")
        .arg(
            Arg::new("addr")
                .required(true)
                .validator(parse_addr::<C>)
        );

    Command::new(app)
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;

            let byte_code = ctx.rt.block_on(ctx.get_code(addr))?;
            println!("{}", byte_code.display());
            Ok(())
        })
}

pub fn get_balance<C, Ac, Co, Ex, Ev, Wa>() -> Command<Ac, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Ac, Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    let app = App::new("get-balance")
        .about("Get balance by account address")
        .arg(
            Arg::new("addr")
                .required(true)
                .validator(parse_addr::<C>)
        );

    Command::new(app)
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;

            let balance = ctx.rt.block_on(ctx.get_balance(addr))?;
            println!("{}", balance.display());
            Ok(())
        })
}

pub fn get_tx_count<C, Ac, Co, Ex, Ev, Wa>() -> Command<Ac, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Ac, Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    let app = App::new("get-tx-count")
        .about("Get the transaction count of the address")
        .arg(Arg::new("addr").required(true).validator(parse_addr::<C>));

    Command::new(app)
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;

            let count = ctx.rt.block_on(ctx.get_tx_count(addr))?;
            println!("{}", count.display());
            Ok(())
        })
}

pub fn get_abi<C, Ac, Co, Ex, Ev, Wa>() -> Command<Ac, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Ac, Co, Ex, Ev, Wa>: EvmBehaviour<C>
{
    let app = App::new("get-abi").about("Get the specific contract ABI").arg(
        Arg::new("addr")
            .required(true)
            .takes_value(true)
            .validator(parse_addr::<C>),
    );

    Command::new(app)
        .handler(|ctx, m| {
            let addr = parse_addr::<C>(m.value_of("addr").unwrap())?;

            let byte_abi = ctx.rt.block_on(ctx.get_abi(addr))?;
            println!("{}", byte_abi.display());
            Ok(())
        })
}
