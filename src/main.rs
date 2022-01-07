#[cfg(all(feature = "evm", feature = "chaincode"))]
compile_error!("features `evm` and `chaincode` are mutually exclusive");

#[cfg(all(feature = "crypto_sm", feature = "crypto_eth"))]
compile_error!("features `crypto_sm` and `crypto_eth` are mutually exclusive");

mod cli;
mod client;
mod crypto;
mod display;
mod proto;
mod utils;
mod wallet;

use std::sync::Arc;
use std::time::Duration;

use rand::{thread_rng, Rng};
use serde_json::json;

use anyhow::anyhow;
use anyhow::Result;

use tonic::transport::channel::Channel;
use tonic::transport::channel::Endpoint;

use rayon::prelude::*;

use proto::{
    blockchain::{RawTransaction, Transaction},
    controller::rpc_service_client::RpcServiceClient as ControllerClient,
};

use cli::build_cli;
use client::Client;
use display::Display as _;
use utils::{hex, parse_addr, parse_data, parse_value};
use wallet::Wallet;

/// Store action target address
pub const STORE_ADDRESS: &str = "0xffffffffffffffffffffffffffffffffff010000";
/// StoreAbi action target address
pub const ABI_ADDRESS: &str = "0xffffffffffffffffffffffffffffffffff010001";
/// Amend action target address
pub const AMEND_ADDRESS: &str = "0xffffffffffffffffffffffffffffffffff010002";

/// amend the abi data
pub const AMEND_ABI: &str = "0x01";
/// amend the account code
pub const AMEND_CODE: &str = "0x02";
/// amend the kv of db
pub const AMEND_KV_H256: &str = "0x03";
/// amend account balance
pub const AMEND_BALANCE: &str = "0x05";

