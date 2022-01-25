use clap::App;
use clap::Arg;

use super::context::Context;
use crate::utils::{parse_addr, parse_data};

use crate::crypto::Crypto;
use prost::Message;

use crate::crypto::ArrayLike;

use super::controller::ControllerBehaviour;
use super::controller::UtxoTransactionSenderBehaviour;
use super::controller::UtxoType;
use crate::utils::hex;
// use super::controller::HasSystemConfig;
use super::account::AccountBehaviour;
// use crate::types::{ Hash, Address };

use anyhow::Context as _;
use anyhow::Result;

/// CITA-Cloud's system config is managed by [UTXO](https://github.com/cita-cloud/rfcs/blob/master/rfcs/0002-technology/0002-technology.md#%E7%B3%BB%E7%BB%9F%E9%85%8D%E7%BD%AE).
/// Admin commands depend on and will change system config.
/// Make sure the system config is up-to-date before issues any admin commands.
/// Otherwise it will fail.
#[tonic::async_trait]
pub trait AdminBehaviour<C: Crypto> {
    // TODO: maybe we can use some concrete error types that allows user to handle them better.
    async fn update_admin(&self, admin: C::Address) -> Result<C::Hash>;
    async fn set_block_interval(&self, block_interval: u32) -> Result<C::Hash>;
    async fn update_validators(&self, validators: &[C::Address]) -> Result<C::Hash>;
    async fn emergency_brake(&self, switch: bool) -> Result<C::Hash>;
}

#[tonic::async_trait]
impl<C, T> AdminBehaviour<C> for T
where
    C: Crypto,
    T: UtxoTransactionSenderBehaviour<C> + Send + Sync,
{
    // Those utxo output formats is defined by controller.

    async fn update_admin(&self, admin: C::Address) -> Result<C::Hash> {
        let output = admin.to_vec();
        self.send_utxo(output, UtxoType::Admin)
            .await
            .context("failed to send `update_admin` utxo")
    }

    async fn set_block_interval(&self, block_interval: u32) -> Result<C::Hash> {
        let output = block_interval.to_be_bytes().to_vec();
        self.send_utxo(output, UtxoType::BlockInterval)
            .await
            .context("failed to send `set_block_interval` utxo")
    }

    async fn update_validators(&self, validators: &[C::Address]) -> Result<C::Hash> {
        let output = {
            let mut output = vec![];
            validators
                .iter()
                .for_each(|v| output.extend_from_slice(v.as_slice()));
            output
        };

        self.send_utxo(output, UtxoType::Validators)
            .await
            .context("failed to send `update_validators` utxo")
    }

    async fn emergency_brake(&self, switch: bool) -> Result<C::Hash> {
        let output = if switch { vec![0] } else { vec![] };
        self.send_utxo(output, UtxoType::EmergencyBrake)
            .await
            .context("failed to send `emergency_brake` utxo")
    }
}
