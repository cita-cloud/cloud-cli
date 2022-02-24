use anyhow::Context as _;
use anyhow::Result;
use rayon::prelude::*;
use std::future::Future;
use std::time::Duration;
use std::sync::Arc;
use parking_lot::Mutex;
use rand::{thread_rng, Rng};

use super::Command;
use crate::crypto::Crypto;
use crate::sdk::client::GrpcClientBehaviour;
use clap::Arg;
use crate::sdk::context::Context;

use tonic::transport::channel::Channel;
use tonic::transport::channel::Endpoint;

use crate::sdk::{
    controller::{
        ControllerBehaviour, SignerBehaviour, TransactionSenderBehaviour,
        ControllerClient,
    },
};
use crate::proto::{
    blockchain::Transaction,
    evm::{
        rpc_service_client::RpcServiceClient as EvmClient, Balance, ByteAbi, ByteCode, Nonce,
        Receipt,
    },
    executor::{executor_service_client::ExecutorServiceClient as ExecutorClient, CallRequest},
};

use crate::utils::{parse_addr, parse_data, parse_value, hex};

pub fn bench_send<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
{
    Command::<Context<Co, Ex, Ev>>::new("bench-send")
        .about("Send transactions with {-c} workers over {--connections} connections")
        .arg(
            Arg::new("concurrency")
                .help(
                    "Number of request workers to run concurrently for sending transactions. \
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
                .default_value("16")
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
                .default_value("100")
                .validator(str::parse::<u32>),
        )
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

                let (current_block_number, system_config) =
                    tokio::try_join!(ctx.controller.get_block_number(false), ctx.controller.get_system_config())
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
                    signer.sign_raw_tx(raw_tx)
                };

                let worker_fn = |client: Co, raw| async move {
                    client.send_raw(raw).await.map(|_| ())
                };

                bench_fn_with_progbar(
                    total,
                    connections,
                    workers,
                    connector,
                    workload_builder,
                    worker_fn,
                    "Preparing connections and transactions",
                    "Sending transactions.."
                ).await;

                anyhow::Ok(())
            })??;
            Ok(())
        })
}

// async fn bench_send(
//     rpc_addr: &str,
//     total: u64,
//     connections: u64,
//     workers: u64,
//     timeout: u64,
// ) -> Result<(), anyhow::Error>
// {
//     let progbar = {
//         let progbar = indicatif::ProgressBar::new(total);
//         progbar.set_style(
//             indicatif::ProgressStyle::default_bar()
//                 .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
//                 .progress_chars("=> ")
//         );
//         Arc::new(progbar)
//     };
//     let mut t = None;
//     let before_preparing = || println!("Preparing connections and transactions..");
//     let before_working = || {
//         println!("Sending transactions...");
//         t.replace(std::time::Instant::now());
//     };

//     let connector = || async {
//         let endpoint = {
//             let addr = format!("http://{}", rpc_addr);
//             let mut endpoint = Endpoint::from_shared(addr).unwrap();
//             if timeout > 0 {
//                 endpoint = endpoint.timeout(Duration::from_secs(timeout));
//             }
//             endpoint
//         };
//         endpoint.connect().await.map(ControllerClient::new)
//     };

//     let workload_builder = || {
//         let mut rng = thread_rng();
//         let tx = Transaction {
//             to: rng.gen::<[u8; 20]>().to_vec(),
//             data: rng.gen::<[u8; 32]>().to_vec(),
//             value: rng.gen::<[u8; 32]>().to_vec(),
//             nonce: rng.gen::<u64>().to_string(),
//             quota: 3_000_000,
//             valid_until_block: start_at + 95,
//             chain_id: chain_id.clone(),
//             version,
//         };
//         client.prepare_raw_tx(tx)
//     };

//     let progbar_cloned = progbar.clone();
//     // OK, I couldn't manage to use &mut for conn here and have to pass a owned client,
//     // because there is no way to specify the lifetime bound for the returned closure here.
//     let worker_fn = |mut conn: ControllerClient<Channel>, tx: RawTransaction| async move {
//         if conn
//             .send_raw_transaction(tx)
//             .await
//             .map(|h| !h.into_inner().hash.is_empty())
//             .unwrap_or(false)
//         {
//             progbar_cloned.inc(1);
//         }
//     };

//     bench_fn(
//         total,
//         connections,
//         workers,
//         connector,
//         workload_builder,
//         worker_fn,
//         before_preparing,
//         before_working,
//     )
//     .await
//     .context("bench failed")?;

