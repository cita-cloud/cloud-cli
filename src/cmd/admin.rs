use clap::App;
use clap::Arg;

use crate::wallet::Account;
use crate::context::Context;
use crate::utils::{parse_addr, parse_data, parse_value};

use prost::Message;
use crate::crypto::Crypto;

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

use crate::crypto::{ hash_data, sign_message };

use crate::utils::hex;
use crate::cmd::controller::ControllerBehaviour;
// use crate::types::{ Hash, Address };
use super::{ Command, CommandHandler };

use anyhow::Result;


/// CITA-Cloud's system config is managed in [UTXO](https://github.com/cita-cloud/rfcs/blob/master/rfcs/0002-technology/0002-technology.md#%E7%B3%BB%E7%BB%9F%E9%85%8D%E7%BD%AE).
/// Admin commands depend on and will change system config.
/// Make sure the system config is up-to-date before issues any admin commands.
#[tonic::async_trait]
pub trait AdminBehaviour {
    type Hash;
    type Address;

    // TODO: maybe we can use some concrete error types that allows user to handle them better.
    async fn set_block_interval(&self, block_interval: u32) -> Result<Self::Hash>;
    async fn emergency_brake(&self, switch: bool) -> Result<Self::Hash>;
    async fn update_admin(&self, admin: Self::Address) -> Result<Self::Hash>;
    async fn update_validators(&self, validators: &[Self::Address]) -> Result<Self::Hash>;
}

#[tonic::async_trait]
impl<C: Crypto> AdminBehaviour for Context<C> {
    type Hash = <C as Crypto>::Hash;
    type Address = <C as Crypto>::Address;

    async fn set_block_interval(&self, block_interval: u32) -> Result<Self::Hash> {
        let output = block_interval.to_be_bytes().to_vec();
        let utxo = CloudUtxoTransaction {
            version: self.system_config.version,
            pre_tx_hash: self.system_config.block_interval_pre_hash.clone(),
            output,
            lock_id: 1003,
        };

        self.send_utxo(utxo).await.context("failed to send `set_block_interval` utxo")
    }

    async fn emergency_brake(&self, switch: bool) -> Result<Self::Hash> {
        let output = if switch { vec![0] } else { vec![] };
        let utxo = CloudUtxoTransaction {
            version: self.system_config.version,
            pre_tx_hash: self.system_config.emergency_brake_pre_hash.clone(),
            output,
            lock_id: 1005,
        };

        self.send_utxo(utxo).await.context("failed to send `emergency_brake` utxo")
    }

    async fn update_admin(&self, admin: Self::Address) -> Result<Self::Hash> {
        let output = admin.as_ref().to_vec();
        let utxo = CloudUtxoTransaction {
            version: self.system_config.version,
            pre_tx_hash: self.system_config.admin_pre_hash.clone(),
            output,
            lock_id: 1002,
        };
        self.send_utxo(utxo).await.context("failed to send `update_admin` utxo")
    }

    async fn update_validators(&self, validators: &[Address]) -> Result<Self::Hash> {
        let output = validators.concat();
        let utxo = CloudUtxoTransaction {
            version: self.system_config.version,
            pre_tx_hash: self.system_config.validators_pre_hash.clone(),
            output,
            lock_id: 1004,
        };

        self.send_utxo(utxo).await.context("failed to send `update_validators` utxo")
    }
}

mod cmd {
    use super::*;

    pub fn update_admin<C>() -> Command<C> {
        let app = App::new("update-admin")
            .about("Update admin of the chain")
            .arg(
                Arg::new("admin")
                    .help("the address of the new admin")
                    .required(true)
                    .validator(parse_addr)
            );
        Command::new(app)
            .handler(|ctx, m| {
                let admin = parse_addr(m.value_of("admin").unwrap())?;
                let tx_hash = ctx.rt.block_on(ctx.update_admin(admin))?;
                println!("tx_hash: {}", hex(&tx_hash));
                Ok(())
            })
    }

    pub fn update_validators<C>() -> Command<C> {
        let app = App::new("update-validators")
            .about("Update validators of the chain")
            .arg(
                Arg::new("validators")
                    .help("a space-separated list of the new validator addresses, e.g. `cldi update-validators 0x12..34 0xab..cd`")
                    .required(true)
                    .multiple_values(true)
                    .validator(parse_addr)
            );
        Command::new(app)
            .handler(|ctx, m| {
                let validators = m
                    .values_of("validators")
                    .unwrap()
                    .map(parse_addr)
                    .collect::<Result<Vec<_>>>()?;
                let tx_hash = ctx.rt.block_on(ctx.update_validators(&validators))?;
                println!("tx_hash: {}", hex(&tx_hash));
                Ok(())
            })
    }

    pub fn set_block_interval<C>() -> Command<C> {
        let app = App::new("set-block-interval")
            .about("Set block interval")
            .arg(
                Arg::new("block_interval")
                    .help("new block interval")
                    .required(true)
                    .validator(str::parse::<u32>),
            );
        Command::new(app)
            .handler(|ctx, m| {
                let block_interval = m.value_of("block_interval").unwrap().parse::<u32>()?;
                let tx_hash = ctx.rt.block_on(ctx.set_block_interval(block_interval))?;
                println!("tx_hash: {}", hex(&tx_hash));
                Ok(())
            })
    }

    pub fn emergency_brake<C>() -> Command<C> {
        let app = App::new("emergency-brake")
            .about("Send emergency brake cmd to chain")
            .arg(
                Arg::new("switch")
                    .help("turn on/off")
                    .required(true)
                    .possible_values(&["on", "off"]),
            );
        Command::new(app)
            .handler(|ctx, m| {
                let switch = m.value_of("switch").unwrap() == "on";
                let tx_hash = ctx.rt.block_on(ctx.emergency_brake(switch))?;
                println!("tx_hash: {}", hex(&tx_hash));
                Ok(())
            })
    }
}
