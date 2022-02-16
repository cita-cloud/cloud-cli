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
};

use crate::crypto::Crypto;
use crate::sdk::{
    account::AccountBehaviour, admin::AdminBehaviour, controller::ControllerBehaviour,
    evm::EvmBehaviour, evm::EvmBehaviourExt, executor::ExecutorBehaviour, wallet::WalletBehaviour,
    controller::ControllerClient, executor::ExecutorClient, evm::EvmClient,
    wallet::Wallet
};


pub fn get_cmd<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
    Ev: EvmBehaviour<C>,
{
    Command::new("get")
        .about("Get chain info")
        .subcommands([
            evm::get_contract_abi().name("abi"),
            evm::get_balance().name("balance").alias("ba"),
            rpc::get_block().name("block").alias("b"),
            evm::get_code().name("code"),
            // TODO: get index
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

pub fn cldi_cmd<'help, C: Crypto>() -> Command<'help, ControllerClient, ExecutorClient, EvmClient, Wallet<C>>
{
    Command::new("cldi")
        .about("The command line interface to interact with `CITA-Cloud v6.3.0`.")
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
        .handler(|cmd, m, ctx| {
            ctx.rt.block_on(async {
                if let Some(controller_addr) = m.value_of("controller-addr") {
                    let controller = {
                        let addr = format!("http://{controller_addr}");
                        let channel = Endpoint::from_shared(addr)?.connect_lazy();
                        ControllerClient::new(channel)
                    };
                    ctx.controller = controller;
                }

                if let Some(executor_addr) = m.value_of("executor-addr") {
                    let executor = {
                        let addr = format!("http://{executor_addr}");
                        let channel = Endpoint::from_shared(addr)?.connect_lazy();
                        ExecutorClient::new(channel)
                    };

                    let evm = {
                        // the same addr
                        let addr = format!("http://{executor_addr}");
                        let channel = Endpoint::from_shared(addr).unwrap().connect_lazy();
                        EvmClient::new(channel)
                    };

                    ctx.executor = executor;
                    ctx.evm = evm;
                }
                anyhow::Ok(())
            })??;

            cmd.dispatch_subcmd(m, ctx)
        })
        .subcommands([
            admin::admin_cmd(),
            key::key_cmd(),
            // TODO: figure out why it cannot infer C.
            self::get_cmd::<C, _, _, _, _>(),
            // evm::store_contract_abi(),
            // rpc::add_node::<C, _, _, _, _>(),
            evm::evm_cmd(),
            rpc::rpc_cmd::<C, _, _, _, _>(),
            rpc::send(),
            rpc::call::<C, _, _, _, _>(),
            bench::bench_send().name("bench"),
        ])
}