#[tokio::main]
async fn main() -> Result<()> {
    // security not included yet:p
    let mut cli = build_cli();

    let matches = cli.clone().get_matches();

    let user = matches
        .value_of("user")
        .map(str::to_string)
        .or_else(|| std::env::var("CITA_CLOUD_DEFAULT_USER").ok());

    let rpc_addr = matches
        .value_of("rpc_addr")
        .map(str::to_string)
        .unwrap_or_else(|| {
            if let Ok(rpc_addr) = std::env::var("CITA_CLOUD_RPC_ADDR") {
                rpc_addr
            } else {
                "localhost:50004".to_string()
            }
        });
    let executor_addr = matches
        .value_of("executor_addr")
        .map(str::to_string)
        .unwrap_or_else(|| {
            if let Ok(executor_addr) = std::env::var("CITA_CLOUD_EXECUTOR_ADDR") {
                executor_addr
            } else {
                "localhost:50002".to_string()
            }
        });

    let wallet = {
        let data_dir = {
            let home = home::home_dir().expect("cannot find home dir");
            home.join(".cloud-cli")
        };
        Wallet::open(data_dir)
    };

    let account = match user {
        Some(user) => wallet
            .load_account(&user)
            .ok_or_else(|| anyhow!("account not found"))?,
        None => wallet.default_account()?,
    };

    let mut client = Client::new(account.clone(), &rpc_addr, &executor_addr);

    let subcmd = match matches.subcommand() {
        Some(subcmd) => subcmd,
        None => {
            cli.print_help()?;
            return Err(anyhow!("no subcommand provided"));
        }
    };
    match subcmd {
        ("call", m) => {
            let from = parse_addr(m.value_of("from").unwrap_or_default())?;
            let to = parse_addr(m.value_of("to").unwrap())?;
            let data = parse_data(m.value_of("data").unwrap())?;

            let result = client.call(from, to, data).await;
            println!("result: {}", hex(&result));
        }
        ("send", m) => {
            let to = parse_addr(m.value_of("to").unwrap())?;
            let data = parse_data(m.value_of("data").unwrap())?;
            let value = parse_value(m.value_of("value").unwrap_or_default())?;

            let tx_hash = client.send(to, data, value).await;
            println!("tx_hash: {}", hex(&tx_hash));
        }
        ("block-number", m) => {
            let for_pending = m.is_present("for_pending");

            let block_number = client.get_block_number(for_pending).await;
            println!("block_number: {}", block_number);
        }
        ("get-block", m) => {
            let s = m.value_of("number_or_hash").unwrap();
            let block = if s.starts_with("0x") {
                let hash = parse_value(s)?;
                client.get_block_by_hash(hash).await
            } else {
                let block_number = s.parse()?;
                client.get_block_by_number(block_number).await
            };

            println!("{}", block.display());
        }
        ("block-hash", m) => {
            let n = m.value_of("number").unwrap().parse().unwrap();
            let hash = client.get_block_hash(n).await;
            println!("hash: 0x{}", hex::encode(&hash));
        }
        ("get-tx", m) => {
            let tx_hash = parse_value(m.value_of("tx_hash").unwrap())?;

            let tx = client.get_tx(tx_hash).await;
            println!("tx: {}", tx.display());
        }
        ("get-tx-index", m) => {
            let tx_hash = parse_value(m.value_of("tx_hash").unwrap())?;

            let index = client.get_tx_index(tx_hash).await;
            println!("tx index: {}", index.tx_index);
        }
        ("get-tx-block-number", m) => {
            let tx_hash = parse_value(m.value_of("tx_hash").unwrap())?;

            let block_number = client.get_tx_block_number(tx_hash).await;
            println!("block number: {}", block_number.block_number);
        }
        ("peer-count", _m) => {
            let cnt = client.get_peer_count().await;
            println!("peer_count: {}", cnt);
        }
        ("add-node", m) => {
            let address = m.value_of("multi_address").unwrap();
            println!("Adding node: {}", address);
            let status = client.add_node(address.into()).await;
            if status != 0 {
                println!("`add_node` failed with status `{}`", status);
            } else {
                println!("Ok");
            }
        }
        ("peers-info", _m) => {
            let peers_info = client.get_peers_info().await;
            for info in &peers_info {
                println!("{}", info.display());
            }
        }
        ("system-config", _m) => {
            let system_config = client.get_system_config().await;
            println!("{}", system_config.display());
        }
        ("bench", m) => {
            let total = m.value_of("total").unwrap().parse::<u64>().unwrap();
            let connections = m.value_of("connections").unwrap().parse::<u64>().unwrap();
            let timeout = m.value_of("timeout").unwrap().parse::<u64>().unwrap();
            let workers = m
                .value_of("concurrency")
                .map(|s| s.parse::<u64>().unwrap())
                .unwrap_or(total);

            let mut start_at = client.get_block_number(false).await;
            let sys_config = client.get_system_config().await;
            let chain_id = sys_config.chain_id;
            let version = sys_config.version;

            println!("Preparing connections and transactions..");
            let conns = {
                let mut conns = vec![];
                let endpoint = {
                    let addr = format!("http://{}", rpc_addr);
                    let mut endpoint = Endpoint::from_shared(addr).unwrap();
                    if timeout > 0 {
                        endpoint = endpoint.timeout(Duration::from_secs(timeout));
                    }
                    endpoint
                };
                for _ in 0..connections {
                    let conn = endpoint.connect().await.unwrap();
                    conns.push(ControllerClient::new(conn));
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
                    let residual_txs_for_this_conn = total % connections;
                    let residual_workers_for_this_conn = workers % connections;

                    let (txs_for_this_conn, workers_for_this_conn) = {
                        let txs_for_this_conn = if i < residual_txs_for_this_conn {
                            total / connections + 1
                        } else {
                            total / connections
                        };
                        let workers_for_this_conn = if i < residual_workers_for_this_conn {
                            workers / connections + 1
                        } else {
                            workers / connections
                        };
                        (txs_for_this_conn, workers_for_this_conn)
                    };

                    let worker_workloads = (0..workers_for_this_conn)
                        .into_par_iter()
                        .map(|w| {
                            let residual_txs_for_this_worker =
                                txs_for_this_conn % workers_for_this_conn;

                            let txs_for_this_worker = if w < residual_txs_for_this_worker {
                                txs_for_this_conn / workers_for_this_conn + 1
                            } else {
                                txs_for_this_conn / workers_for_this_conn
                            };

                            (0..txs_for_this_worker)
                                .into_par_iter()
                                .map(|_| {
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
                                })
                                .collect()
                        })
                        .collect();

                    (conn, worker_workloads)
                })
                .collect::<Vec<(ControllerClient<Channel>, Vec<Vec<RawTransaction>>)>>();

            println!("Sending transactions...");
            let progbar = {
                let progbar = indicatif::ProgressBar::new(total);
                progbar.set_style(
                    indicatif::ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
                        .progress_chars("=> ")
                );
                Arc::new(progbar)
            };
            let t = std::time::Instant::now();
            let hs = conn_workloads
                .into_iter()
                .map(|(conn, worker_workloads)| {
                    let progbar = progbar.clone();
                    tokio::spawn(async move {
                        let hs = worker_workloads
                            .into_iter()
                            .map(|workload| {
                                let progbar = progbar.clone();
                                let mut conn = conn.clone();
                                tokio::spawn(async move {
                                    for tx in workload {
                                        if conn
                                            .send_raw_transaction(tx)
                                            .await
                                            .map(|h| !h.into_inner().hash.is_empty())
                                            .unwrap_or(false)
                                        {
                                            progbar.inc(1);
                                        }
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
            progbar.finish_at_current_pos();

            println!(
                "sending `{}` transactions finished in `{}` ms",
                total,
                t.elapsed().as_millis()
            );
            let success = progbar.position();
            let failure = total - success;
            println!("`{}` success, `{}` failure", success, failure);

            let mut check_interval = tokio::time::interval(Duration::from_secs(1));
            let mut finalized_tx = 0;
            let mut begin_time = None;

            while finalized_tx < total {
                check_interval.tick().await;
                let end_at = {
                    let n = client.get_block_number(false).await;
                    if n >= start_at {
                        n
                    } else {
                        continue;
                    }
                };

                let blocks = {
                    let mut blocks = vec![];
                    for i in start_at..=end_at {
                        let block = client.get_block_by_number(i).await;
                        blocks.push(block);
                    }
                    blocks
                };

                for b in blocks {
                    let (header, body) = (b.header.unwrap(), b.body.unwrap());

                    let height = header.height;
                    let secs = {
                        let t = std::time::UNIX_EPOCH + Duration::from_millis(header.timestamp);
                        if begin_time.is_none() {
                            begin_time.replace(t);
                        }
                        t.duration_since(begin_time.unwrap()).unwrap().as_secs()
                    };
                    let cnt = body.tx_hashes.len() as u64;
                    finalized_tx += cnt;
                    println!(
                        "{:0>2}:{:0>2} block `{}` contains `{}` txs, finalized: `{}`",
                        secs / 60,
                        secs % 60,
                        height,
                        cnt,
                        finalized_tx
                    );
                }
                start_at = end_at + 1;
            }
        }
        ("account", m) => {
            if let Some(subcmd) = m.subcommand() {
                match subcmd {
                    ("create", m) => {
                        let user = m.value_of("user").unwrap();
                        let addr = wallet.create_account(user);
                        println!("user: `{}`\naccount_addr: {}", user, hex(&addr));
                    }
                    ("login", m) => {
                        let user = m.value_of("user").unwrap();
                        let addr = wallet.set_default_account(user)?;
                        println!(
                            "OK, now the default user is `{}`, account addr is {}",
                            user,
                            hex(&addr)
                        );
                    }
                    ("import", m) => {
                        let user = m.value_of("user").unwrap();
                        let pk = parse_data(m.value_of("pk").unwrap())?;
                        let sk = parse_data(m.value_of("sk").unwrap())?;
                        wallet.import_account(user, pk, sk);
                        println!("OK, account `{}` imported", user);
                    }
                    ("export", m) => {
                        let user = m.value_of("user").unwrap();
                        if let Some(account) = wallet.load_account(user) {
                            println!("{}", account.display());
                        } else {
                            println!("No such an account");
                        }
                    }
                    ("delete", m) => {
                        let user = m.value_of("user").unwrap();
                        wallet.delete_account(user);
                        println!("Ok, the account of user `{}` has been deleted", user);
                    }
                    _ => unreachable!(),
                }
            } else {
                let accounts = wallet
                    .list_account()
                    .into_iter()
                    .map(|(user, addr)| {
                        json!({
                            "user": user,
                            "addr": hex(&addr),
                        })
                    })
                    .collect::<Vec<_>>();
                let display = serde_json::to_string_pretty(&json!(accounts))?;
                println!("{}", display);
            }
        }
        ("completions", m) => {
            let shell: clap_complete::Shell = m.value_of("shell").unwrap().parse().unwrap();
            let mut cli = cli::build_cli();
            let mut stdout = std::io::stdout();
            clap_complete::generate(shell, &mut cli, "cldi", &mut stdout);
        }
        ("update-admin", m) => {
            let admin_addr = parse_addr(m.value_of("admin_addr").unwrap())?;
            let tx_hash = client.update_admin(admin_addr).await;
            println!("tx_hash: {}", hex(&tx_hash));
        }
        ("update-validators", m) => {
            let validators = m
                .values_of("validators")
                .unwrap()
                .map(parse_addr)
                .collect::<Result<Vec<_>>>()?;
            let tx_hash = client.update_validators(&validators).await;
            println!("tx_hash: {}", hex(&tx_hash));
        }
        ("emergency-brake", m) => {
            let switch = m.value_of("switch").unwrap() == "on";
            let tx_hash = client.emergency_brake(switch).await;
            println!("tx_hash: {}", hex(&tx_hash));
        }
        ("set-block-interval", m) => {
            let block_interval = m.value_of("block_interval").unwrap().parse::<u32>()?;
            let tx_hash = client.set_block_interval(block_interval).await;
            println!("tx_hash: {}", hex(&tx_hash));
        }
        #[cfg(feature = "evm")]
        ("create", m) => {
            let to = vec![];
            let data = parse_data(m.value_of("data").unwrap())?;
            let value = parse_value(m.value_of("value").unwrap_or_default())?;

            let tx_hash = client.send(to, data, value).await;
            println!("tx_hash: {}", hex(&tx_hash));
        }
        #[cfg(feature = "evm")]
        ("receipt", m) => {
            let tx_hash = parse_value(m.value_of("tx_hash").unwrap())?;

            let receipt = client.get_receipt(tx_hash).await;
            println!("{}", receipt.display());
        }
        #[cfg(feature = "evm")]
        ("get-code", m) => {
            let addr = parse_addr(m.value_of("addr").unwrap())?;

            let code = client.get_code(addr).await;
            println!("code: {}", hex(&code.byte_code));
        }
        #[cfg(feature = "evm")]
        ("get-balance", m) => {
            let addr = parse_addr(m.value_of("addr").unwrap())?;

            let balance = client.get_balance(addr).await;
            println!("balance: {}", hex(&balance.value));
        }
        #[cfg(feature = "evm")]
        ("get-tx-count", m) => {
            let addr = parse_addr(m.value_of("addr").unwrap())?;

            let tx_count = client.get_transaction_count(addr).await;
            println!("tx count: {}", hex(&tx_count.nonce));
        }
        #[cfg(feature = "evm")]
        ("store-abi", m) => {
            let to = parse_addr(ABI_ADDRESS)?;
            let data = {
                let addr = parse_addr(m.value_of("addr").unwrap())?;
                let abi = m.value_of("abi").unwrap();

                // [<addr><abi>]
                [addr.as_slice(), abi.as_bytes()].concat()
            };

            let tx_hash = client.send(to, data, vec![0; 32]).await;
            println!("tx_hash: {}", hex(&tx_hash));
        }
        #[cfg(feature = "evm")]
        ("get-abi", m) => {
            let addr = parse_addr(m.value_of("addr").unwrap())?;
            let abi = client.get_abi(addr).await;

            println!("ABI: {}", String::from_utf8(abi.bytes_abi)?);
        }
        _ => {
            unreachable!()
        }
    }

    Ok(())
}
