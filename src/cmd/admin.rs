use clap::App;
use clap::Arg;

use crate::context::Context;
use crate::utils::{parse_addr, parse_data, parse_value};

use prost::Message;
use super::*;


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

use crate::crypto::{ArrayLike, Crypto};
use crate::utils::hex;

pub fn update_admin<C: Crypto>() -> Command<C> {
    let app = App::new("update-admin")
        .about("Update admin of the chain")
        .arg(
            Arg::new("admin")
                .help("the address of the new admin")
                .required(true)
                .validator(parse_addr)
        );
    Command::new(app)
        .handler(|ctx, m| {
            let admin = parse_addr(m.value_of("admin").unwrap())?;
            let tx_hash = ctx.rt.block_on(ctx.update_admin(admin))?;
            println!("tx_hash: {}", hex(&tx_hash));
            Ok(())
        })
}

pub fn update_validators<C: Crypto>() -> Command<C> {
    let app = App::new("update-validators")
        .about("Update validators of the chain")
        .arg(
            Arg::new("validators")
                .help("a space-separated list of the new validator addresses, e.g. `cldi update-validators 0x12..34 0xab..cd`")
                .required(true)
                .multiple_values(true)
                .validator(parse_addr)
        );
    Command::new(app)
        .handler(|ctx, m| {
            let validators = m
                .values_of("validators")
                .unwrap()
                .map(parse_addr)
                .collect::<Result<Vec<_>>>()?;
            let tx_hash = ctx.rt.block_on(ctx.update_validators(&validators))?;
            println!("tx_hash: {}", hex(&tx_hash));
            Ok(())
        })
}

pub fn set_block_interval<C: Crypto>() -> Command<C> {
    let app = App::new("set-block-interval")
        .about("Set block interval")
        .arg(
            Arg::new("block_interval")
                .help("new block interval")
                .required(true)
                .validator(str::parse::<u32>),
        );
    Command::new(app)
        .handler(|ctx, m| {
            let block_interval = m.value_of("block_interval").unwrap().parse::<u32>()?;
            let tx_hash = ctx.rt.block_on(ctx.set_block_interval(block_interval))?;
            println!("tx_hash: {}", hex(&tx_hash));
            Ok(())
        })
}

pub fn emergency_brake<C: Crypto>() -> Command<C> {
    let app = App::new("emergency-brake")
        .about("Send emergency brake cmd to chain")
        .arg(
            Arg::new("switch")
                .help("turn on/off")
                .required(true)
                .possible_values(&["on", "off"]),
        );
    Command::new(app)
        .handler(|ctx, m| {
            let switch = m.value_of("switch").unwrap() == "on";
            let tx_hash = ctx.rt.block_on(ctx.emergency_brake(switch))?;
            println!("tx_hash: {}", hex(&tx_hash));
            Ok(())
        })
}
