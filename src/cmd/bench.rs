use std::{future::Future, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use clap::Arg;
use parking_lot::Mutex;
use rand::{thread_rng, Rng};
use rayon::prelude::*;

use crate::{
    cmd::Command,
    core::executor::ExecutorBehaviour,
    core::{
        client::GrpcClientBehaviour,
        context::Context,
        controller::{ControllerBehaviour, SignerBehaviour},
    },
    proto::blockchain::Transaction,
    utils::{parse_addr, parse_data},
};


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
                .takes_value(true)
                .required(false)
                .validator(str::parse::<u64>),
        )
        .arg(
            Arg::new("connections")
                .help("Number of connections connects to server")
                .long("connections")
                .takes_value(true)
                .default_value("1")
                .validator(str::parse::<u64>),
        )
        .arg(
            Arg::new("timeout")
                .help("Timeout for each request (in seconds). 0 means no timeout")
                .long("timeout")
                .takes_value(true)
                .default_value("0")
                .validator(str::parse::<u64>),
        )
        .arg(
            Arg::new("total")
                .help("Number of transactions to send")
                .takes_value(true)
                .required(true)
                .validator(str::parse::<u32>),
        )
}

pub fn bench_send<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
{
    bench_basic::<Co, Ex, Ev>()
        .name("bench-send")
        .about("Send transactions with {-c} workers over {--connections} connections")
        // TODO: add args to allow customized transaction
        .handler(|_cmd, m, ctx| {
            let total = m.value_of("total").unwrap().parse::<u64>().unwrap();
            let connections = m.value_of("connections").unwrap().parse::<u64>().unwrap();
            let timeout = m.value_of("timeout").unwrap().parse::<u64>().unwrap();
            let workers = m
                .value_of("concurrency")
                .map(|s| s.parse::<u64>().unwrap())
                .unwrap_or(total);

            let controller_addr = ctx.current_controller_addr();
            let signer = ctx.current_account()?;
            ctx.rt.block_on(async {
                let connector = || async {
                    if timeout > 0 {
                        Co::connect_timeout(controller_addr, Duration::from_secs(timeout)).await
                    } else {
                        Co::connect(controller_addr).await
                    }
                };

                let (current_block_number, system_config) = tokio::try_join!(
                    ctx.controller.get_block_number(false),
                    ctx.controller.get_system_config()
                )
                .context("failed to fetch chain status")?;

                let workload_builder = || {
                    let mut rng = thread_rng();
                    let raw_tx = Transaction {
                        to: rng.gen::<[u8; 20]>().to_vec(),
                        data: rng.gen::<[u8; 32]>().to_vec(),
                        value: rng.gen::<[u8; 32]>().to_vec(),
                        nonce: rng.gen::<u64>().to_string(),
                        quota: 3_000_000,
                        valid_until_block: current_block_number + 95,
                        chain_id: system_config.chain_id.clone(),
                        version: system_config.version,
                    };
                    dbg!("before sign");
                    let res = signer.sign_raw_tx(raw_tx);
                    dbg!("after sign");
                    res
                };

                let worker_fn =
                    |client: Co, raw| async move { client.send_raw(raw).await.map(|_| ()) };

                bench_fn_with_progbar(
                    total,
                    connections,
                    workers,
                    connector,
                    workload_builder,
                    worker_fn,
                    "Preparing connections and transactions",
                    "Sending transactions..",
                )
                .await;

                anyhow::Ok(())
            })??;
            Ok(())
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
                .short('d')
                .long("data")
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        )
        .handler(|_cmd, m, ctx| {
            let total = m.value_of("total").unwrap().parse::<u64>().unwrap();
            let connections = m.value_of("connections").unwrap().parse::<u64>().unwrap();
            let timeout = m.value_of("timeout").unwrap().parse::<u64>().unwrap();
            let workers = m
                .value_of("concurrency")
                .map(|s| s.parse::<u64>().unwrap())
                .unwrap_or(total);

            let from = parse_addr(m.value_of("from").unwrap())?;
            let to = parse_addr(m.value_of("to").unwrap())?;
            let data = parse_data(m.value_of("data").unwrap())?;

            let executor_addr = ctx.current_executor_addr();
            ctx.rt.block_on(async {
                let connector = || async {
                    if timeout > 0 {
                        Ex::connect_timeout(executor_addr, Duration::from_secs(timeout)).await
                    } else {
                        Ex::connect(executor_addr).await
                    }
                };

                let workload_builder = || (from, to, data);

                let worker_fn = |client: Ex, (from, to, data)| async move {
                    client.call(from, to, data).await.map(|_| ())
                };

                bench_fn_with_progbar(
                    total,
                    connections,
                    workers,
                    connector,
                    workload_builder,
                    worker_fn,
                    "Preparing connections and transactions",
                    "Sending transactions..",
                )
                .await;

                anyhow::Ok(())
            })??;
            Ok(())
        })
}

async fn bench_fn_with_progbar<
    Connector,
    Connection,
    ConnectionResultFut,
    Workload,
    WorkloadBuilder,
    WorkResultFut,
    Worker,
>(
    total: u64,
    connections: u64,
    workers: u64,

    connector: Connector,
    workload_builder: WorkloadBuilder,
    worker_fn: Worker,

    preparing_info: &str,
    working_info: &str,
) where
    Connection: Clone + Send + Sync + 'static,
    ConnectionResultFut: Future<Output = Result<Connection>>,
    Connector: FnOnce() -> ConnectionResultFut + Clone,

    Workload: Send + 'static,
    WorkloadBuilder: FnOnce() -> Workload + Send + Sync + Clone,

    WorkResultFut: Future<Output = Result<()>> + Send,
    Worker: FnOnce(Connection, Workload) -> WorkResultFut + Clone + Send + Sync + 'static,
{
    let progbar = {
        let progbar = indicatif::ProgressBar::new(total);
        progbar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}",
                )
                .progress_chars("=> "),
        );
        Arc::new(progbar)
    };
    let mut t = None;

    let before_preparing = || println!("{preparing_info}");
    let before_working = || {
        println!("{working_info}");
        // start the timer
        t.replace(std::time::Instant::now());
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

    dbg!(&bench_res);
    progbar.finish_at_current_pos();

    println!(
        "`{}` tasks finished in `{}` ms",
        total,
        t.unwrap().elapsed().as_millis()
    );
    let success = progbar.position();
    let failure = total - success;
    println!("`{}` success, `{}` failure", success, failure);

    if let Err(e) = bench_res {
        println!(
            "bench isn't completed successfully, the first reported error is {}",
            e
        );
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
    BeforeWorking,
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

    BeforePreparing: FnOnce(),
    BeforeWorking: FnOnce(),
{
    before_preparing();
    let conns = {
        let mut conns = Vec::with_capacity(connections as usize);
        for _ in 0..connections {
            let connector_fn = connector_fn.clone();
            conns.push(
                connector_fn()
                    .await
                    .context("preparing connection failed")?,
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

    before_working();

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
                                    first_reported_error.lock().get_or_insert(e.into());
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
    use super::*;
    use crate::cmd::cldi_cmd;
    use crate::core::mock::context;

    #[test]
    fn test_bench_send() {
        let cmd = bench_send();

        let (mut ctx, _temp_dir) = context();
        ctx.controller.expect_get_block_number()
            .returning(|_| Ok(Default::default()));
        ctx.controller.expect_get_system_config()
            .returning(|| Ok(Default::default()));
        ctx.controller.expect_send_raw()
            .times(1)
            .returning(|_| Ok(Default::default()));

        cmd.exec_from(
            ["bench-send", "1"],
            &mut ctx
        ).unwrap();
    }
}
