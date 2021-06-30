use crate::util::{parse_addr, parse_data, parse_value};
use clap::App;
use clap::AppSettings;
use clap::Arg;

pub fn build_cli() -> App<'static> {
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
        .setting(AppSettings::ColoredHelp)
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
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("for_pending").short('p').long("for_pending"));

    let get_block = App::new("get-block")
        .about("Get block by number or hash")
        .setting(AppSettings::ColoredHelp)
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

    let get_tx = App::new("get-tx")
        .about("Get transaction by hash")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("tx_hash").required(true).validator(parse_value));

    let peer_count = App::new("peer-count").about("Get peer count");

    let system_config = App::new("system-config")
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

    let account = build_account_subcmd();

    let completions = App::new("completions")
        .about("Generate completions for current shell")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("shell").required(true).possible_values(&[
            "bash",
            "powershell",
            "zsh",
            "fish",
            "elvish",
        ]));

    let update_admin = App::new("update-admin")
        .about("Update admin of the chain")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("admin_addr")
                .about("the address of the new admin")
                .required(true)
                .validator(parse_addr),
        );

    let update_validators = App::new("update-validators")
        .about("Update validators of the chain")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("validators")
                .about("the new validator list")
                .required(true)
                .multiple(true)
                .validator(parse_addr),
        );

    let set_block_interval = App::new("set-block-interval")
        .about("Set block interval")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("block_interval")
                .about("new block interval")
                .required(true)
                .validator(str::parse::<u64>),
        );

    let emergency_brake = App::new("emergency-brake")
        .about("Send emergency brake cmd to chain")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("switch")
                .about("turn on/off")
                .required(true)
                .possible_values(&["on", "off"]),
        );

    #[cfg(feature = "evm")]
    let create = App::new("create")
        .about("Create contract")
        .setting(AppSettings::ColoredHelp)
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
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("tx_hash").required(true).validator(parse_value));

    #[cfg(feature = "evm")]
    let get_code = App::new("get-code")
        .about("Get code by contract address")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("addr").required(true).validator(parse_addr));

    #[cfg(feature = "evm")]
    let get_balance = App::new("get-balance")
        .about("Get balance by account address")
        .setting(AppSettings::ColoredHelp)
        .arg(Arg::new("addr").required(true).validator(parse_addr));

    #[cfg(feature = "evm")]
    let store_abi = App::new("store-abi")
        .about("Store abi")
        .setting(AppSettings::ColoredHelp)
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
    let get_abi = App::new("get-abi")
        .about("Get specific contract abi")
        .setting(AppSettings::ColoredHelp)
        .arg(
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
        .about("The command line interface to interact with `CITA-Cloud`.")
        .version("0.1.0")
        .setting(AppSettings::ColoredHelp)
        .arg(user_arg)
        .arg(rpc_addr_arg)
        .arg(executor_addr_arg)
        .subcommands(vec![
            call,
            send,
            create,
            block_number,
            get_block,
            get_tx,
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
    let cli_app = cli_app.subcommands(vec![receipt, get_code, get_balance, store_abi, get_abi]);

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
