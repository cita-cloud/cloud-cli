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

use super::controller::{SignerBehaviour, TransactionSenderBehaviour, UtxoType};
use crate::crypto::{Address, ArrayLike, Hash};
use anyhow::{Context, Result};

/// CITA-Cloud's system config is managed by [UTXO](https://github.com/cita-cloud/rfcs/blob/master/rfcs/0002-technology/0002-technology.md#%E7%B3%BB%E7%BB%9F%E9%85%8D%E7%BD%AE).
/// Admin commands depend on and will change system config.
#[tonic::async_trait]
pub trait AdminBehaviour {
    // TODO: maybe we can use some concrete error types that allows user to handle them better.
    async fn update_admin<S>(&self, old_admin_signer: &S, new_admin_addr: Address) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
    async fn set_block_interval<S>(&self, admin_signer: &S, block_interval: u32) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
    async fn update_validators<S>(&self, admin_signer: &S, validators: &[Vec<u8>]) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
    async fn emergency_brake<S>(&self, admin_signer: &S, switch: bool) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
    async fn set_quota_limit<S>(&self, admin_signer: &S, quota_limit: u64) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
    async fn set_block_limit<S>(&self, admin_signer: &S, block_limit: u64) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
}

#[tonic::async_trait]
impl<T> AdminBehaviour for T
where
    T: TransactionSenderBehaviour + Send + Sync,
{
    // Those utxo output formats are defined by controller.

    async fn update_admin<S>(&self, old_admin_signer: &S, new_admin_addr: Address) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let output = new_admin_addr.to_vec();
        self.send_utxo(old_admin_signer, output, UtxoType::Admin)
            .await
            .context("failed to send `update_admin` utxo")
    }

    async fn set_block_interval<S>(&self, admin_signer: &S, block_interval: u32) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let output = block_interval.to_be_bytes().to_vec();
        self.send_utxo(admin_signer, output, UtxoType::BlockInterval)
            .await
            .context("failed to send `set_block_interval` utxo")
    }

    async fn update_validators<S>(&self, admin_signer: &S, validators: &[Vec<u8>]) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let output = {
            let mut output = Vec::new();
            validators
                .iter()
                .for_each(|v| output.extend_from_slice(v.as_slice()));
            output
        };

        self.send_utxo(admin_signer, output, UtxoType::Validators)
            .await
            .context("failed to send `update_validators` utxo")
    }

    async fn emergency_brake<S>(&self, admin_signer: &S, switch: bool) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let output = if switch { vec![0] } else { Vec::new() };
        self.send_utxo(admin_signer, output, UtxoType::EmergencyBrake)
            .await
            .context("failed to send `emergency_brake` utxo")
    }

    async fn set_quota_limit<S>(&self, admin_signer: &S, quota_limit: u64) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let output = quota_limit.to_be_bytes().to_vec();
        self.send_utxo(admin_signer, output, UtxoType::QuotaLimit)
            .await
            .context("failed to send `set_quota_limit` utxo")
    }

    async fn set_block_limit<S>(&self, admin_signer: &S, block_limit: u64) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let output = block_limit.to_be_bytes().to_vec();
        self.send_utxo(admin_signer, output, UtxoType::BlockLimit)
            .await
            .context("failed to send `set_block_limit` utxo")
    }
}
