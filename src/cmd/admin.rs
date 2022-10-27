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

use clap::Arg;

use crate::{
    cmd::Command,
    core::{admin::AdminBehaviour, context::Context},
    crypto::Address,
    display::Display,
    utils::{parse_addr, parse_validator_addr},
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
                .value_parser(parse_addr),
        )
        .handler(|_cmd, m, ctx| {
            let new_admin_addr = m.get_one::<Address>("admin").unwrap();
            let old_admin_signer = ctx.current_account()?;
            let tx_hash = ctx.rt.block_on(async {
                ctx.controller
                    .update_admin(old_admin_signer, *new_admin_addr)
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
                .num_args(1..)
                .value_parser(parse_validator_addr)
        )
        .handler(|_cmd, m, ctx| {
            let validators = m
                .get_many::<Vec<u8>>("validators")
                .unwrap()
                .map(|v| v.to_owned())
                .collect::<Vec<Vec<u8>>>();

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
                .value_parser(str::parse::<u32>),
        )
        .handler(|_cmd, m, ctx| {
            let block_interval = *m.get_one::<u32>("block_interval").unwrap();
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
        .about("Send emergency brake cmd to chain")
        .arg(
            Arg::new("switch")
                .help("turn on/off")
                .required(true)
                .value_parser(["on", "off"]),
        )
        .handler(|_cmd, m, ctx| {
            let switch = m.get_one::<String>("switch").unwrap() == "on";
            let admin_signer = ctx.current_account()?;
            let tx_hash = ctx
                .rt
                .block_on(async { ctx.controller.emergency_brake(admin_signer, switch).await })??;
            println!("{}", tx_hash.display());
            Ok(())
        })
}

pub fn set_quota_limit<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: AdminBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("set-quota-limit")
        .about("Set quota limit")
        .arg(
            Arg::new("quota_limit")
                .help("new quota limit")
                .required(true)
                .value_parser(str::parse::<u64>),
        )
        .handler(|_cmd, m, ctx| {
            let quota_limit = *m.get_one::<u64>("quota_limit").unwrap();
            let admin_signer = ctx.current_account()?;
            let tx_hash = ctx.rt.block_on(async {
                ctx.controller
                    .set_quota_limit(admin_signer, quota_limit)
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
            set_quota_limit(),
        ])
}

#[cfg(test)]
mod tests {

    use crate::cmd::cldi_cmd;
    use crate::core::mock::context;
    use crate::crypto::Hash;
    use cita_cloud_proto::controller::SystemConfig;

    #[test]
    fn test_admin_subcmds() {
        let cldi_cmd = cldi_cmd();

        let (mut ctx, _temp_dir) = context();
        ctx.controller
            .expect_get_system_config()
            .returning(|| Ok(SystemConfig::default()));
        ctx.controller
            .expect_send_raw()
            .returning(|_utxo| Ok(Hash::default()));

        cldi_cmd
            .exec_from(["cldi", "admin", "emergency-brake", "on"], &mut ctx)
            .unwrap();

        cldi_cmd
            .exec_from(["cldi", "admin", "emergency-brake", "off"], &mut ctx)
            .unwrap();

        cldi_cmd
            .exec_from(["cldi", "admin", "set-block-interval", "6"], &mut ctx)
            .unwrap();

        cldi_cmd
            .exec_from(["cldi", "admin", "set-quota-limit", "10000000"], &mut ctx)
            .unwrap();

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "admin",
                    "update-admin",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                ],
                &mut ctx,
            )
            .unwrap();

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "admin",
                    "update-validators",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "0x51219f84f5ff1cc54f9b52867fbbfb6d3196ff25",
                ],
                &mut ctx,
            )
            .unwrap();
    }
}
