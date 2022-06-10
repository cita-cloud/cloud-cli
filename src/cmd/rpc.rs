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

use anyhow::Context as _;
use clap::Arg;
use std::net::IpAddr;
use tokio::try_join;

use crate::{
    cmd::{evm::store_abi, Command},
    core::{
        context::Context,
        controller::{ControllerBehaviour, TransactionSenderBehaviour},
        executor::ExecutorBehaviour,
    },
    crypto::ArrayLike,
    display::Display,
    utils::{get_block_height_at, parse_addr, parse_data, parse_hash, parse_position, parse_value},
};

pub fn call_executor<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ex: ExecutorBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("call-executor")
        .about("Call executor")
        .arg(
            Arg::new("from")
                .help("default to use current account address")
                .short('f')
                .long("from")
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(
            Arg::new("to")
                .help("the target contract address")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(
            Arg::new("data")
                .help("the data of this call request")
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        )
        .handler(|_cmd, m, ctx| {
            let from = match m.value_of("from") {
                Some(from) => parse_addr(from).unwrap(),
                None => *ctx.current_account()?.address(),
            };
            let to = parse_addr(m.value_of("to").unwrap())?;
            let data = parse_data(m.value_of("data").unwrap())?;

            let resp = ctx.rt.block_on(ctx.executor.call(from, to, data))??;
            println!("{}", resp.display());
            Ok(())
        })
}

pub fn send_tx<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + Send + Sync,
{
    Command::<Context<Co, Ex, Ev>>::new("send-tx")
        .about("Send transaction")
        .arg(
            Arg::new("to")
                .help("the target address of this tx")
                .takes_value(true)
                .required(true)
                .validator(parse_addr),
        )
        .arg(
            Arg::new("data")
                .help("the data of this tx")
                .takes_value(true)
                .default_value("0x")
                .validator(parse_data),
        )
        .arg(
            Arg::new("value")
                .help("the value of this tx")
                .short('v')
                .long("value")
                .takes_value(true)
                .default_value("0x0")
                .validator(parse_value),
        )
        .arg(
            Arg::new("quota")
                .help("the quota of this tx")
                .short('q')
                .long("quota")
                .takes_value(true)
                .default_value("1073741824")
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
            ctx.rt.block_on(async {
                let to = parse_addr(m.value_of("to").unwrap())?.to_vec();
                let data = parse_data(m.value_of("data").unwrap())?;
                let value = parse_value(m.value_of("value").unwrap())?.to_vec();
                let quota = m.value_of("quota").unwrap().parse::<u64>()?;
                let valid_until_block = {
                    let pos = parse_position(m.value_of("valid-until-block").unwrap())?;
                    get_block_height_at(&ctx.controller, pos).await?
                };

                let signer = ctx.current_account()?;
                let tx_hash = ctx
                    .controller
                    .send_tx(signer, to, data, value, quota, valid_until_block)
                    .await?;
                println!("{}", tx_hash.display());

                anyhow::Ok(())
            })??;
            Ok(())
        })
}

