use clap::App;
use clap::Arg;

use crate::utils::{parse_addr, parse_data};

use prost::Message;
use super::*;
use crate::sdk::context::Context;


use crate::proto::{
    blockchain::{
        raw_transaction::Tx, CompactBlock, RawTransaction, Transaction as CloudTransaction,
        UnverifiedTransaction, UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction,
        Witness,
    },
    common::{Address, Empty, Hash, NodeInfo, NodeNetInfo},
    controller::{
        rpc_service_client::RpcServiceClient as ControllerClient, BlockNumber, Flag, SystemConfig,
        TransactionIndex,
    },
    evm::{
        rpc_service_client::RpcServiceClient as EvmClient, Balance, ByteAbi, ByteCode, Nonce,
        Receipt,
    },
    executor::{executor_service_client::ExecutorServiceClient as ExecutorClient, CallRequest},
};

use crate::crypto::{ArrayLike, Crypto};
use crate::utils::hex;
use crate::sdk::wallet::WalletBehaviour;


pub fn create_account<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: WalletBehaviour<C>
{
    Command::new("create-account")
        .about("create an account")
        .arg(
            Arg::new("id")
                .short('u')
                .help("The ID for the new generated account")
                .required(true)
                // TODO: add validator
        )
        .arg(
            Arg::new("password")
                .short('p')
                .help("The password to encrypt account")
                // TODO: add validator
        )
        .handler(|ctx, m| {
            let id = m.value_of("id").unwrap();
            let pw = m.value_of("pw");
            let acc = ctx.rt.handle().clone().block_on(async move {
                ctx.generate_account(id, pw).await?;
                ctx.get_account(id).await
            })?;

            let addr = hex(acc.address().as_slice());
            println!("Account `{id}` generated, address: {addr}");

            Ok(())
        })
}


pub fn account_cmd<'help, C, Co, Ex, Ev, Wa>() -> Command<'help, Co, Ex, Ev, Wa>
where
    C: Crypto + 'static,
    Context<Co, Ex, Ev, Wa>: WalletBehaviour<C>
{
    Command::new("account")
        .about("account commands")
        .subcommands([
            create_account(),
        ])
}

