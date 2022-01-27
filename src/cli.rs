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

use clap::App;
use clap::Arg;

use anyhow::Context;

use crate::utils::{parse_addr, parse_data, parse_value};

pub fn build_cli() -> App<'static> {
    // subcommands
    let call = App::new("call")
        .about("Executor call")
        .arg(
            Arg::new("from")
                .short('f')
                .long("from")
                .required(false)
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
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        );

    let send = App::new("send")
        .about("Send transaction")
        .arg(
            Arg::new("to")
                .help("the address to send")
                .short('t')
                .long("to")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(
            Arg::new("value")
                .help("the value to send")
                .short('v')
                .long("value")
                .required(false)
                .takes_value(true)
                .validator(parse_value),
        )
        .arg(
            Arg::new("data")
                .help("the data of the tx")
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        );

    let block_number = App::new("block-number").about("Get block number").arg(
        Arg::new("for_pending")
            .help("get the block number of pending block")
            .short('p')
            .long("for_pending"),
    );

    let get_block = App::new("get-block")
        .about("Get block by block number(height) or hash")
        .arg(
            Arg::new("number_or_hash")
                .help("plain decimal number or hash with `0x` prefix")
                .required(true)
                .takes_value(true)
                .validator(|s| {
                    if s.starts_with("0x") {
                        parse_value(s)?;
                    } else {
                        s.parse::<u64>().context("cannot parse block number, if you want to get block by hash, please prefix it with `0x`")?;
                    }
                    Ok::<(), anyhow::Error>(())
                })
        );

    let get_block_hash = App::new("block-hash")
        .about("Get block hash by block number(height)")
        .arg(
            Arg::new("number")
                .help("the block number(height)")
                .takes_value(true)
                .validator(str::parse::<u64>),
        );

    let get_tx = App::new("get-tx")
        .about("Get transaction by hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_value));

    let get_tx_index = App::new("get-tx-index")
        .about("Get transaction's index by tx_hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_value));

    let get_tx_block_number = App::new("get-tx-block-number")
        .about("Get transaction's block number by tx_hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_value));

    let peer_count = App::new("peer-count").about("Get peer count");

    let add_node = App::new("add-node")
        .about("Add node")
        .arg(Arg::new("multi_address").required(true));

    let peers_info = App::new("peers-info").about("Get peers info");

    let system_config = App::new("system-config").about("Get system config");

    let bench = App::new("bench")
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
                .help("Timeout for each request (in seconds). Use 0 for infinite")
                .long("timeout")
                .takes_value(true)
                .default_value("120")
                .validator(str::parse::<u64>),
        )
        .arg(
            Arg::new("total")
                .help("Number of transactions to send")
                .default_value("200")
                .validator(str::parse::<u32>),
        );

    let account = build_account_subcmd();

    let completions = App::new("completions")
        .about("Generate completions for current shell. Add the output script to `.profile` or `.bashrc` etc. to make it effective.")
        .arg(
            Arg::new("shell")
                .required(true)
                .possible_values(&[
                    "bash",
                    "zsh",
                    "powershell",
                    "fish",
                    "elvish",
                ])
                .validator(|s| s.parse::<clap_complete::Shell>()),
        );

    let update_admin = App::new("update-admin")
        .about("Update admin of the chain")
        .arg(
            Arg::new("admin_addr")
                .help("the address of the new admin")
                .required(true)
                .validator(parse_addr),
        );

    let update_validators = App::new("update-validators")
        .about("Update validators of the chain")
        .arg(
            Arg::new("validators")
                .help("a space-separated list of the new validator addresses, e.g. `cldi update-validators 0x12..34 0xab..cd`")
                .required(true)
                .multiple_values(true)
                .validator(parse_addr),
        );

    let set_block_interval = App::new("set-block-interval")
        .about("Set block interval")
        .arg(
            Arg::new("block_interval")
                .help("new block interval")
                .required(true)
                .validator(str::parse::<u64>),
        );

    let emergency_brake = App::new("emergency-brake")
        .about("Send emergency brake cmd to chain")
        .arg(
            Arg::new("switch")
                .help("turn on/off")
                .required(true)
                .possible_values(&["on", "off"]),
        );

    #[cfg(feature = "evm")]
    let create = App::new("create")
        .about("Create contract")
        .arg(
            Arg::new("value")
                .short('v')
                .long("value")
                .required(false)
                .takes_value(true)
                .validator(parse_value),
        )
        .arg(Arg::new("data").required(true).validator(parse_data));

    #[cfg(feature = "evm")]
    let receipt = App::new("receipt")
        .about("Get receipt by tx_hash")
        .arg(Arg::new("tx_hash").required(true).validator(parse_value));

    #[cfg(feature = "evm")]
    let get_code = App::new("get-code")
        .about("Get code by contract address")
        .arg(Arg::new("addr").required(true).validator(parse_addr));

    #[cfg(feature = "evm")]
    let get_balance = App::new("get-balance")
        .about("Get balance by account address")
        .arg(Arg::new("addr").required(true).validator(parse_addr));

    #[cfg(feature = "evm")]
    let get_tx_count = App::new("get-tx-count")
        .about("Get the transaction count of the address")
        .arg(Arg::new("addr").required(true).validator(parse_addr));

    #[cfg(feature = "evm")]
    let store_abi = App::new("store-abi")
        .about("Store contract ABI")
        .arg(
            Arg::new("addr")
                .short('a')
                .long("addr")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(Arg::new("abi").required(true).takes_value(true));

    #[cfg(feature = "evm")]
    let get_abi = App::new("get-abi").about("Get specific contract ABI").arg(
        Arg::new("addr")
            .required(true)
            .takes_value(true)
            .validator(parse_addr),
    );

    let user_arg = Arg::new("user")
        .help("the user(account) to send tx")
        .short('u')
        .long("user")
        .takes_value(true);

    // addrs args
    let rpc_addr_arg = Arg::new("rpc_addr")
        .help("rpc(controller) address")
        .short('r')
        .long("rpc_addr")
        .takes_value(true);

    let executor_addr_arg = Arg::new("executor_addr")
        .help("executor address")
        .short('e')
        .long("executor_addr")
        .takes_value(true);

    // main command
    let cli_app = App::new("cloud-cli")
        .about("The command line interface to interact with `CITA-Cloud v6.3.0`.")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(user_arg)
        .arg(rpc_addr_arg)
        .arg(executor_addr_arg)
        .subcommands(vec![
            call,
            send,
            create,
            block_number,
            get_block,
            get_block_hash,
            get_tx,
            get_tx_block_number,
            get_tx_index,
            peer_count,
            add_node,
            peers_info,
            system_config,
            bench,
            account,
            completions,
            update_admin,
            update_validators,
            set_block_interval,
            emergency_brake,
        ]);

    #[cfg(feature = "evm")]
    let cli_app = cli_app.subcommands(vec![
        receipt,
        get_code,
        get_balance,
        store_abi,
        get_abi,
        get_tx_count,
    ]);

    cli_app
}

fn build_account_subcmd() -> App<'static> {
    let create = App::new("create")
        .about("Create an account")
        .arg(
            Arg::new("user")
                .help("The user name of the account")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("password")
                .help("The password of the account. (unimplemented yet)")
                .short('p')
                .long("password")
                .takes_value(true),
        );

    let login = App::new("login")
        .about("Login to use the user's account as default")
        .arg(
            Arg::new("user")
                .help("The user name to login")
                .takes_value(true)
                .required(true),
        );

    let import = App::new("import")
        .about("Import an account")
        .arg(
            Arg::new("user")
                .help("The user name for the incoming account")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("pk")
                .help("The public key of the incoming account")
                .short('p')
                .long("pk")
                .takes_value(true)
                .required(true)
                .validator(parse_data),
        )
        .arg(
            Arg::new("sk")
                .help("The secret key of the incoming account")
                .short('s')
                .long("sk")
                .takes_value(true)
                .required(true)
                .validator(parse_data),
        );

    let export = App::new("export").about("Export an account").arg(
        Arg::new("user")
            .help("The user name of the account to be exported")
            .takes_value(true)
            .required(true),
    );

    let delete = App::new("delete").about("Delete an account").arg(
        Arg::new("user")
            .help("The user name of the account to be deleted")
            .takes_value(true)
            .required(true),
    );

    App::new("account")
        .about("Manage account")
        .subcommands(vec![create, login, import, export, delete])
}