//     progbar.finish_at_current_pos();

//     println!(
//         "sending `{}` transactions finished in `{}` ms",
//         total,
//         t.unwrap().elapsed().as_millis()
//     );
//     let success = progbar.position();
//     let failure = total - success;
//     println!("`{}` success, `{}` failure", success, failure);

//     Ok(())
// }


// async fn bench_call(
//     executor_addr: &str,
//     from: Vec<u8>,
//     to: Vec<u8>,
//     payload: Vec<u8>,

//     total: u64,
//     connections: u64,
//     workers: u64,
//     timeout: u64,
// ) -> Result<(), anyhow::Error> {
//     let progbar = {
//         let progbar = indicatif::ProgressBar::new(total);
//         progbar.set_style(
//             indicatif::ProgressStyle::default_bar()
//                 .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
//                 .progress_chars("=> ")
//         );
//         Arc::new(progbar)
//     };
//     let mut t = None;
//     let before_preparing = || println!("Preparing connections and transactions..");
//     let before_working = || {
//         println!("Sending transactions...");
//         t.replace(std::time::Instant::now());
//     };

//     let connector = || async {
//         let endpoint = {
//             let addr = format!("http://{}", executor_addr);
//             let mut endpoint = Endpoint::from_shared(addr).unwrap();
//             if timeout > 0 {
//                 endpoint = endpoint.timeout(Duration::from_secs(timeout));
//             }
//             endpoint
//         };
//         endpoint.connect().await.map(ExecutorClient::new)
//     };

//     let workload_builder = || CallRequest { from, to, method: payload, args: vec![] };
//     let progbar_cloned = progbar.clone();
//     let worker_fn = |mut client: ExecutorClient<Channel>, call_req: CallRequest| async move {
//         if client
//             .call(call_req)
//             .await
//             .is_ok()
//         {
//             progbar_cloned.inc(1);
//         }
//     };

//     let mut t = None;
//     let before_preparing = || println!("Preparing call requests");
//     let before_working = || {
//         println!("Sending call requests...");
//         t.replace(std::time::Instant::now());
//     };

//     bench_fn(
//         total,
//         connections,
//         workers,
//         connector,
//         workload_builder,
//         worker_fn,
//         before_preparing,
//         before_working,
//     )
//     .await
//     .context("bench failed")?;

//     progbar.finish_at_current_pos();

//     println!(
//         "sending `{}` transactions finished in `{}` ms",
//         total,
//         t.unwrap().elapsed().as_millis()
//     );
//     let success = progbar.position();
//     let failure = total - success;
//     println!("`{}` success, `{}` failure", success, failure);

//     Ok(())
// }

// async fn bench_tx() {

// }


async fn bench_fn_with_progbar<
    Connector,
    Connection,
    // ConnectionError,
    ConnectionResultFut,
    Workload,
    WorkloadBuilder,
    // WorkError,
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
)
where
    Connection: Clone + Send + Sync + 'static,
    // ConnectionError: std::error::Error + Send + Sync + 'static,
    ConnectionResultFut: Future<Output = Result<Connection>>,
    Connector: FnOnce() -> ConnectionResultFut + Clone,

    Workload: Send + 'static,
    WorkloadBuilder: FnOnce() -> Workload + Send + Sync + Clone,

    // WorkError: std::error::Error + Send + Sync + 'static,
    WorkResultFut: Future<Output = Result<()>> + Send,
    Worker: FnOnce(Connection, Workload) -> WorkResultFut + Clone + Send + Sync + 'static,
{
    let progbar = {
        let progbar = indicatif::ProgressBar::new(total);
        progbar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
                .progress_chars("=> ")
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
        println!("bench isn't completed successfully, the first reported error is {}", e);
    }
}

// I think it's too complex.
#[allow(clippy::too_many_arguments)]
async fn bench_fn<
    Connector,
    Connection,
    // ConnectionError,
    ConnectionResultFut,
    Workload,
    WorkloadBuilder,
    // WorkError,
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
    // ConnectionError: std::error::Error + Send + Sync + 'static,
    ConnectionResultFut: Future<Output = Result<Connection>>,
    Connector: FnOnce() -> ConnectionResultFut + Clone,

    Workload: Send + 'static,
    WorkloadBuilder: FnOnce() -> Workload + Send + Sync + Clone,

    // WorkError: std::error::Error + Send + Sync + 'static,
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
            conns.push(connector_fn().await.context("preparing connection failed")?);
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
