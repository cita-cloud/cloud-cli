use clap::App;
use clap::Arg;

use tonic::transport::Endpoint;

use super::Command;
use super::{
    admin,
    key,
    evm,
    rpc,
};

use crate::crypto::Crypto;
use crate::sdk::{
    account::AccountBehaviour, admin::AdminBehaviour, controller::ControllerBehaviour,
    evm::EvmBehaviour, evm::EvmBehaviourExt, executor::ExecutorBehaviour, wallet::WalletBehaviour,
    controller::ControllerClient, executor::ExecutorClient, evm::EvmClient,
    wallet::Wallet
};


pub fn get<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto,
    Co: ControllerBehaviour<C>,
{
    Command::new("get")
        .about("Get chain info")
}

pub fn cldi<'help, C: Crypto>() -> Command<'help, ControllerClient, ExecutorClient, EvmClient, Wallet<C>>
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
        .handler(|ctx, m| {
            let rt = ctx.rt.handle().clone();
            rt.block_on(async {
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
                        let addr = format!("http://{executor_addr}");
                        let channel = Endpoint::from_shared(addr).unwrap().connect_lazy();
                        EvmClient::new(channel)
                    };

                    ctx.executor = executor;
                    ctx.evm = evm;
                }
                anyhow::Ok(())
            })
        })
        .subcommands([
            admin::admin_cmd(),
            key::key_cmd(),
            // rpc::rpc_cmd::<C, _, _, _, _>(),
            // evm::evm_cmd(),
        ])
}
