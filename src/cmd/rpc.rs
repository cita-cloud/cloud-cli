use clap::App;
use clap::Arg;
use clap::AppSettings;

use crate::utils::parse_value;
use crate::utils::{parse_addr, parse_hash, parse_data, hex};

// use super::*;
use super::Command;
use crate::core::context::Context;
use prost::Message;
use crate::core::controller::ControllerBehaviour;
use crate::core::controller::TransactionSenderBehaviour;
use crate::core::executor::ExecutorBehaviour;

use tokio::try_join;

use anyhow::Result;
use anyhow::Context as _;
use crate::display::Display;
use crate::crypto::ArrayLike;

// TODO: get version

pub fn call<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ex: ExecutorBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("call")
        .about("Executor call")
        .arg(
            Arg::new("from")
                .short('f')
                .long("from")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(
            Arg::new("to")
                .short('t')
                .long("to")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(
            Arg::new("data")
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        )
        .handler(|_cmd, m, ctx| {
            let from = parse_addr(m.value_of("from").unwrap())?;
            let to = parse_addr(m.value_of("to").unwrap())?;
            let data = parse_data(m.value_of("data").unwrap_or_default())?;

            let resp = ctx.rt.block_on(ctx.executor.call(from, to, data))??;
            println!("{}", resp.display());
            Ok(())
        })
}


pub fn send<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: TransactionSenderBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("send")
        .about("Send transaction")
        .arg(
            Arg::new("to")
                .help("the address to send")
                .short('t')
                .long("to")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
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
            let to = parse_addr(m.value_of("to").unwrap())?;
            let value = parse_value(m.value_of("value").unwrap_or_default())?.to_vec();
            let data = parse_data(m.value_of("data").unwrap_or_default())?;

            let signer = ctx.current_account()?;
            ctx.rt.block_on(async {
                let tx_hash = ctx.controller.send_tx(signer, to, value, data).await?;
                println!("{}", hex(tx_hash.as_slice()));

                anyhow::Ok(())
            })??;
            Ok(())
        })
}

pub fn get_version<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-version")
        .about("Get version")
        .handler(|_cmd, _m, ctx| {
            let version = ctx.rt.block_on(ctx.controller.get_version())??;
            println!("{}", version);
            Ok(())
        })
}

pub fn get_system_config<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-system-config")
        .about("Get system config")
        .handler(|_cmd, _m, ctx| {
            let system_config = ctx.rt.block_on(ctx.controller.get_system_config())??;
            println!("{}", system_config.display());
            Ok(())
        })
}

pub fn get_block<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-block")
        .about("Get block by block height or hash(0x)")
        .arg(
            Arg::new("height_or_hash")
                .help("plain decimal number or hash with `0x` prefix")
                .required(true)
                .takes_value(true)
                .validator(|s| {
                    if s.starts_with("0x") {
                        parse_hash(s)?;
                    } else {
                        s.parse::<u64>().context("cannot parse block number, if you want to get block by hash, please prefix it with `0x`")?;
                    }
                    anyhow::Ok(())
                })
        )
        .handler(|_cmd, m, ctx| {
            let s = m.value_of("height_or_hash").unwrap();
            let block = if s.starts_with("0x") {
                let hash = parse_hash(s)?;
                ctx.rt.block_on(ctx.controller.get_block_by_hash(hash))??
            } else {
                let height = s.parse()?;
                ctx.rt.block_on(ctx.controller.get_block_by_number(height))??
            };

            println!("{}", block.display());
            Ok(())
        })
}

pub fn get_block_number<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-block-number")
        .about("Get block number")
        .arg(
            Arg::new("for_pending")
                .help("if set, get block number of the pending block")
                .short('p')
                .long("for_pending"),
        )
        .handler(|_cmd, m, ctx| {
            let for_pending = m.is_present("for_pending");

            let block_number = ctx.rt.block_on(ctx.controller.get_block_number(for_pending))??;
            println!("{}", block_number);
            Ok(())
        })
}

pub fn get_block_hash<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-block-hash")
        .about("Get block hash by block height")
        .arg(
            Arg::new("height")
                .help("the block height")
                .takes_value(true)
                .validator(str::parse::<u64>),
        )
        .handler(|_cmd, m, ctx| {
            let height = m.value_of("height").unwrap().parse()?;
            let hash = ctx.rt.block_on(ctx.controller.get_block_hash(height))??;
            println!("{}", hex(hash.as_slice()));

            Ok(())
        })
}

pub fn get_tx<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-tx")
        .about("Get transaction by hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_hash))
        .handler(|_cmd, m, ctx| {
            let s = m.value_of("tx_hash").unwrap();
            let tx_hash = parse_hash(s)?;
            let c = &ctx.controller;
            let tx_with_index = ctx.rt.block_on(async move {
                try_join!(
                    c.get_tx(tx_hash),
                    c.get_tx_block_number(tx_hash.clone()),
                    c.get_tx_index(tx_hash),
                )
            })??;

            println!("{}", tx_with_index.display());

            Ok(())
        })
}

// pub fn get_tx_index<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Context<Co, Ex, Ev>>
// where
//     C: Crypto,
//     Co: ControllerBehaviour,
// {
//     Command::<Context<Co, Ex, Ev>>::new("get-tx-index")
//         .about("Get transaction's index by tx_hash")
//         .arg(Arg::new("tx_hash").required(true).validator(parse_hash))
//         .handler(|_cmd, m, ctx| {
//             let s = m.value_of("tx_hash").unwrap();
//             let tx_hash = parse_hash(s)?;
//             let tx_index = ctx.rt.block_on(ctx.controller.get_tx_index(tx_hash))??;
//             println!("{}", tx_index);

//             Ok(())
//         })
// }

// pub fn get_tx_block_number<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Context<Co, Ex, Ev>>
// where
//     C: Crypto,
//     Co: ControllerBehaviour,
// {
//     Command::<Context<Co, Ex, Ev>>::new("get-tx-block-height")
//         .about("Get transaction's block height by tx_hash")
//         .arg(Arg::new("tx_hash").required(true).validator(parse_hash))
//         .handler(|_cmd, m, ctx| {
//             let s = m.value_of("tx_hash").unwrap();
//             let tx_hash = parse_hash(s)?;
//             let height = ctx.rt.block_on(ctx.controller.get_tx_block_number(tx_hash))??;
//             println!("{}", height);

//             Ok(())
//         })
// }

pub fn get_peer_count<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("peer-count")
        .about("Get peer count")
        .handler(|_cmd, _m, ctx| {
            let peer_count = ctx.rt.block_on(ctx.controller.get_peer_count())??;
            println!("{}", peer_count);

            Ok(())
        })
}

pub fn get_peers_info<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("peers-info")
        .about("Get peers info")
        .handler(|_cmd, _m, ctx| {
            let peers_info = ctx.rt.block_on(ctx.controller.get_peers_info())??;
            println!("{}", peers_info.display());

            Ok(())
        })
}

pub fn add_node<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("add-node")
        .about("Add new node")
        .arg(
            Arg::new("multiaddr")
                .help("multi addres of the new node")
                .required(true)
        )
        .handler(|_cmd, m, ctx| {
            let multiaddr = m.value_of("multiaddr").unwrap().into();
            let status = ctx.rt.block_on(ctx.controller.add_node(multiaddr))??;
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

pub fn rpc_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("rpc")
        .about("RPC commands")
        .arg_required_else_help(true)
        .subcommands([
            get_version(),
            get_system_config(),
            get_block_number(),
            get_block(),
            get_block_hash(),
            get_tx(),
            get_peer_count(),
            get_peers_info(),
            add_node(),
        ])
}
