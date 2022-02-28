use clap::App;
use clap::Arg;

use tonic::transport::Endpoint;

use super::Command;
use super::{
    admin,
    key,
    evm,
    rpc,
    bench,
    context,
};

use crate::crypto::Crypto;
use crate::core::client::GrpcClientBehaviour;
use crate::core::{
    admin::AdminBehaviour, controller::ControllerBehaviour,
    evm::EvmBehaviour, evm::EvmBehaviourExt, executor::ExecutorBehaviour,
    controller::ControllerClient, executor::ExecutorClient, evm::EvmClient,
    wallet::Wallet, context::Context,
};
use crate::config::ContextSetting;


pub fn get_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour,
    Ev: EvmBehaviour,
{
    Command::new("get")
        .about("Get chain info")
        .subcommands([
            evm::get_contract_abi().name("abi"),
            evm::get_balance().name("balance").alias("ba"),
            rpc::get_block().name("block").alias("b"),
            evm::get_code().name("code"),
            rpc::get_tx().name("tx"),
            rpc::get_peer_count().name("peer-count").alias("pc"),
            rpc::get_peers_info().name("peers-info").alias("pi"),
            evm::get_tx_count().name("nonce"),
            evm::get_receipt().name("receipt").alias("r"),
            rpc::get_version().name("version"),
            rpc::get_system_config().name("system-config").alias("sc"),
            rpc::get_block_hash().name("block-hash").alias("bh"),
            rpc::get_block_number().name("block-number").alias("bn"),
        ])
}

// pub fn with_completions_subcmd<'help, Co, Ex, Ev>(cmd: Command<'help, Co, Ex, Ev, Wa>) -> Command<'help, Context<Co, Ex, Ev>> {
//     let without_handler = || Command::new("completions")
//         .about("Generate completions for current shell. Add the output script to `.profile` or `.bashrc` etc. to make it effective.")
//         .arg(
//             Arg::new("shell")
//                 .required(true)
//                 .possible_values(&[
//                     "bash",
//                     "zsh",
//                     "powershell",
//                     "fish",
//                     "elvish",
//                 ])
//                 .validator(|s| s.parse::<clap_complete::Shell>()),
//         );
//     let cmd = cmd.subcommand(without_handler());
//     let completions_subcmd = without_handler()
//         .handler(|_cmd, m, ctx|{
//             let shell: clap_complete::Shell = m.value_of("shell").unwrap().parse().unwrap();
//             let mut stdout = std::io::stdout();
//             clap_complete::generate(shell, &mut cmd.get_clap_command().clone(), "cldi", &mut stdout);
//             Ok(())
//         });
    
//     cmd.subcommand(completions_subcmd)
// }

pub fn cldi_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + GrpcClientBehaviour + Clone + Send + Sync + 'static,
    Ex: ExecutorBehaviour + GrpcClientBehaviour,
    Ev: EvmBehaviour + GrpcClientBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("cldi")
        .about("The command line interface to interact with `CITA-Cloud v6.3.0`")
        .arg(
            Arg::new("controller-addr")
                .help("controller address")
                .short('r')
                .takes_value(true)
                // TODO: add validator
        )
        .arg(
            Arg::new("executor-addr")
                .help("executor address")
                .short('e')
                .takes_value(true)
                // TODO: add validator
        )
        .arg(
            Arg::new("account-id")
                .help("account id")
                .short('u')
                .takes_value(true)
                // TODO: add validator
        )
        .handler(|cmd, m, ctx| {
            // If a subcommand is passed, it's considered as a tmp context for that subcommand.
            // Otherwise modify the current context.
            let mut previous_setting: Option<ContextSetting> = None;
            let mut current_setting = ctx.current_setting.clone();

            let is_tmp_ctx = m.subcommand().is_some();
            if is_tmp_ctx {
                previous_setting.replace(current_setting.clone());
            }

            if let Some(controller_addr) = m.value_of("controller-addr") {
                current_setting.controller_addr = controller_addr.into();
            }
            if let Some(executor_addr) = m.value_of("executor-addr") {
                current_setting.executor_addr = executor_addr.into();
            }
            if let Some(account_id) = m.value_of("account-id") {
                current_setting.account_id = account_id.into();
            }

            ctx.switch_context(current_setting)?;
            let ret = cmd.dispatch_subcmd(m, ctx);
            if let Some(previous) = previous_setting {
                ctx.switch_context(previous).expect("cannot restore previous context");
            }

            ret
        })
        .subcommands([
            admin::admin_cmd(),
            key::key_cmd(),
            self::get_cmd(),
            context::context_cmd(),
            evm::evm_cmd(),
            rpc::rpc_cmd(),
            bench::bench_send().name("bench"),
            // re-export
            rpc::send(),
            rpc::call(),
        ])
}
