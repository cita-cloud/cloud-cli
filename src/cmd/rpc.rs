use clap::App;
use clap::Arg;

use crate::utils::parse_value;
use crate::utils::{parse_addr, parse_hash, parse_data, hex};

use super::*;
use crate::sdk::context::Context;
use prost::Message;
use crate::sdk::controller::TransactionSenderBehaviour;
use crate::sdk::wallet::WalletBehaviour;

use crate::display::Display;
use crate::crypto::ArrayLike;


pub fn call<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Ex: ExecutorBehaviour<C>,
{
    Command::new("call")
        .about("Executor call")
        .arg(
            Arg::new("from")
                .short('f')
                .long("from")
                .takes_value(true)
                .validator(parse_addr::<C>),
        )
        .arg(
            Arg::new("to")
                .short('t')
                .long("to")
                .required(true)
                .takes_value(true)
                .validator(parse_addr::<C>),
        )
        .arg(
            Arg::new("data")
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        )
        .handler(|_cmd, m, ctx| {
            let from = parse_addr::<C>(m.value_of("from").unwrap_or_default())?;
            let to = parse_addr::<C>(m.value_of("to").unwrap())?;
            let data = parse_data(m.value_of("data").unwrap_or_default())?;

            let resp = ctx.rt.block_on(ctx.executor.call(from, to, data))?;
            println!("{}", resp.display());
            Ok(())
        })
}


pub fn send<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: TransactionSenderBehaviour<C>,
    Wa: WalletBehaviour<C>,
{
    Command::new("send")
        .about("Send transaction")
        .arg(
            Arg::new("to")
                .help("the address to send")
                .short('t')
                .long("to")
                .required(true)
                .takes_value(true)
                .validator(parse_addr::<C>),
        )
        .arg(
            Arg::new("value")
                .help("the value to send")
                .short('v')
                .long("value")
                .takes_value(true)
                .validator(parse_value),
        )
        .arg(
            Arg::new("data")
                .help("the data of the tx")
                .short('d')
                .long("data")
                .takes_value(true)
                .validator(parse_data),
        )
        .handler(|_cmd, m, ctx| {
            let to = parse_addr::<C>(m.value_of("to").unwrap())?;
            let value = parse_value(m.value_of("value").unwrap_or_default())?.to_vec();
            let data = parse_data(m.value_of("data").unwrap_or_default())?;

            ctx.rt.block_on(async {
                let signer = ctx.wallet.current_account().await?.1;
                let tx_hash = ctx.rt.block_on(ctx.controller.send_tx(signer, to, value, data))?;
                println!("{}", hex(tx_hash.as_slice()));

                anyhow::Ok(())
            })?;
            Ok(())
        })
}

pub fn get_system_config<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("get-system-config")
        .about("Get system config")
        .handler(|_cmd, _m, ctx| {
            let system_config = ctx.rt.block_on(ctx.controller.get_system_config())?;
            println!("{}", system_config.display());
            Ok(())
        })
}

pub fn get_block<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("get-block")
        .about("Get block by block height or hash(0x)")
        .arg(
            Arg::new("height_or_hash")
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
        .handler(|_cmd, m, ctx| {
            let s = m.value_of("height_or_hash").unwrap();
            let block = if s.starts_with("0x") {
                let hash = parse_hash::<C>(s)?;
                ctx.rt.block_on(ctx.controller.get_block_by_hash(hash))?
            } else {
                let height = s.parse()?;
                ctx.rt.block_on(ctx.controller.get_block_by_number(height))?
            };

            println!("{}", block.display());
            Ok(())
        })
}

pub fn get_block_number<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("get-block-number")
        .about("Get block number")
        .arg(
            Arg::new("for_pending")
                .help("if set, get block number of the pending block")
                .short('p')
                .long("for_pending"),
        )
        .handler(|_cmd, m, ctx| {
            let for_pending = m.is_present("for_pending");

            let block_number = ctx.rt.block_on(ctx.controller.get_block_number(for_pending))?;
            println!("{}", block_number);
            Ok(())
        })
}

pub fn get_block_hash<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("get-block-hash")
        .about("Get block hash by block height")
        .arg(
            Arg::new("height")
                .help("the block height")
                .takes_value(true)
                .validator(str::parse::<u64>),
        )
        .handler(|_cmd, m, ctx| {
            let height = m.value_of("height").unwrap().parse()?;
            let hash = ctx.rt.block_on(ctx.controller.get_block_hash(height))?;
            println!("{}", hex(hash.as_slice()));

            Ok(())
        })
}

pub fn get_tx<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("get-tx")
        .about("Get transaction by hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_hash::<C>))
        .handler(|_cmd, m, ctx| {
            let s = m.value_of("tx_hash").unwrap();
            let tx_hash = parse_hash::<C>(s)?;
            let tx = ctx.rt.block_on(ctx.controller.get_tx(tx_hash))?;
            println!("{}", tx.display());

            Ok(())
        })
}

pub fn get_tx_index<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("get-tx-index")
        .about("Get transaction's index by tx_hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_hash::<C>))
        .handler(|_cmd, m, ctx| {
            let s = m.value_of("tx_hash").unwrap();
            let tx_hash = parse_hash::<C>(s)?;
            let tx_index = ctx.rt.block_on(ctx.controller.get_tx_index(tx_hash))?;
            println!("{}", tx_index);

            Ok(())
        })
}

pub fn get_tx_block_number<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("get-tx-block-height")
        .about("Get transaction's block height by tx_hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_hash::<C>))
        .handler(|_cmd, m, ctx| {
            let s = m.value_of("tx_hash").unwrap();
            let tx_hash = parse_hash::<C>(s)?;
            let height = ctx.rt.block_on(ctx.controller.get_tx_block_number(tx_hash))?;
            println!("{}", height);

            Ok(())
        })
}

pub fn get_peer_count<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("peer-count")
        .about("Get peer count")
        .handler(|_cmd, _m, ctx| {
            let peer_count = ctx.rt.block_on(ctx.controller.get_peer_count())?;
            println!("{}", peer_count);

            Ok(())
        })
}

pub fn get_peers_info<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("peers-info")
        .about("Get peers info")
        .handler(|_cmd, _m, ctx| {
            let peers_info = ctx.rt.block_on(ctx.controller.get_peers_info())?;
            println!("{}", peers_info.display());

            Ok(())
        })
}

pub fn add_node<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("add-node")
        .about("Add new node")
        .arg(
            Arg::new("multiaddr")
                .help("multi addres of the new node")
                .required(true)
        )
        .handler(|_cmd, m, ctx| {
            let multiaddr = m.value_of("multiaddr").unwrap().into();
            let status = ctx.rt.block_on(ctx.controller.add_node(multiaddr))?;
            // https://github.com/cita-cloud/status_code
            if status == 0 {
                println!("ok");
            } else {
                // I have no idea about how to explain those status codes.
                println!("status code: {}", status);
            }

            Ok(())
        })
}

pub fn rpc_cmd<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("rpc")
        .about("rpc commands")
        .setting(AppSettings::SubcommandRequired)
        .subcommands([
            get_system_config(),
            get_block_number(),
            get_block(),
            get_block_hash(),
            get_tx(),
            get_tx_index(),
            get_tx_block_number(),
            get_peer_count(),
            get_peers_info(),
            add_node(),
        ])
}
