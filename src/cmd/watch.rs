use anyhow::bail;
use clap::Arg;
use std::time::Duration;

use crate::{
    cmd::Command,
    core::{context::Context, controller::ControllerBehaviour},
};

pub fn watch_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("watch")
        .about("watch")
        .arg(
            Arg::new("begin")
                .help("the block height starts from")
                .short('b')
                .long("begin")
                .takes_value(true)
                .validator(str::parse::<u64>),
        )
        .arg(
            Arg::new("end")
                .help("the block height ends at")
                .short('e')
                .long("end")
                .takes_value(true)
                .validator(str::parse::<u64>),
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
            let mut total_secs = 0u64;

            let watch_result = ctx.rt.block_on(async {
                let current_height = ctx.controller.get_block_number(false).await?;

                let begin = m
                    .value_of("begin")
                    .map(|s| s.parse::<u64>().unwrap())
                    .unwrap_or(current_height);
                let end = m
                    .value_of("end")
                    .map(|s| s.parse::<u64>().unwrap())
                    .unwrap_or(u64::MAX);

                let until_finalized_txs = m
                    .value_of("until_finalized_txs")
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

                    while h <= current_height {
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
                                let secs = {
                                    let t = std::time::UNIX_EPOCH + Duration::from_millis(header.timestamp);
                                    if begin_time.is_none() {
                                        begin_time.replace(t);
                                    }
                                    t.duration_since(begin_time.unwrap()).unwrap().as_secs()
                                };
                                total_secs += secs;

                                let cnt = body.tx_hashes.len() as u64;
                                finalized_txs += cnt;

                                if secs < 3600 {
                                    println!(
                                        "{:0>2}:{:0>2} block `{}` contains `{}` txs, finalized: `{}`",
                                        secs / 60,
                                        secs % 60,
                                        height,
                                        cnt,
                                        finalized_txs
                                    );
                                } else {
                                    println!(
                                        "{:0>2}:{:0>2}:{:0>2} block `{}` contains `{}` txs, finalized: `{}`",
                                        secs / 3600,
                                        secs / 60,
                                        secs % 60,
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

            println!("{finalized_txs} txs finalized in {total_secs}, {:.2} tx/s", finalized_txs as f64 / total_secs as f64);
            watch_result??;

            Ok(())
        })
}
