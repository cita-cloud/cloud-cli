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
    utils::parse_position,
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
                .takes_value(true)
                .allow_hyphen_values(true)
                .validator(parse_position),
        )
        .arg(
            Arg::new("end")
                .help("the block height ends at. You can use +/- prefix to seek from current height")
                .short('e')
                .long("end")
                .takes_value(true)
                .allow_hyphen_values(true)
                .validator(parse_position),
        )
        .arg(
            Arg::new("until-finalized-txs")
                .help("stop watching when finalized txs reach the given limit")
                .short('t')
                .long("until")
                .takes_value(true)
                .validator(str::parse::<u64>),
        )
        .handler(|_cmd, m, ctx| {
            let mut finalized_txs = 0;
            let mut total_secs = 0;

            let watch_result = ctx.rt.block_on(async {
                let current_height = ctx.controller.get_block_number(false).await?;

                let begin = m
                    .value_of("begin")
                    .map(|s| parse_position(s).unwrap().with_current(current_height))
                    .unwrap_or(current_height);
                let end = m
                    .value_of("end")
                    .map(|s| parse_position(s).unwrap().with_current(current_height))
                    .unwrap_or(u64::MAX);

                let until_finalized_txs = m
                    .value_of("until-finalized-txs")
                    .map(|s| s.parse::<u64>().unwrap());

                let mut h = begin;

                let mut check_interval = tokio::time::interval(Duration::from_secs(1));
                let mut retry_interval = tokio::time::interval(Duration::from_secs(3));
                let mut begin_time = None;

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
                        let block = match ctx.controller.get_block_by_number(h).await {
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
                                finalized_txs += cnt;

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
