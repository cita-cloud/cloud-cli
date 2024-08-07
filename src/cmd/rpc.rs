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

use crate::types::clean_0x;
use crate::{
    cmd::{evm::store_abi, Command},
    config::ConsensusType,
    core::{
        context::Context,
        controller::{ControllerBehaviour, TransactionSenderBehaviour},
        cross_chain::{self, CrossChainResultCode},
        evm::EvmBehaviour,
        executor::ExecutorBehaviour,
    },
    crypto::{Address, ArrayLike, Hash},
    display::Display,
    utils::{
        get_block_height_at, parse_addr, parse_data, parse_hash, parse_position, parse_u64,
        parse_value, Position,
    },
};
use anyhow::{anyhow, Context as _};
use cita_cloud_proto::controller::{CrossChainProof, SystemConfig};
use clap::builder::ArgPredicate;
use clap::{Arg, ArgAction};
use prost::Message;
use std::fs::File;
use std::io::{Read, Write};
use tokio::try_join;

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
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("to")
                .help("the target contract address")
                .required(true)
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("data")
                .help("the data of this call request")
                .required(true)
                .value_parser(parse_data),
        )
        .arg(
            Arg::new("height")
                .help("the height of this call request")
                .required(false)
                .value_parser(parse_u64),
        )
        .handler(|_cmd, m, ctx| {
            let from = match m.get_one::<Address>("from") {
                Some(from) => from.to_owned(),
                None => *ctx.current_account()?.address(),
            };
            let to = *m.get_one::<Address>("to").unwrap();
            let data = m.get_one::<Vec<u8>>("data").unwrap().to_owned();
            let height = if let Some(height) = m.get_one::<u64>("height") {
                *height
            } else {
                0
            };

            let resp = ctx
                .rt
                .block_on(ctx.executor.call(from, to, data, height))??;
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
                .required(true)
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("data")
                .help("the data of this tx")
                .default_value("0x")
                .value_parser(parse_data),
        )
        .arg(
            Arg::new("value")
                .help("the value of this tx")
                .short('v')
                .long("value")
                .default_value("0x0")
                .value_parser(parse_value),
        )
        .arg(
            Arg::new("quota")
                .help("the quota of this tx")
                .short('q')
                .long("quota")
                .default_value("200000")
                .value_parser(str::parse::<u64>),
        )
        .arg(
            Arg::new("valid-until-block")
                .help("this tx is valid until the given block height. `+h` means `<current-height> + h`")
                .long("until")
                .default_value("+95")
                .value_parser(parse_position),
        )
        .handler(|_cmd, m, ctx| {
            ctx.rt.block_on(async {
                let to = m.get_one::<Address>("to").unwrap().to_vec();
                let data = m.get_one::<Vec<u8>>("data").unwrap().to_owned();
                let value = m.get_one::<[u8; 32]>("value").unwrap().to_vec();
                let quota = *m.get_one::<u64>("quota").unwrap();
                let valid_until_block = {
                    let pos = *m.get_one::<Position>("valid-until-block").unwrap();
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
                .required(true)
                .value_parser(parse_data),
        )
        .arg(
            Arg::new("value")
                .help("the value of this tx")
                .short('v')
                .long("value")
                .default_value("0x0")
                .value_parser(parse_value),
        )
        .arg(
            Arg::new("quota")
                .help("the quota of this tx")
                .short('q')
                .long("quota")
                .default_value("1073741824")
                .value_parser(str::parse::<u64>),
        )
        .arg(
            Arg::new("valid-until-block")
                .help("this tx is valid until the given block height. `+h` means `<current-height> + h`")
                .long("until")
                .default_value("+95")
                .value_parser(parse_position),
        )
        .handler(|_cmd, m, ctx| {
            ctx.rt.block_on(async {
                let to = Vec::new();
                let data = m.get_one::<Vec<u8>>("data").unwrap().to_owned();
                let value = m.get_one::<[u8; 32]>("value").unwrap().to_vec();
                let quota = *m.get_one::<u64>("quota").unwrap();
                let valid_until_block = {
                    let pos = *m.get_one::<Position>("valid-until-block").unwrap();
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

pub fn get_system_config<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-system-config")
        .about("Get system config")
        .arg(
            Arg::new("height")
                .help("Get system config by height")
                .required(false)
                .value_parser(parse_u64),
        )
        .handler(|_cmd, m, ctx| {
            if m.contains_id("height") {
                let height = *m.get_one::<u64>("height").unwrap();
                let current_height = ctx.rt.block_on(ctx.controller.get_block_number(false))??;
                if height > current_height {
                    return Err(anyhow!("current_height: {}", current_height));
                } else {
                    let system_config = ctx
                        .rt
                        .block_on(ctx.controller.get_system_config_by_number(height))??;
                    println!("{}", system_config.display());
                }
            } else {
                let system_config = ctx.rt.block_on(ctx.controller.get_system_config())??;
                println!("{}", system_config.display());
            };
            Ok(())
        })
}

pub fn get_block<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-block")
        .about("Get block by block height or hash(0x)")
        .arg(Arg::new("detail").long("detail").short('d').help("with transaction details").action(ArgAction::SetTrue))
        .arg(
            Arg::new("height_or_hash")
                .help("plain decimal number or hash with `0x` prefix")
                .required(true)
                .value_parser(|s: &str| {
                    if s.starts_with("0x") {
                        parse_hash(s)?;
                    } else {
                        s.parse::<u64>().context("cannot parse block number, if you want to get block by hash, please prefix it with `0x`")?;
                    }
                    anyhow::Ok(())
                })
        )
        .handler(|_cmd, m, ctx| {
            let s = m.get_raw("height_or_hash").unwrap().next().unwrap().to_str().unwrap();
            let d = *m.get_one::<bool>("detail").unwrap();
            let height = if s.starts_with("0x") {
                let hash = parse_hash(s)?;
                ctx.rt.block_on(ctx.controller.get_height_by_hash(hash))??.block_number
            } else {
                s.parse()?
            };
            let current_height = ctx.rt.block_on(ctx.controller.get_block_number(false))??;
            if height > current_height {
                return Err(anyhow!("current_height: {}", current_height));
            } else if d {
                let full_block = ctx.rt.block_on(ctx.controller.get_block_detail_by_number(height))??;
                println!("{}", full_block.display());
            } else {
                let compact_block_with_stateroot_proof = ctx.rt.block_on(ctx.controller.get_block_by_number(height))??;
                println!("{}", compact_block_with_stateroot_proof.display());
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
                .long("for_pending")
                .action(ArgAction::SetTrue),
        )
        .handler(|_cmd, m, ctx| {
            let for_pending = *m.get_one::<bool>("for_pending").unwrap();

            let block_number = ctx
                .rt
                .block_on(ctx.controller.get_block_number(for_pending))??;
            println!("{block_number}");
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
                .required(true)
                .value_parser(str::parse::<u64>),
        )
        .handler(|_cmd, m, ctx| {
            let height = *m.get_one::<u64>("height").unwrap();
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
        .arg(Arg::new("tx_hash").required(true).value_parser(parse_hash))
        .handler(|_cmd, m, ctx| {
            let tx_hash = *m.get_one::<Hash>("tx_hash").unwrap();
            let c = &ctx.controller;

            let tx = ctx.rt.block_on(c.get_tx(tx_hash))??;
            let tx_with_index = match ctx.rt.block_on(async move {
                try_join!(c.get_tx_block_number(tx_hash), c.get_tx_index(tx_hash),)
            })? {
                Ok(info) => (tx, info.0, info.1),
                Err(_) => (tx, u64::MAX, u64::MAX),
            };

            println!("{}", tx_with_index.display());

            Ok(())
        })
}

pub fn get_node_status<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-node-status")
        .about("Get node status")
        .handler(|_cmd, _m, ctx| {
            let node_status = ctx.rt.block_on(ctx.controller.get_node_status())??;
            println!("{}", node_status.display());

            Ok(())
        })
}

pub fn get_cross_chain_proof<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-cross-chain-proof")
        .about("Get transaction cross chain proof by tx_hash")
        .arg(
            Arg::new("tx_hash")
                .help("Input the tx hash to extract crosschain proof")
                .required(true)
                .value_parser(parse_hash),
        )
        .arg(
            Arg::new("output")
                .help("Output the cross chain proof file path")
                .short('o')
                .long("output"),
        )
        .handler(|_cmd, m, ctx| {
            let tx_hash = *m.get_one::<Hash>("tx_hash").unwrap();
            let c = &ctx.controller;

            let cc_proof = ctx.rt.block_on(c.get_cross_chain_proof(tx_hash))??;

            println!("{}", cc_proof.display());
            if m.contains_id("output") {
                let file_path = m.get_one::<String>("output").unwrap();
                let ccp_bytes = {
                    let mut buf = Vec::with_capacity(cc_proof.encoded_len());
                    cc_proof.encode(&mut buf).unwrap();
                    buf
                };
                let mut f = File::options().write(true).create(true).open(format!(
                    "{}/{}-ccp",
                    file_path,
                    hex::encode(&tx_hash[..8])
                ))?;
                f.write_all(&ccp_bytes)?;
            }
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
            Arg::new("port")
                .help("the port of the new node")
                .value_parser(str::parse::<u16>)
                .required(true),
        )
        .arg(
            Arg::new("domain")
                .help("the domain name of the new node")
                .required(true),
        )
        .handler(|_cmd, m, ctx| {
            let port = *m.get_one::<u16>("port").unwrap();
            let domain = m.get_one::<String>("domain").unwrap();
            let multiaddr = format!("/dns4/localhost/tcp/{port}/tls/{domain}");

            let status = ctx.rt.block_on(ctx.controller.add_node(multiaddr))??;
            // https://github.com/cita-cloud/status_code
            if status == 0 {
                println!("Success");
            } else {
                // I have no idea how to explain those status codes.
                println!("Failed with status code: `{status}`, please check controler's log");
            }

            Ok(())
        })
}

pub fn parse_proof<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("parse-proof")
        .about("parse consensus proof")
        .arg(
            Arg::new("proof")
                .help("plain proof data with `0x` prefix")
                .required(true)
                .value_parser(parse_data),
        )
        .arg(
            Arg::new("consensus-type")
                .help(
                    "The consensus type of the proof. [default: <current-context-consensus-type>]",
                )
                .long("consensus")
                .value_parser(["OVERLORD"])
                .ignore_case(true),
        )
        .handler(|_cmd, m, ctx| {
            let consensus_type = m
                .get_one::<String>("consensus-type")
                .map(|s| s.parse::<ConsensusType>().unwrap())
                .unwrap_or(ctx.current_setting.consensus_type);
            let proof = m.get_one::<Vec<u8>>("proof").unwrap().to_owned();
            match consensus_type {
                ConsensusType::Overlord => {
                    let proof_with_validators = ctx
                        .rt
                        .block_on(ctx.controller.parse_overlord_proof(proof))??;
                    println!("{}", proof_with_validators.display());
                }
                _ => return Err(anyhow!("impossible consensus type")),
            }

            Ok(())
        })
}

pub fn estimate_quota<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("estimate-quota")
        .about("estimate quota a specified transaction will cost")
        .arg(
            Arg::new("data")
                .help("the data of this call request")
                .required(true)
                .value_parser(parse_data),
        )
        .arg(
            Arg::new("from")
                .help("default to use current account address")
                .short('f')
                .long("from")
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("to")
                .help("the target contract address, default means create contract")
                .short('t')
                .long("to")
                .value_parser(parse_addr),
        )
        .handler(|_cmd, m, ctx| {
            let from = match m.get_one::<Address>("from") {
                Some(from) => from.to_vec(),
                None => ctx.current_account()?.address().to_vec(),
            };
            let to = match m.get_one::<Address>("to") {
                Some(to) => to.to_vec(),
                None => [0; 20].to_vec(),
            };
            let data = m.get_one::<Vec<u8>>("data").unwrap().to_owned();

            let byte_quota = ctx.rt.block_on(ctx.evm.estimate_quota(from, to, data))??;
            println!("{}", byte_quota.display());
            Ok(())
        })
}

pub fn rpc_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + Send + Sync,
    Ex: ExecutorBehaviour,
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("rpc")
        .about("Other RPC commands")
        .subcommand_required_else_help(true)
        .subcommands([add_node(), store_abi(), parse_proof(), estimate_quota()])
}

pub fn verify_cross_chain_proof<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("verify-cross-chain-proof")
        .about("Verify cross chain proof")
        .arg(
            Arg::new("file-path")
                .long("file")
                .short('f')
                .conflicts_with("proof-data")
                .required_unless_present("proof-data")
                .help("file path of cross chain proof file"),
        )
        .arg(
            Arg::new("proof-data")
                .long("data")
                .short('d')
                .help("input proof data on hex form with 0x prefix"),
        )
        .arg(
            Arg::new("validators")
                .long("validators")
                .short('v')
                .help("input validators' address on hex form with 0x prefix, split with ,"),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .requires_if(ArgPredicate::IsPresent, "chain-id")
                .help("input chain version")
                .value_parser(str::parse::<u32>),
        )
        .arg(
            Arg::new("chain-id")
                .long("chain-id")
                .requires_if(ArgPredicate::IsPresent, "version")
                .help("input chain id"),
        )
        .handler(|_cmd, m, ctx| {
            let ccp_bytes = if m.contains_id("file-path") {
                let file_path = m.get_one::<String>("file-path").unwrap();
                let mut file = File::options().read(true).open(file_path).unwrap();
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).unwrap();
                bytes
            } else {
                let proof_data = m.get_one::<String>("proof-data").unwrap();
                hex::decode(clean_0x(proof_data)).unwrap()
            };
            let cc_proof = CrossChainProof::decode(ccp_bytes.as_slice()).unwrap();

            let mut online = false;
            let sys_conf = if m.contains_id("version") {
                let version = *m.get_one::<u32>("version").unwrap();
                let chain_id = clean_0x(m.get_one::<String>("chain-id").unwrap());
                let sys_conf = SystemConfig {
                    version,
                    chain_id: hex::decode(chain_id).unwrap(),
                    ..Default::default()
                };
                Some(sys_conf)
            } else if let Some(ref receipt_proof) = cc_proof.receipt_proof {
                if let Some(ref roots_info) = receipt_proof.roots_info {
                    let height = roots_info.height;
                    if let Ok(Ok(sys_conf)) = ctx
                        .rt
                        .block_on(ctx.controller.get_system_config_by_number(height))
                    {
                        online = true;
                        Some(sys_conf)
                    } else {
                        None
                    }
                } else {
                    println!("{}", CrossChainResultCode::NoneRootsInfo.display());
                    return Ok(());
                }
            } else {
                println!("{}", CrossChainResultCode::NoneReceiptProof.display());
                return Ok(());
            };

            let validators = if m.contains_id("validators") {
                let validators = m.get_one::<String>("validators").unwrap();
                let validator_vec = validators.split(',').map(|s| s.to_owned()).collect();
                Some(validator_vec)
            } else if online {
                Some(
                    sys_conf
                        .clone()
                        .unwrap()
                        .validators
                        .iter()
                        .map(hex::encode)
                        .collect(),
                )
            } else {
                None
            };

            match cross_chain::verify_cross_chain_proof(cc_proof, validators, sys_conf) {
                Ok(_) => {
                    let code = CrossChainResultCode::Success;
                    println!("{}", code.display());
                }
                Err(code) => println!("{}", code.display()),
            }

            Ok(())
        })
}

pub fn verify_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + Send + Sync,
    Ex: ExecutorBehaviour,
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("verify")
        .about("Verify commands")
        .subcommand_required_else_help(true)
        .subcommands([verify_cross_chain_proof()
            .name("cross-chain-proof")
            .alias("ccp")])
}

#[cfg(test)]
mod tests {
    use cita_cloud_proto::blockchain::{Block, RawTransaction};
    use cita_cloud_proto::controller::{BlockNumber, SystemConfig};
    use cita_cloud_proto::executor::CallResponse;

    use super::*;
    use crate::cmd::cldi_cmd;
    use crate::core::controller::ProofWithValidators;
    use crate::core::mock::context;

    #[test]
    fn test_rpc_subcmds() {
        let cldi_cmd = cldi_cmd();
        let (mut ctx, _temp_dir) = context();

        ctx.executor
            .expect_call()
            .returning(|_, _, _, _| Ok(CallResponse::default()));
        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "call",
                    "-f",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "0xabcd",
                    "100",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.controller
            .expect_get_system_config()
            .returning(|| Ok(SystemConfig::default()));

        ctx.controller
            .expect_get_block_number()
            .returning(|_| Ok(100u64));

        ctx.controller
            .expect_send_raw()
            .returning(|_utxo| Ok(Hash::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "send",
                    "-v",
                    "0x0",
                    "-q",
                    "200000",
                    "--until",
                    "+80",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "0xabcd",
                ],
                &mut ctx,
            )
            .unwrap();

        cldi_cmd
            .exec_from(
                [
                    "cldi", "create", "-v", "0x0", "-q", "200000", "--until", "+80", "0xabcd",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.controller
            .expect_get_system_config_by_number()
            .returning(|_| Ok(SystemConfig::default()));

        cldi_cmd
            .exec_from(["cldi", "get", "system-config", "100"], &mut ctx)
            .unwrap();

        ctx.controller
            .expect_get_block_detail_by_number()
            .returning(|_| Ok(Block::default()));

        cldi_cmd
            .exec_from(["cldi", "get", "block", "-d", "100"], &mut ctx)
            .unwrap();

        ctx.controller
            .expect_get_height_by_hash()
            .returning(|_| Ok(BlockNumber::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "get",
                    "block",
                    "-d",
                    "0x74ac6372ab461de6817d7146a9b8ad17c35525b13a37f4bb0da325fbfd999f3a",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.controller
            .expect_get_block_hash()
            .returning(|_| Ok(Hash::default()));

        cldi_cmd
            .exec_from(["cldi", "get", "block-hash", "100"], &mut ctx)
            .unwrap();

        ctx.controller
            .expect_get_tx()
            .returning(|_| Ok(RawTransaction::default()));

        ctx.controller
            .expect_get_tx_block_number()
            .returning(|_| Ok(100u64));

        ctx.controller.expect_get_tx_index().returning(|_| Ok(0u64));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "get",
                    "tx",
                    "0x74ac6372ab461de6817d7146a9b8ad17c35525b13a37f4bb0da325fbfd999f3a",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.controller.expect_add_node().returning(|_| Ok(0u32));

        cldi_cmd
            .exec_from(["cldi", "rpc", "add-node", "4000", "node3"], &mut ctx)
            .unwrap();

        ctx.controller
            .expect_parse_overlord_proof()
            .returning(|_| Ok(ProofWithValidators::default()));

        cldi_cmd
            .exec_from(["cldi", "rpc", "parse-proof", "0xf8880280a0851d013b28aa0dbabd0558593a808accd9c6bcf8b41194d9c341fe0001cf6112f863b860948bec9fa600d1a4dd20d60ea679c9e45af2ab22d915ff251d47816b126a1450ea1270cad8a16df198a8870937efa79e09c60ba70acf12ebf0a08888a5edfeeeb34ca98ba30f869a99fa3302ba7d61ea2c966c8e35e32b88a447d2182e22dbcf70"], &mut ctx)
            .unwrap();
    }
}
