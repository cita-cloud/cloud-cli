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

use anyhow::bail;
use clap::Arg;
use std::time::Duration;

use crate::{
    cmd::Command,
    core::{context::Context, controller::ControllerBehaviour},
    utils::{parse_position, Position},
};

pub fn watch_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("watch")
        .about("Watch blocks")
        .arg(
            Arg::new("begin")
                .help("the block height starts from. You can use +/- prefix to seek from current height")
                .short('b')
                .long("begin")
                .allow_hyphen_values(true)
                .value_parser(parse_position),
        )
        .arg(
            Arg::new("end")
                .help("the block height ends at. You can use +/- prefix to seek from current height")
                .short('e')
                .long("end")
                .allow_hyphen_values(true)
                .value_parser(parse_position),
        )
        .arg(
            Arg::new("until-finalized-txs")
                .help("stop watching when finalized txs reach the given limit")
                .short('t')
                .long("until")
                .value_parser(str::parse::<u64>),
        )
        .arg(
            Arg::new("until-empty")
                .help("stop watching when the number of consecutive empty blocks reach the given limit")
                .short('u')
                .long("until-empty")
                .value_parser(str::parse::<u64>),
        )
        .handler(|_cmd, m, ctx| {
            let mut finalized_txs = 0;
            let mut total_secs = 0;

            let watch_result = ctx.rt.block_on_without_timeout(async {
                let current_height = ctx.controller.get_block_number(false).await?;

                let begin = m
                    .get_one::<Position>("begin")
                    .map(|s| s.with_current(current_height))
                    .unwrap_or(current_height);
                let end = m
                    .get_one::<Position>("end")
                    .map(|s| s.with_current(current_height))
                    .unwrap_or(u64::MAX);

                let until_finalized_txs = m.get_one::<u64>("until-finalized-txs").copied();

                let until_empty = m.get_one::<u64>("until-empty").copied();

                let mut h = begin;

                let mut check_interval = tokio::time::interval(Duration::from_millis(500));
                let mut retry_interval = tokio::time::interval(Duration::from_secs(3));
                let mut begin_time = None;
                let mut empty_block_num = 0;

            'outter:
                while h <= end {
                    check_interval.tick().await;
                    let current_height = match ctx.controller.get_block_number(false).await {
                        Ok(current) => current,
                        Err(e) => {
                            println!("failed to get current height: `{e}`");
                            retry_interval.tick().await;
                            continue;
                        }
                    };

                    while h <= std::cmp::min(current_height, end) {
                        let block = match ctx.controller.get_compact_block_by_number(h).await {
                            Ok(block) => block,
                            Err(e) => {
                                println!("failed to get block `{h}`: `{e}`");
                                retry_interval.tick().await;
                                continue;
                            },
                        };
                        match (block.header, block.body) {
                            (Some(header), Some(body)) => {
                                let height = header.height;
                                let elapsed_secs = {
                                    let t = std::time::UNIX_EPOCH + Duration::from_millis(header.timestamp);
                                    if begin_time.is_none() {
                                        begin_time.replace(t);
                                    }
                                    t.duration_since(begin_time.unwrap()).unwrap().as_secs()
                                };
                                total_secs = elapsed_secs;

                                let cnt = body.tx_hashes.len() as u64;
                                if cnt > 0 {
                                    finalized_txs += cnt;
                                    empty_block_num = 0;
                                } else {
                                    empty_block_num += 1;
                                }

                                if elapsed_secs < 3600 {
                                    println!(
                                        "{:0>2}:{:0>2} block `{}` contains `{}` txs, finalized: `{}`",
                                        elapsed_secs / 60,
                                        elapsed_secs % 60,
                                        height,
                                        cnt,
                                        finalized_txs
                                    );
                                } else {
                                    println!(
                                        "{:0>2}:{:0>2}:{:0>2} block `{}` contains `{}` txs, finalized: `{}`",
                                        elapsed_secs / 3600,
                                        (elapsed_secs % 3600) / 60,
                                        elapsed_secs % 60,
                                        height,
                                        cnt,
                                        finalized_txs
                                    );
                                }

                                if let Some(until_finalized_txs) = until_finalized_txs {
                                    if finalized_txs >= until_finalized_txs {
                                        break 'outter;
                                    }
                                }
                                if let Some(until_empty) = until_empty {
                                    if empty_block_num >= until_empty {
                                        break 'outter;
                                    }
                                }
                            }
                            _ => {
                                bail!("invalid block, missing block header or body");
                            }
                        };

                        h += 1;
                    }
                }

                anyhow::Ok(())
            });

            println!();
            if total_secs > 0 {
                println!(
                    "`{finalized_txs}` txs finalized in `{total_secs}` secs, `{:.2}` tx/s",
                    finalized_txs as f64 / total_secs as f64
                );
            } else {
                println!("`{finalized_txs}` txs finalized");
            }
            watch_result??;

            Ok(())
        })
}

#[cfg(test)]
mod tests {

    use cita_cloud_proto::blockchain::CompactBlock;

    use crate::cmd::cldi_cmd;
    use crate::core::mock::context;

    #[test]
    #[should_panic]
    fn test_watch_subcmds() {
        let cldi_cmd = cldi_cmd();

        let (mut ctx, _temp_dir) = context();

        ctx.controller
            .expect_get_block_number()
            .returning(|_| Ok(100u64));

        ctx.controller
            .expect_get_compact_block_by_number()
            .returning(|_| Ok(CompactBlock::default()));

        cldi_cmd
            .exec_from(
                ["cldi", "watch", "-b", "1", "-e", "2", "-t", "1", "-u", "1"],
                &mut ctx,
            )
            .unwrap();
    }
}