pub fn create_contract<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + Send + Sync,
{
    Command::<Context<Co, Ex, Ev>>::new("create-contract")
        .about("create an EVM contract")
        .arg(
            Arg::new("data")
                .help("the data of this tx")
                .takes_value(true)
                .required(true)
                .validator(parse_data),
        )
        .arg(
            Arg::new("value")
                .help("the value of this tx")
                .short('v')
                .long("value")
                .takes_value(true)
                .default_value("0x0")
                .validator(parse_value),
        )
        .arg(
            Arg::new("quota")
                .help("the quota of this tx")
                .short('q')
                .long("quota")
                .takes_value(true)
                .default_value("1073741824")
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
            ctx.rt.block_on(async {
                let to = Vec::new();
                let data = parse_data(m.value_of("data").unwrap())?;
                let value = parse_value(m.value_of("value").unwrap())?.to_vec();
                let quota = m.value_of("quota").unwrap().parse::<u64>()?;
                let valid_until_block = {
                    let pos = parse_position(m.value_of("valid-until-block").unwrap())?;
                    get_block_height_at(&ctx.controller, pos).await?
                };

                let signer = ctx.current_account()?;
                let tx_hash = ctx
                    .controller
                    .send_tx(signer, to, data, value, quota, valid_until_block)
                    .await?;
                println!("{}", tx_hash.display());

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
        .arg(Arg::new("detail").long("detail").short('d').help("with transaction details"))
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
            let d = m.is_present("detail");
            if d {
                let block = if s.starts_with("0x") {
                    let hash = parse_hash(s)?;
                    ctx.rt.block_on(ctx.controller.get_block_detail_by_hash(hash))??
                } else {
                    let height = s.parse()?;
                    ctx.rt.block_on(ctx.controller.get_block_detail_by_number(height))??
                };

                println!("{}", block.display());
            }  else {
                let block = if s.starts_with("0x") {
                    let hash = parse_hash(s)?;
                    ctx.rt.block_on(ctx.controller.get_block_by_hash(hash))??
                } else {
                    let height = s.parse()?;
                    ctx.rt.block_on(ctx.controller.get_block_by_number(height))??
                };

                println!("{}", block.display());
            }
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

            let block_number = ctx
                .rt
                .block_on(ctx.controller.get_block_number(for_pending))??;
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
                .required(true)
                .validator(str::parse::<u64>),
        )
        .handler(|_cmd, m, ctx| {
            let height = m.value_of("height").unwrap().parse()?;
            let hash = ctx.rt.block_on(ctx.controller.get_block_hash(height))??;
            println!("{}", hash.display());

            Ok(())
        })
}

pub fn get_tx<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-tx")
        .about("Get transaction data by tx_hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_hash))
        .handler(|_cmd, m, ctx| {
            let s = m.value_of("tx_hash").unwrap();
            let tx_hash = parse_hash(s)?;
            let c = &ctx.controller;
            let tx_with_index = ctx.rt.block_on(async move {
                try_join!(
                    c.get_tx(tx_hash),
                    c.get_tx_block_number(tx_hash),
                    c.get_tx_index(tx_hash),
                )
            })??;

            println!("{}", tx_with_index.display());

            Ok(())
        })
}

pub fn get_peer_count<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-peer-count")
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
    Command::<Context<Co, Ex, Ev>>::new("get-peers-info")
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
        .about("call add-node rpc")
        .arg(
            Arg::new("host")
                .help("the host of the new node")
                .required(true),
        )
        .arg(
            Arg::new("port")
                .help("the port of the new node")
                .validator(str::parse::<u16>)
                .required(true),
        )
        .arg(Arg::new("tls").help("the domain name of the new node"))
        .handler(|_cmd, m, ctx| {
            let host = m.value_of("host").unwrap();
            let port = m.value_of("port").unwrap().parse::<u64>().unwrap();
            let tls = m.value_of("tls");

            let ptcl = match host.parse::<std::net::IpAddr>() {
                Ok(IpAddr::V4(_)) => "ip4",
                Ok(IpAddr::V6(_)) => "ip6",
                Err(_) => "dns4",
            };

            let multiaddr = if let Some(tls) = tls {
                format!("/{ptcl}/{host}/tcp/{port}/tls/{tls}")
            } else {
                format!("/{ptcl}/{host}/tcp/{port}")
            };

            let status = ctx.rt.block_on(ctx.controller.add_node(multiaddr))??;
            // https://github.com/cita-cloud/status_code
            if status == 0 {
                println!("Success");
            } else {
                // I have no idea how to explain those status codes.
                println!(
                    "Failed with status code: `{}`, please check controler's log",
                    status
                );
            }

            Ok(())
        })
}

pub fn rpc_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + Send + Sync,
    Ex: ExecutorBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("rpc")
        .about("Other RPC commands")
        .subcommand_required_else_help(true)
        .subcommands([add_node(), store_abi()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::cldi_cmd;
    use crate::core::mock::context;

    #[test]
    fn test_get_peer_count() {
        let cmd = get_peer_count();
        let cldi_cmd = cldi_cmd();

        let (mut ctx, _temp_dir) = context();
        ctx.controller.expect_get_peer_count().returning(|| Ok(42));

        cmd.exec_from(["get-peer-count"], &mut ctx).unwrap();
        cldi_cmd
            .exec_from(["cldi", "get", "peer-count"], &mut ctx)
            .unwrap();
    }
}
