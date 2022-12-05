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

use anyhow::{bail, Context as _, Result};
use clap::{Arg, ArgAction};
use crossbeam::atomic::AtomicCell;
use parking_lot::Mutex;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use std::{future::Future, sync::Arc};
use tokio::{time, time::Duration};

use crate::{
    cmd::{watch, Command},
    core::executor::ExecutorBehaviour,
    core::{
        client::GrpcClientBehaviour,
        context::Context,
        controller::{ControllerBehaviour, SignerBehaviour},
    },
    crypto::Address,
    utils::{
        get_block_height_at, parse_addr, parse_data, parse_position, parse_u64, parse_value,
        Position,
    },
};
use cita_cloud_proto::blockchain::Transaction;

pub fn bench_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
    Ex: ExecutorBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
{
    Command::<Context<Co, Ex, Ev>>::new("bench")
        .about("Simple benchmarks")
        .subcommand_required_else_help(true)
        .subcommands([bench_send().name("send"), bench_call().name("call")])
}

fn bench_basic<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("bench-basic")
        .arg(
            Arg::new("concurrency")
                .help(
                    "Number of request workers to run concurrently. \
                    Workers will be distributed evenly among all the connections. \
                    [default: the same as total]",
                )
                .short('c')
                .long("concurrency")
                .value_parser(str::parse::<u64>),
        )
        .arg(
            Arg::new("connections")
                .help("Number of connections connects to server")
                .long("connections")
                .default_value("1")
                .value_parser(str::parse::<u64>),
        )
        .arg(
            Arg::new("timeout")
                .help("Timeout for each request (in seconds). 0 means no timeout")
                .long("timeout")
                .default_value("0")
                .value_parser(str::parse::<u64>),
        )
        .arg(
            Arg::new("total")
                .help("Number of tasks in the benchmark")
                .default_value("10000")
                .value_parser(str::parse::<u32>),
        )
}

