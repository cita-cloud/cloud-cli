#[cfg(all(feature = "evm", feature = "chaincode"))]
compile_error!("features `evm` and `chaincode` are mutually exclusive");

mod interactive;

use clap::App;
use clap::AppSettings;
use clap::Arg;
use cloud_client::Client;
use cloud_client::Display as _;
use futures::future::join_all;
use interactive::Interactive;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let hex_validator = |s: &str| {
        let s = remove_0x(s);
        hex::decode(s)
            .map_err(|e| format!("this must be a valid hex encoded, parse error: `{}`", e))
    };

    // subcommands
    let call = App::new("call")
        .about("Executor call")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("from")
                .short('f')
                .long("from")
                .required(false)
                .takes_value(true)
                .validator(hex_validator),
        )
        .arg(
            Arg::new("to")
                .short('t')
                .long("to")
                .required(true)
                .takes_value(true)
                .validator(hex_validator),
        )
        .arg(
            Arg::new("data")
                .short('d')
                .long("data")
                .required(true)
                .takes_value(true)
                .validator(hex_validator),
        );

    let send = App::new("send")
        .about("Send transaction")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("to")
                .short('t')
                .long("to")
                .required(true)
                .takes_value(true)
                .validator(hex_validator),
        )
        .arg(
            Arg::new("data")
                .short('d')
                .long("data")
                .required(true)
                .takes_value(true)
                .validator(hex_validator),
        );

    let block_number = App::new("block_number")
        .about("Get block number")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("for_pending").short('p').long("for_pending"));

    let block_at = App::new("block_at")
        .about("Get block by number")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("block_number")
                .required(true)
                .validator(hex_validator),
        );

    let get_tx = App::new("get_tx")
        .about("Get transaction by hash")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("tx_hash").required(true).validator(hex_validator));

    let peer_count = App::new("peer_count").about("Get peer count");

    let system_config = App::new("system_config")
        .about("Get system config")
        .setting(AppSettings::ColoredHelp);

    let bench = App::new("bench")
        .about("Send multiple txs with random content")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("count")
                .about("How many txs to send in decimal")
                .required(false)
                .default_value("1024")
                .validator(str::parse::<u64>),
        );

    #[cfg(feature = "evm")]
    let create = App::new("create")
        .about("Create contract")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("data").required(true).validator(hex_validator));

    #[cfg(feature = "evm")]
    let receipt = App::new("receipt")
        .about("Get receipt by tx_hash")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("tx_hash").required(true).validator(hex_validator));

    #[cfg(feature = "evm")]
    let get_code = App::new("get_code")
        .about("Get code by contract address")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("addr").required(true).validator(hex_validator));

    #[cfg(feature = "evm")]
    let get_balance = App::new("get_balance")
        .about("Get balance by account address")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("addr").required(true).validator(hex_validator));

    // addrs args
    let rpc_addr_arg = Arg::new("rpc_addr")
        .short('r')
        .long("rpc_addr")
        .takes_value(true);
    let executor_addr_arg = Arg::new("executor_addr")
        .short('e')
        .long("executor_addr")
        .takes_value(true);

    // main command
    let cli_app = App::new("cloud-cli")
        .about("The command line interface to interact with `CITA-CLOUD`.")
        .version("0.1.0")
        .setting(AppSettings::ColoredHelp)
        .arg(rpc_addr_arg)
        .arg(executor_addr_arg)
        .subcommands(vec![
            call,
            send,
            create,
            block_number,
            block_at,
            get_tx,
            peer_count,
            system_config,
            bench,
        ]);

    #[cfg(feature = "evm")]
    let cli_app = cli_app
        .subcommand(receipt)
        .subcommand(get_code)
        .subcommand(get_balance);

    let matches = cli_app.get_matches();

    let controller_addr = matches
        .value_of("controller_addr")
        .map(str::to_string)
        .unwrap_or_else(|| {
            if let Ok(controller_addr) = std::env::var("CITA_CLOUD_CONTROLLER_ADDR") {
                controller_addr
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

    let client = Arc::new(Client::new(&controller_addr, &executor_addr));

    if let Some(subcmd) = matches.subcommand() {
        match subcmd {
            ("call", m) => {
                let from = {
                    if let Ok(s) = m.value_of_t::<String>("from") {
                        hex::decode(&parse_addr(&s)).unwrap()
                    } else {
                        vec![0u8; 20]
                    }
                };
                let to = {
                    let s: String = m.value_of_t_or_exit("to");
                    hex::decode(&parse_addr(&s)).unwrap()
                };
                let data = {
                    let s: String = m.value_of_t_or_exit("data");
                    hex::decode(&remove_0x(&s)).unwrap()
                };

                let result = client.call(from, to, data).await;
                println!("result: 0x{}", hex::encode(&result));
            }
            ("send", m) => {
                let to = {
                    let s: String = m.value_of_t_or_exit("to");
                    hex::decode(parse_addr(&s)).unwrap()
                };
                let data = {
                    let s: String = m.value_of_t_or_exit("data");
                    hex::decode(remove_0x(&s)).unwrap()
                };

                let tx_hash = client.send(to, data).await;
                println!("tx_hash: 0x{}", hex::encode(&tx_hash));
            }
            ("block_number", m) => {
                let for_pending = m.is_present("for_pending");

                let block_number = client.get_block_number(for_pending).await;
                println!("block_number: {}", block_number);
            }
            ("block_at", m) => {
                let block_number = {
                    let s: String = m.value_of_t_or_exit("block_number");
                    s.parse::<u64>().unwrap()
                };

                let block = client.get_block_by_number(block_number).await;
                println!("{}", block.display());
            }
            ("get_tx", m) => {
                let tx_hash = {
                    let s: String = m.value_of_t_or_exit("tx_hash");
                    hex::decode(remove_0x(&s)).unwrap()
                };

                let tx = client.get_tx(tx_hash).await;
                println!("tx: {}", tx.display());
            }
            ("peer_count", _m) => {
                let cnt = client.get_peer_count().await;
                println!("peer_count: {}", cnt);
            }
            ("system_config", _m) => {
                let system_config = client.get_system_config().await;
                println!("{}", system_config.display());
            }
            ("bench", m) => {
                let tx_count = {
                    let s: String = m.value_of_t_or_exit("count");
                    s.parse::<u64>().unwrap()
                };

                let mut start_at = client.get_block_number(false).await;

                let mut rng = thread_rng();
                let handles = (0..tx_count)
                    .map(|_| {
                        let client = Arc::clone(&client);

                        let to: [u8; 20] = rng.gen();
                        let data: [u8; 32] = rng.gen();
                        tokio::spawn(async move { client.send(to.into(), data.into()).await })
                    })
                    .collect::<Vec<_>>();
                join_all(handles).await;

                println!("sending txs done.");

                let mut check_interval = tokio::time::interval(Duration::from_secs(1));
                let mut finalized_tx = 0;
                let mut begin_time = None;

                while finalized_tx < tx_count {
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
                        let handles = (start_at..=end_at)
                            .map(|n| {
                                let client = Arc::clone(&client);
                                tokio::spawn(async move { client.get_block_by_number(n).await })
                            })
                            .collect::<Vec<_>>();
                        join_all(handles).await
                    };

                    for b in blocks {
                        let b = b.unwrap();
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
            #[cfg(feature = "evm")]
            ("create", m) => {
                let to = vec![];
                let data = {
                    let s: String = m.value_of_t_or_exit("data");
                    hex::decode(remove_0x(&s)).unwrap()
                };

                let tx_hash = client.send(to, data).await;
                println!("tx_hash: 0x{}", hex::encode(&tx_hash));
            }
            #[cfg(feature = "evm")]
            ("receipt", m) => {
                let tx_hash = {
                    let s: String = m.value_of_t_or_exit("tx_hash");
                    hex::decode(remove_0x(&s)).unwrap()
                };

                let receipt = client.get_receipt(tx_hash).await;
                println!("{}", receipt.display());
            }
            #[cfg(feature = "evm")]
            ("get_code", m) => {
                let addr = {
                    let s: String = m.value_of_t_or_exit("addr");
                    hex::decode(remove_0x(&s)).unwrap()
                };

                let code = client.get_code(addr).await;
                println!("code: 0x{}", hex::encode(&code.byte_code));
            }
            #[cfg(feature = "evm")]
            ("get_balance", m) => {
                let addr = {
                    let s: String = m.value_of_t_or_exit("addr");
                    hex::decode(remove_0x(&s)).unwrap()
                };

                let balance = client.get_balance(addr).await;
                println!("balance: 0x{}", hex::encode(&balance.value));
            }
            _ => {
                unreachable!()
            }
        }
    } else {
        Interactive::run()
    }
}

fn parse_addr(s: &str) -> String {
    // padding 0 to 20 bytes
    format!("{:0>40}", remove_0x(s))
}

fn remove_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}
