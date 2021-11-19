use clap::App;
use clap::Arg;

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
                .about("the address to send")
                .short('t')
                .long("to")
                .required(true)
                .takes_value(true)
                .validator(parse_addr),
        )
        .arg(
            Arg::new("value")
                .about("the value to send")
                .short('v')
                .long("value")
                .required(false)
                .takes_value(true)
                .validator(parse_value),
        )
        .arg(
            Arg::new("data")
                .about("the data of the tx")
                .required(true)
                .takes_value(true)
                .validator(parse_data),
        );

    let block_number = App::new("block-number")
        .about("Get block number")
        .arg(Arg::new("for_pending").short('p').long("for_pending"));

    let get_block = App::new("get-block")
        .about("Get block by number or hash")
        .arg(
            Arg::new("number")
                .about("the block number(height)")
                .long("number")
                .short('n')
                .required_unless_present("hash")
                .takes_value(true)
                .validator(str::parse::<u64>),
        )
        .arg(
            Arg::new("hash")
                .long("hash")
                .about("the block hash")
                .short('h')
                .required_unless_present("number")
                .takes_value(true)
                .validator(parse_value),
        );

    let get_block_hash = App::new("block-hash")
        .about("Get block hash by block number(height)")
        .arg(
            Arg::new("number")
                .about("the block number(height)")
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

    let system_config = App::new("system-config").about("Get system config");

    let bench = App::new("bench")
        .about("Send transactions with {-c} workers over {--connections} connections")
        .arg(
            Arg::new("concurrency")
                .about(
                    "Number of request workers to run concurrently for sending transactions. \
                    Workers will be distributed evenly among all the connections. \
                    [default: the same as total]",
                )
                .short('c')
                .long("concurrency")
                .takes_value(true)
                .required(false)
                .validator(str::parse::<u32>),
        )
        .arg(
            Arg::new("connections")
                .about("Number of connections connects to server")
                .long("connections")
                .takes_value(true)
                .default_value("16")
                .validator(str::parse::<u32>),
        )
        .arg(
            Arg::new("total")
                .about("Number of transactions to send")
                .default_value("200")
                .validator(str::parse::<u32>),
        );

    let account = build_account_subcmd();

    let completions = App::new("completions")
        .about("Generate completions for current shell")
        .arg(
            Arg::new("shell")
                .required(true)
                .validator(|s| s.parse::<clap_generate::Shell>()),
        );

    let update_admin = App::new("update-admin")
        .about("Update admin of the chain")
        .arg(
            Arg::new("admin_addr")
                .about("the address of the new admin")
                .required(true)
                .validator(parse_addr),
        );

    let update_validators = App::new("update-validators")
        .about("Update validators of the chain")
        .arg(
            Arg::new("validators")
                .about("a space-separated list of the new validator addresses, e.g. `cldi update-validators 0x12..34 0xab..cd`")
                .required(true)
                .multiple_occurrences(true)
                .validator(parse_addr),
        );

    let set_block_interval = App::new("set-block-interval")
        .about("Set block interval")
        .arg(
            Arg::new("block_interval")
                .about("new block interval")
                .required(true)
                .validator(str::parse::<u64>),
        );

    let emergency_brake = App::new("emergency-brake")
        .about("Send emergency brake cmd to chain")
        .arg(
            Arg::new("switch")
                .about("turn on/off")
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
        .about("Store abi")
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
    let get_abi = App::new("get-abi").about("Get specific contract abi").arg(
        Arg::new("addr")
            .required(true)
            .takes_value(true)
            .validator(parse_addr),
    );

    let user_arg = Arg::new("user")
        .about("the user(account) to send tx")
        .short('u')
        .long("user")
        .takes_value(true);

    // addrs args
    let rpc_addr_arg = Arg::new("rpc_addr")
        .about("rpc(controller) address")
        .short('r')
        .long("rpc_addr")
        .takes_value(true);

    let executor_addr_arg = Arg::new("executor_addr")
        .about("executor address")
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
                .about("The user name of the account")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("password")
                .about("The password of the account. (unimplemented yet)")
                .short('p')
                .long("password")
                .takes_value(true),
        );

    let login = App::new("login")
        .about("Login to use the user's account by default")
        .arg(
            Arg::new("user")
                .about("The user name to login")
                .takes_value(true)
                .required(true),
        );

    let import = App::new("import")
        .about("Import an account")
        .arg(
            Arg::new("user")
                .about("The user name for the incoming account")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("pk")
                .about("The public key of the incoming account")
                .short('p')
                .long("pk")
                .takes_value(true)
                .required(true)
                .validator(parse_data),
        )
        .arg(
            Arg::new("sk")
                .about("The secret key of the incoming account")
                .short('s')
                .long("sk")
                .takes_value(true)
                .required(true)
                .validator(parse_data),
        );

    let export = App::new("export").about("Export an account").arg(
        Arg::new("user")
            .about("The user name of the account to be exported")
            .takes_value(true)
            .required(true),
    );

    let delete = App::new("delete").about("Delete an account").arg(
        Arg::new("user")
            .about("The user name of the account to be deleted")
            .takes_value(true)
            .required(true),
    );

    App::new("account")
        .about("Manage account")
        .subcommands(vec![create, login, import, export, delete])
}