pub fn bench_send<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
{
    bench_basic::<Co, Ex, Ev>()
        .name("bench-send")
        .about("Send transactions with {-c} workers over {--connections} connections")
        .arg(
            Arg::new("to")
                .help("the target address of this tx. Default to random")
                .short('t')
                .long("to")
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("data")
                .help("the data of this tx. Default to random 32 bytes")
                .short('d')
                .long("data")
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
        .arg(
            Arg::new("disable-watch")
                .help("don't watch blocks")
                .long("disable-watch")
                .action(ArgAction::SetTrue)
        )
        .handler(|_cmd, m, ctx| {
            let total = *m.get_one::<u32>("total").unwrap() as u64;
            let connections = *m.get_one::<u64>("connections").unwrap();
            let timeout = *m.get_one::<u64>("timeout").unwrap();
            let workers = *m
                .get_one::<u64>("concurrency")
                .unwrap_or(&total);

            let watch_blocks = !m.get_one::<bool>("disable-watch").unwrap();
            let watch_begin = Arc::new(AtomicCell::new(Option::<u64>::None));

            let bn = Arc::new(AtomicCell::new(Option::<u64>::None));

            ctx.rt.block_on(async {
                // Workload builder
                let mut rng = thread_rng();

                let to = match m.get_one::<Address>("to") {
                    Some(to) => to.to_owned(),
                    None => rng.gen(),
                }.to_vec();
                let data = match m.get_one::<Vec<u8>>("data") {
                    Some(to) => to.to_owned(),
                    None => rng.gen::<[u8; 32]>().to_vec(),
                };
                let value = m.get_one::<[u8; 32]>("value").unwrap().to_vec();
                let quota = *m.get_one::<u64>("quota").unwrap();
                let pos = *m.get_one::<Position>("valid-until-block").unwrap();

                let system_config = ctx.controller.get_system_config().await
                    .context("failed to fetch chain status")?;

                let signer = ctx.current_account()?;
                let block_number = bn.clone();

                let workload_builder = || {
                    let nonce = {
                        // Nonce must be different to avoid dup tx,
                        // and workload builder may be passed to other threads.
                        let mut rng = thread_rng();
                        rng.gen::<u64>().to_string()
                    };
                    let valid_until_block = block_number.load().unwrap();
                    let raw_tx = Transaction {
                        to,
                        data,
                        value,
                        nonce,
                        quota,
                        valid_until_block,
                        chain_id: system_config.chain_id.clone(),
                        version: system_config.version,
                    };
                    signer.sign_raw_tx(raw_tx)
                };

                // Connection builder
                let controller_addr = ctx.current_controller_addr();
                let connector = || async {
                    if timeout > 0 {
                        Co::connect_timeout(controller_addr, Duration::from_secs(timeout)).await
                    } else {
                        Co::connect(controller_addr).await
                    }
                };

                // Work
                let worker_fn =
                    |client: Co, raw| async move { client.send_raw(raw).await.map(|_| ()) };

                // before fns
                let before_preparing = || async {
                    println!("Preparing connections and transactions..");
                    anyhow::Ok(())
                };

                let watch_begin = watch_begin.clone();
                let controller = ctx.controller.clone();
                let before_working = || async {
                    if watch_blocks {
                        let current_block_height = controller.get_block_number(false).await
                            .context("cannot get current block height for watch begin")?;
                        watch_begin.store(Some(current_block_height));
                    }

                    println!("Sending transactions..");
                    anyhow::Ok(())
                };
                let controller = ctx.controller.clone();
                let valid_until_block = get_block_height_at(&controller, pos).await?;
                bn.store(Some(valid_until_block));
                tokio::spawn(async move {
                    let mut internal = time::interval(Duration::from_secs(3));
                    loop {
                        internal.tick().await;
                        let valid_until_block = get_block_height_at(&controller, pos).await.unwrap();
                        bn.store(Some(valid_until_block));
                    }
                });
                bench_fn_with_progbar(
                    total,
                    connections,
                    workers,
                    connector,
                    workload_builder,
                    worker_fn,
                    before_preparing,
                    before_working,
                )
                .await
            })??;

            if watch_blocks {
                let watch_begin = watch_begin.load()
                    .expect("if bench has succeeded, watch begin should always exist. This is a bug, please contact with maintainers");
                watch::watch_cmd()
                    .exec_from(
                        [
                        "watch",
                        "--begin",
                        &watch_begin.to_string(),
                        "--until",
                        &total.to_string(),
                        "--until-empty",
                        "3",
                        ],
                        ctx
                    )
            } else {
                Ok(())
            }
        })
}

pub fn bench_call<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ex: ExecutorBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
{
    bench_basic::<Co, Ex, Ev>()
        .name("bench-call")
        .about("Call executor with {-c} workers over {--connections} connections")
        .arg(
            Arg::new("from")
                .help("Default to use current account address")
                .short('f')
                .long("from")
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("to")
                .help("the target contract address to call. Default to random")
                .short('t')
                .long("to")
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("data")
                .help("the data for the call request. Default to random 32 bytes")
                .short('d')
                .long("data")
                .value_parser(parse_data),
        )
        .arg(
            Arg::new("height")
                .help("the height for the call request. Default ro current height")
                .required(false)
                .long("height")
                .value_parser(parse_u64),
        )
        .handler(|_cmd, m, ctx| {
            let total = *m.get_one::<u32>("total").unwrap() as u64;
            let connections = *m.get_one::<u64>("connections").unwrap();
            let timeout = *m.get_one::<u64>("timeout").unwrap();
            let workers = *m.get_one::<u64>("concurrency").unwrap_or(&total);

            ctx.rt.block_on(async {
                // Workload builder
                let mut rng = thread_rng();

                let from = match m.get_one::<Address>("from") {
                    Some(from) => from.to_owned(),
                    None => *ctx.current_account()?.address(),
                };
                let to = match m.get_one::<Address>("to") {
                    Some(to) => to.to_owned(),
                    None => rng.gen(),
                };
                let data = match m.get_one::<Vec<u8>>("data") {
                    Some(data) => data.to_owned(),
                    None => rng.gen::<[u8; 32]>().to_vec(),
                };
                let height = match m.get_one::<u64>("height") {
                    Some(height) => *height,
                    None => 0,
                };

                let workload_builder = || (from, to, data, height);

                // Connection builder
                let executor_addr = ctx.current_executor_addr();
                let connector = || async {
                    if timeout > 0 {
                        Ex::connect_timeout(executor_addr, Duration::from_secs(timeout)).await
                    } else {
                        Ex::connect(executor_addr).await
                    }
                };

                // Work
                let worker_fn = |client: Ex, (from, to, data, height)| async move {
                    client.call(from, to, data, height).await.map(|_| ())
                };

                // before fns
                let before_preparing = || async {
                    println!("Preparing connections and call requests..");
                    anyhow::Ok(())
                };
                let before_working = || async {
                    println!("Sending call requests..");
                    anyhow::Ok(())
                };

                bench_fn_with_progbar(
                    total,
                    connections,
                    workers,
                    connector,
                    workload_builder,
                    worker_fn,
                    before_preparing,
                    before_working,
                )
                .await?;

                anyhow::Ok(())
            })??;
            Ok(())
        })
}

#[allow(clippy::too_many_arguments)]
async fn bench_fn_with_progbar<
    Connector,
    Connection,
    ConnectionResultFut,
    Workload,
    WorkloadBuilder,
    WorkResultFut,
    Worker,
    BeforePreparing,
    BeforePreparingResultFut,
    BeforeWorking,
    BeforeWorkingResultFut,
>(
    total: u64,
    connections: u64,
    workers: u64,

    connector: Connector,
    workload_builder: WorkloadBuilder,
    worker_fn: Worker,

    before_preparing: BeforePreparing,
    before_working: BeforeWorking,
) -> Result<()>
where
    Connection: Clone + Send + Sync + 'static,
    ConnectionResultFut: Future<Output = Result<Connection>>,
    Connector: FnOnce() -> ConnectionResultFut + Clone,

    Workload: Send + 'static,
    WorkloadBuilder: FnOnce() -> Workload + Send + Sync + Clone,

    WorkResultFut: Future<Output = Result<()>> + Send,
    Worker: FnOnce(Connection, Workload) -> WorkResultFut + Clone + Send + Sync + 'static,

    BeforePreparing: FnOnce() -> BeforePreparingResultFut,
    BeforePreparingResultFut: Future<Output = Result<()>>,
    BeforeWorking: FnOnce() -> BeforeWorkingResultFut,
    BeforeWorkingResultFut: Future<Output = Result<()>>,
{
    let progbar = {
        let progbar = indicatif::ProgressBar::new(total);
        progbar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}",
                )?
                .progress_chars("=> "),
        );
        Arc::new(progbar)
    };

    let mut t = None;
    let before_working = || async {
        before_working().await?;
        // start the timer
        t.replace(std::time::Instant::now());
        anyhow::Ok(())
    };

    let progbar_cloned = Arc::clone(&progbar);
    let wrapped_worker_fn = |conn, workload| async move {
        let work_res = worker_fn(conn, workload).await;
        if work_res.is_ok() {
            progbar_cloned.inc(1)
        }
        work_res
    };

    let bench_res = bench_fn(
        total,
        connections,
        workers,
        connector,
        workload_builder,
        wrapped_worker_fn,
        before_preparing,
        before_working,
    )
    .await;

    progbar.finish();

    if let Some(t) = t {
        println!(
            "`{}` tasks finished in `{}` ms",
            total,
            t.elapsed().as_millis()
        );
    }
    let success = progbar.position();
    let failure = total - success;
    println!("`{}` success, `{}` failure", success, failure);

    if let Err(e) = bench_res {
        println!(
            "bench isn't completed successfully, the first reported error is `{:?}`",
            e
        );
        // Don't repeat the error msg.
        bail!("bench failed")
    } else {
        Ok(())
    }
}

