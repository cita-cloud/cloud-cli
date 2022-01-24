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

    // async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash>;

    // async fn get_block_hash(&self, block_number: u64) -> Result<C::Hash>;

    // async fn get_tx(&self, tx_hash: C::Hash) -> Result<RawTransaction>;
    // async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<u64>;
    // async fn get_tx_block_number(&self, tx_hash: C::Hash) -> Result<u64>;

    // async fn get_peer_count(&self) -> Result<u64>;
    // async fn get_peers_info(&self) -> Result<Vec<NodeInfo>>;

    // async fn add_node(&self, multiaddr: String) -> Result<u32>;

pub fn get_system_config<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa> 
where
    C: Crypto,
    Context<Co, Ex, Ev, Wa>: ControllerBehaviour<C>
{
    Command::new("get-system-config")
        .about("Get system config")
        .handler(|ctx, _m| {
            let system_config = ctx.rt.block_on(ctx.get_system_config())?;
            println!("{}", system_config.display());
            Ok(())
        })
}

pub fn get_block_number<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa> 
where
    C: Crypto,
    Context<Co, Ex, Ev, Wa>: ControllerBehaviour<C>
{
    Command::new("get-block-number")
        .about("Get block number")
        .arg(
            Arg::new("for_pending")
                .help("if set, get block number of the pending block")
                .short('p')
                .long("for_pending"),
        )
        .handler(|ctx, m| {
            let for_pending = m.is_present("for_pending");

            let block_number = ctx.rt.block_on(ctx.get_block_number(for_pending))?;
            println!("block_number: {}", block_number);
            Ok(())
        })
}

pub fn get_block<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa> 
where
    C: Crypto,
    Context<Co, Ex, Ev, Wa>: ControllerBehaviour<C>
{
    Command::new("get-block")
        .about("Get block by block number(height) or hash(0x)")
        .arg(
            Arg::new("number_or_hash")
                .help("plain decimal number or hash with `0x` prefix")
                .required(true)
                .takes_value(true)
                .validator(|s| {
                    if s.starts_with("0x") {
                        parse_hash::<C>(s)?;
                    } else {
                        s.parse::<u64>().context("cannot parse block number, if you want to get block by hash, please prefix it with `0x`")?;
                    }
                    anyhow::Ok(())
                })
        )
        .handler(|ctx, m| {
            let s = m.value_of("number_or_hash").unwrap();
            let block = if s.starts_with("0x") {
                let hash = parse_hash::<C>(s)?;
                ctx.rt.block_on(ctx.get_block_by_hash(hash))?
            } else {
                let block_number = s.parse()?;
                ctx.rt.block_on(ctx.get_block_by_number(block_number))?
            };

            println!("{}", block.display());
            Ok(())
        })
}

pub fn controller_cmd<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa> 
where
    C: Crypto,
    Context<Co, Ex, Ev, Wa>: ControllerBehaviour<C>
{
    Command::new("controller")
        .about("controller commands")
        .subcommands([
            get_system_config(),
            get_block_number(),
            get_block()
        ])
}
