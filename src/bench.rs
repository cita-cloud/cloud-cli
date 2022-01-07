use anyhow::Context;
use rayon::prelude::*;
use std::future::Future;
use std::time::Duration;
use std::sync::Arc;
use rand::{thread_rng, Rng};

use tonic::transport::channel::Channel;
use tonic::transport::channel::Endpoint;

use crate::proto::{
    blockchain::{RawTransaction, Transaction},
    // controller::rpc_service_client::RpcServiceClient as ControllerClient,
    // controller::e_service_client::RpcServiceClient as ControllerClient,
    controller::{
        rpc_service_client::RpcServiceClient as ControllerClient
    },
    evm::{
        rpc_service_client::RpcServiceClient as EvmClient, Balance, ByteAbi, ByteCode, Nonce,
        Receipt,
    },
    executor::{executor_service_client::ExecutorServiceClient as ExecutorClient, CallRequest},
};

async fn bench_send(
    rpc_addr: &str,
    total: u64,
    connections: u64,
    workers: u64,
    timeout: u64,
) -> Result<(), anyhow::Error>
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
    let before_preparing = || println!("Preparing connections and transactions..");
    let before_working = || {
        println!("Sending transactions...");
        t.replace(std::time::Instant::now());
    };

    let connector = || async {
        let endpoint = {
            let addr = format!("http://{}", rpc_addr);
            let mut endpoint = Endpoint::from_shared(addr).unwrap();
            if timeout > 0 {
                endpoint = endpoint.timeout(Duration::from_secs(timeout));
            }
            endpoint
        };
        endpoint.connect().await.map(ControllerClient::new)
    };

    let workload_builder = || {
        let mut rng = thread_rng();
        let tx = Transaction {
            to: rng.gen::<[u8; 20]>().to_vec(),
            data: rng.gen::<[u8; 32]>().to_vec(),
            value: rng.gen::<[u8; 32]>().to_vec(),
            nonce: rng.gen::<u64>().to_string(),
            quota: 3_000_000,
            valid_until_block: start_at + 95,
            chain_id: chain_id.clone(),
            version,
        };
        client.prepare_raw_tx(tx)
    };

    let progbar_cloned = progbar.clone();
    // OK, I couldn't manage to use &mut for conn here and have to pass a owned client,
    // because there is no way to specify the lifetime bound for the returned closure here.
    let worker_fn = |mut conn: ControllerClient<Channel>, tx: RawTransaction| async move {
        if conn
            .send_raw_transaction(tx)
            .await
            .map(|h| !h.into_inner().hash.is_empty())
            .unwrap_or(false)
        {
            progbar_cloned.inc(1);
        }
    };

    bench_fn(
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
    .context("bench failed")?;

    progbar.finish_at_current_pos();

    println!(
        "sending `{}` transactions finished in `{}` ms",
        total,
        t.unwrap().elapsed().as_millis()
    );
    let success = progbar.position();
    let failure = total - success;
    println!("`{}` success, `{}` failure", success, failure);

    Ok(())
}


async fn bench_call(
    executor_addr: &str,
    from: Vec<u8>,
    to: Vec<u8>,
    payload: Vec<u8>,

    total: u64,
    connections: u64,
    workers: u64,
    timeout: u64,
) -> Result<(), anyhow::Error> {
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
    let before_preparing = || println!("Preparing connections and transactions..");
    let before_working = || {
        println!("Sending transactions...");
        t.replace(std::time::Instant::now());
    };

    let connector = || async {
        let endpoint = {
            let addr = format!("http://{}", executor_addr);
            let mut endpoint = Endpoint::from_shared(addr).unwrap();
            if timeout > 0 {
                endpoint = endpoint.timeout(Duration::from_secs(timeout));
            }
            endpoint
        };
        endpoint.connect().await.map(ExecutorClient::new)
    };

    let workload_builder = || CallRequest { from, to, method: payload, args: vec![] };
    let progbar_cloned = progbar.clone();
    let worker_fn = |mut client: ExecutorClient<Channel>, call_req: CallRequest| async move {
        if client
            .call(call_req)
            .await
            .is_ok()
        {
            progbar_cloned.inc(1);
        }
    };

    let mut t = None;
    let before_preparing = || println!("Preparing call requests");
    let before_working = || {
        println!("Sending call requests...");
        t.replace(std::time::Instant::now());
    };

    bench_fn(
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
    .context("bench failed")?;

    progbar.finish_at_current_pos();

    println!(
        "sending `{}` transactions finished in `{}` ms",
        total,
        t.unwrap().elapsed().as_millis()
    );
    let success = progbar.position();
    let failure = total - success;
    println!("`{}` success, `{}` failure", success, failure);

    Ok(())
}

async fn bench_tx() {

}


// I think it's too complex.
#[allow(clippy::too_many_arguments)]
async fn bench_fn<
    Connector,
    Connection,
    ConnectionError,
    ConnectionResultFut,
    WorkloadBuilder,
    Workload,
    WorkerFn,
    WorkResultFut,
    BeforePreparing,
    BeforeWorking,
>(
    total: u64,
    connections: u64,
    workers: u64,

    connector: Connector,
    workload_builder: WorkloadBuilder,
    worker_fn: WorkerFn,

    before_preparing: BeforePreparing,
    before_working: BeforeWorking,
) -> Result<(), anyhow::Error>
where
    Connection: Clone + Send + Sync + 'static,
    ConnectionError: std::error::Error + Send + Sync + 'static,
    ConnectionResultFut: Future<Output = Result<Connection, ConnectionError>>,
    Connector: FnOnce() -> ConnectionResultFut + Clone,

    Workload: Send + 'static,
    WorkloadBuilder: FnOnce() -> Workload + Send + Sync + Clone,

    WorkResultFut: Future<Output = ()> + Send,
    WorkerFn: FnOnce(Connection, Workload) -> WorkResultFut + Clone + Send + Sync + 'static,

    BeforePreparing: FnOnce(),
    BeforeWorking: FnOnce(),
{
    before_preparing();
    let conns = {
        let mut conns = Vec::with_capacity(connections as usize);
        for _ in 0..connections {
            let connector = connector.clone();
            conns.push(connector().await.context("preparing connection failed")?);
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
    let hs = conn_workloads
        .into_iter()
        .map(|(conn, worker_workloads)| {
            let worker_fn = worker_fn.clone();
            tokio::spawn(async move {
                let hs = worker_workloads
                    .into_iter()
                    .map(|workloads| {
                        let worker_fn = worker_fn.clone();
                        let conn = conn.clone();
                        tokio::spawn(async move {
                            for workload in workloads {
                                let worker_fn = worker_fn.clone();
                                worker_fn(conn.clone(), workload).await;
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                for h in hs {
                    let _ = h.await;
                }
            })
        })
        .collect::<Vec<_>>();

    for h in hs {
        let _ = h.await;
    }

    Ok(())
}