// I think it's too complex.
#[allow(clippy::too_many_arguments)]
async fn bench_fn<
    Connector,
    Connection,
    ConnectionResultFut,
    Workload,
    WorkloadBuilder,
    WorkResultFut,
    Worker,
    BeforePreparing,
    BeforePreparingResultFut,
    BeforeWorking,
    BeforeWorkingResultFut,
>(
    total: u64,
    connections: u64,
    workers: u64,

    connector_fn: Connector,
    workload_builder: WorkloadBuilder,
    worker_fn: Worker,

    before_preparing: BeforePreparing,
    before_working: BeforeWorking,
) -> Result<(), anyhow::Error>
where
    Connection: Clone + Send + Sync + 'static,
    ConnectionResultFut: Future<Output = Result<Connection>>,
    Connector: FnOnce() -> ConnectionResultFut + Clone,

    Workload: Send + 'static,
    WorkloadBuilder: FnOnce() -> Workload + Send + Sync + Clone,

    WorkResultFut: Future<Output = Result<()>> + Send,
    Worker: FnOnce(Connection, Workload) -> WorkResultFut + Clone + Send + Sync + 'static,

    BeforePreparing: FnOnce() -> BeforePreparingResultFut,
    BeforePreparingResultFut: Future<Output = Result<()>>,
    BeforeWorking: FnOnce() -> BeforeWorkingResultFut,
    BeforeWorkingResultFut: Future<Output = Result<()>>,
{
    before_preparing().await?;
    let conns = {
        let mut conns = Vec::with_capacity(connections as usize);
        for _ in 0..connections {
            let connector_fn = connector_fn.clone();
            conns.push(
                connector_fn()
                    .await
                    .context("preparing connections failed")?,
            );
        }
        conns
    };

    // Avoid lazy evaluation.
    #[allow(clippy::needless_collect)]
    let conn_workloads = conns
        .into_par_iter()
        .enumerate()
        .map(|(i, conn)| {
            let i = i as u64;
            // Those residual_* are for distributing residual evenly.
            let residual_workloads_for_this_conn = total % connections;
            let residual_workers_for_this_conn = workers % connections;

            let (workloads_for_this_conn, workers_for_this_conn) = {
                let workloads_for_this_conn = if i < residual_workloads_for_this_conn {
                    total / connections + 1
                } else {
                    total / connections
                };
                let workers_for_this_conn = if i < residual_workers_for_this_conn {
                    workers / connections + 1
                } else {
                    workers / connections
                };
                (workloads_for_this_conn, workers_for_this_conn)
            };

            let worker_workloads = (0..workers_for_this_conn)
                .into_par_iter()
                .map(|w| {
                    let residual_workloads_for_this_worker =
                        workloads_for_this_conn % workers_for_this_conn;

                    let workloads_for_this_worker = if w < residual_workloads_for_this_worker {
                        workloads_for_this_conn / workers_for_this_conn + 1
                    } else {
                        workloads_for_this_conn / workers_for_this_conn
                    };

                    (0..workloads_for_this_worker)
                        .into_par_iter()
                        .map(|_| {
                            let workload_builder = workload_builder.clone();
                            workload_builder()
                        })
                        .collect()
                })
                .collect();

            (conn, worker_workloads)
        })
        .collect::<Vec<(Connection, Vec<Vec<Workload>>)>>();

    before_working().await?;

    let mut first_reported_error: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
    let hs = conn_workloads
        .into_iter()
        .map(|(conn, worker_workloads)| {
            let first_reported_error = Arc::clone(&first_reported_error);
            let worker_fn = worker_fn.clone();
            tokio::spawn(async move {
                let first_reported_error = Arc::clone(&first_reported_error);
                let hs = worker_workloads
                    .into_iter()
                    .map(|workloads| {
                        let first_reported_error = Arc::clone(&first_reported_error);
                        let worker_fn = worker_fn.clone();
                        let conn = conn.clone();
                        tokio::spawn(async move {
                            for workload in workloads {
                                let worker_fn = worker_fn.clone();
                                if let Err(e) = worker_fn(conn.clone(), workload).await {
                                    first_reported_error.lock().get_or_insert(e);
                                }
                            }
                        })
                    })
                    .collect::<Vec<_>>();

                // TODO: return earily if error
                for h in hs {
                    if let Err(e) = h.await {
                        first_reported_error.lock().get_or_insert(e.into());
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    for h in hs {
        if let Err(e) = h.await {
            first_reported_error.lock().get_or_insert(e.into());
        }
    }

    // Because we will own the lock at this point, we don't need to lock it.
    Arc::get_mut(&mut first_reported_error)
        .expect("other references should have been dropped here")
        .get_mut()
        // ...
        .take()
        .map(Err)
        .unwrap_or(Ok(()))
}

#[cfg(test)]
mod tests {

    use crate::cmd::cldi_cmd;
    use crate::core::mock::context;

    #[test]
    #[should_panic]
    fn test_bench_send_subcmds() {
        let cldi_cmd = cldi_cmd();

        let (mut ctx, _temp_dir) = context();

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "bench",
                    "send",
                    "-c",
                    "1",
                    "--connections",
                    "1",
                    "-t",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "-d",
                    "0x1234",
                    "-v",
                    "0x0",
                    "-q",
                    "500000",
                    "--until",
                    "100",
                    "--disable-watch",
                    "1",
                ],
                &mut ctx,
            )
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn test_bench_call_subcmds() {
        let cldi_cmd = cldi_cmd();

        let (mut ctx, _temp_dir) = context();

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "bench",
                    "call",
                    "-c",
                    "1",
                    "--connections",
                    "1",
                    "--timeout",
                    "1",
                    "-f",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "-t",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "-d",
                    "0x1234",
                    "--height",
                    "100",
                    "1",
                ],
                &mut ctx,
            )
            .unwrap();
    }
}
