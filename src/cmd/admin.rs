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

use anyhow::Result;
use clap::Arg;

use crate::{
    cmd::Command,
    core::{admin::AdminBehaviour, context::Context},
    crypto::Address,
    display::Display,
    utils::parse_addr,
};

pub fn update_admin<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: AdminBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("update-admin")
        .about("Update admin of the chain")
        .arg(
            Arg::new("admin")
                .help("the address of the new admin")
                .required(true)
                .validator(parse_addr),
        )
        .handler(|_cmd, m, ctx| {
            let new_admin_addr = parse_addr(m.value_of("admin").unwrap())?;
            let old_admin_signer = ctx.current_account()?;
            let tx_hash = ctx.rt.block_on(async {
                ctx.controller
                    .update_admin(old_admin_signer, new_admin_addr)
                    .await
            })??;
            println!("{}", tx_hash.display());
            Ok(())
        })
}

pub fn update_validators<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: AdminBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("update-validators")
        .about("Update validators of the chain")
        .arg(
            Arg::new("validators")
                .help("a space-separated list of the new validator addresses, e.g. `cldi update-validators 0x12..34 0xab..cd`")
                .required(true)
                .multiple_values(true)
                .validator(parse_addr)
        )
        .handler(|_cmd, m, ctx| {
            let validators = m
                .values_of("validators")
                .unwrap()
                .map(parse_addr)
                .collect::<Result<Vec<Address>>>()?;

            let admin_signer = ctx.current_account()?;
            let tx_hash = ctx.rt.block_on(async {
                ctx.controller.update_validators(admin_signer, &validators).await
            })??;
            println!("{}", tx_hash.display());
            Ok(())
        })
}

pub fn set_block_interval<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: AdminBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("set-block-interval")
        .about("Set block interval")
        .arg(
            Arg::new("block_interval")
                .help("new block interval")
                .required(true)
                .validator(str::parse::<u32>),
        )
        .handler(|_cmd, m, ctx| {
            let block_interval = m.value_of("block_interval").unwrap().parse::<u32>()?;
            let admin_signer = ctx.current_account()?;
            let tx_hash = ctx.rt.block_on(async {
                ctx.controller
                    .set_block_interval(admin_signer, block_interval)
                    .await
            })??;
            println!("{}", tx_hash.display());
            Ok(())
        })
}

pub fn emergency_brake<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: AdminBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("emergency-brake")
        .about("Set package limit")
        .arg(
            Arg::new("switch")
                .help("turn on/off")
                .required(true)
                .possible_values(&["on", "off"]),
        )
        .handler(|_cmd, m, ctx| {
            let switch = m.value_of("switch").unwrap() == "on";
            let admin_signer = ctx.current_account()?;
            let tx_hash = ctx
                .rt
                .block_on(async { ctx.controller.emergency_brake(admin_signer, switch).await })??;
            println!("{}", tx_hash.display());
            Ok(())
        })
}

pub fn set_package_limit<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
    where
        Co: AdminBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("set-package-limit")
        .about("Set package limit")
        .arg(
            Arg::new("package_limit")
                .help("new package limit")
                .required(true)
                .validator(str::parse::<u64>),
        )
        .handler(|_cmd, m, ctx| {
            let package_limit = m.value_of("package_limit").unwrap().parse::<u64>()?;
            let admin_signer = ctx.current_account()?;
            let tx_hash = ctx.rt.block_on(async {
                ctx.controller
                    .set_package_limit(admin_signer, package_limit)
                    .await
            })??;
            println!("{}", tx_hash.display());
            Ok(())
        })
}

pub fn admin_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: AdminBehaviour,
{
    Command::new("admin")
        .about("The admin commands for managing chain")
        .subcommand_required_else_help(true)
        .subcommands([
            update_admin(),
            update_validators(),
            set_block_interval(),
            emergency_brake(),
            set_package_limit(),
        ])
}



