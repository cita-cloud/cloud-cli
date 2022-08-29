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

use clap::{crate_authors, crate_version, AppSettings, Arg, ColorChoice};
use std::str::FromStr;
use tonic::transport::Endpoint;

use crate::{
    cmd::{account, admin, bench, context, ethabi, evm, rpc, watch, Command},
    config::{ConsensusType, ContextSetting, CryptoType},
    core::{
        client::GrpcClientBehaviour, context::Context, controller::ControllerBehaviour,
        evm::EvmBehaviour, executor::ExecutorBehaviour,
    },
};

pub fn get_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
    Ev: EvmBehaviour,
{
    Command::new("get")
        .about("Get data from chain")
        .subcommand_required_else_help(true)
        .subcommands([
            evm::get_contract_abi().name("abi"),
            evm::get_balance().name("balance").alias("ba"),
            rpc::get_block().name("block").alias("b"),
            evm::get_code().name("code"),
            rpc::get_tx().name("tx"),
            rpc::get_peer_count().name("peer-count").alias("pc"),
            rpc::get_peers_info().name("peers-info").alias("pi"),
            evm::get_account_nonce().name("nonce"),
            evm::get_receipt().name("receipt").alias("r"),
            rpc::get_version().name("version").alias("ver"),
            rpc::get_system_config().name("system-config").alias("sc"),
            rpc::get_block_hash().name("block-hash").alias("bh"),
            rpc::get_block_number().name("block-number").alias("bn"),
        ])
}

pub fn cldi_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
    Ex: ExecutorBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
    Ev: EvmBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
{
    Command::<Context<Co, Ex, Ev>>::new("cldi")
        .about("The command line interface to interact with CITA-Cloud")
        .author(crate_authors!())
        .version(crate_version!())
        .color(ColorChoice::Auto)
        .global_setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::new("context")
                .help("context setting")
                .short('c')
                .long("context")
                .takes_value(true),
        )
        .arg(
            Arg::new("controller-addr")
                .help("controller address")
                .short('r')
                .takes_value(true)
                .validator(|s| Endpoint::from_shared(s.to_string()).map(|_| ())),
        )
        .arg(
            Arg::new("executor-addr")
                .help("executor address")
                .short('e')
                .takes_value(true)
                .validator(|s| Endpoint::from_shared(s.to_string()).map(|_| ())),
        )
        .arg(
            Arg::new("account-name")
                .help("account name")
                .short('u')
                .takes_value(true),
        )
        .arg(
            Arg::new("password")
                .help("password to unlock the account")
                .short('p')
                .takes_value(true),
        )
        .arg(
            Arg::new("crypto-type")
                .help("The crypto type of the target chain")
                .long("crypto")
                .possible_values(["SM", "ETH"])
                .ignore_case(true)
                .validator(CryptoType::from_str),
        )
        .arg(
            Arg::new("consensus-type")
                .help("The consensus type of the target chain")
                .long("consensus")
                .possible_values(["BFT", "OVERLORD", "RAFT"])
                .ignore_case(true)
                .validator(ConsensusType::from_str),
        )
        .handler(|cmd, m, ctx| {
            // If a subcommand is present, context modifiers(e.g. -r) will construct a tmp context for that subcommand.
            // Otherwise modify the current context.
            let mut previous_setting: Option<ContextSetting> = None;
            let mut current_setting = ctx.current_setting.clone();

            let is_tmp_ctx = m.subcommand().is_some()
                && (m.is_present("context")
                    || m.is_present("controller-addr")
                    || m.is_present("executor-addr")
                    || m.is_present("account-name")
                    || m.is_present("password")
                    || m.is_present("crypto-type")
                    || m.is_present("consensus-type"));
            if is_tmp_ctx {
                previous_setting.replace(current_setting.clone());
            }
            // (account_name, password) for restoring previous account lock status if it's in tmp context.
            let mut relock_info: Option<(String, String)> = None;

            if let Some(setting_name) = m.value_of("context") {
                current_setting = ctx.get_context_setting(setting_name)?.clone();
            }
            if let Some(controller_addr) = m.value_of("controller-addr") {
                current_setting.controller_addr = controller_addr.into();
            }
            if let Some(executor_addr) = m.value_of("executor-addr") {
                current_setting.executor_addr = executor_addr.into();
            }
            if let Some(account_name) = m.value_of("account-name") {
                // Check if the account exists.
                ctx.wallet.get(account_name)?;
                current_setting.account_name = account_name.into();
            }
            if let Some(pw) = m.value_of("password") {
                let account_name = m
                    .value_of("account-name")
                    .unwrap_or(&ctx.current_setting.account_name);
                let was_locked = ctx.wallet.get(account_name)?.is_locked();
                ctx.wallet.unlock(account_name, pw.as_bytes())?;
                if is_tmp_ctx && was_locked {
                    relock_info.replace((account_name.into(), pw.into()));
                }
            }
            if let Some(crypto_type) = m.value_of("crypto-type") {
                current_setting.crypto_type = crypto_type.parse().unwrap();
            }
            if let Some(consensus_type) = m.value_of("consensus-type") {
                current_setting.consensus_type = consensus_type.parse().unwrap();
            }

            ctx.switch_context(current_setting)?;
            let ret = cmd.dispatch_subcmd(m, ctx);

            // Restore previous lock status and context setting if it's in tmp context.
            if let Some((account_name, pw)) = relock_info {
                ctx.wallet.lock_in_memory(&account_name, pw.as_bytes())?;
            }
            if let Some(previous) = previous_setting {
                ctx.switch_context(previous)
                    .expect("cannot restore previous context");
            }

            ret
        })
        .subcommands([
            self::get_cmd().aliases(&["ge", "g"]),
            rpc::send_tx().name("send"),
            rpc::call_executor().name("call"),
            rpc::create_contract().name("create"),
            context::context_cmd(),
            account::account_cmd().alias("a"),
            admin::admin_cmd(),
            rpc::rpc_cmd(),
            ethabi::ethabi_cmd(),
            bench::bench_cmd().alias("b"),
            watch::watch_cmd().alias("w"),
        ])
        .with_completions_subcmd()
}
