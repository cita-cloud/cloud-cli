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

#![allow(clippy::let_and_return)]

use anyhow::Context;
use anyhow::Result;

use prost::Message;
use tonic::transport::Channel;

use crate::config::CryptoType;
use crate::crypto::{ArrayLike, Hash};
use crate::utils::recover_validators;
use bincode::deserialize;
use cita_cloud_proto::{
    blockchain::{
        raw_transaction::Tx, Block, CompactBlock, RawTransaction,
        Transaction as CloudNormalTransaction, UnverifiedTransaction, UnverifiedUtxoTransaction,
        UtxoTransaction as CloudUtxoTransaction, Witness,
    },
    common::{Empty, Hash as CloudHash, NodeNetInfo, Proof, StateRoot, TotalNodeInfo},
    controller::{BlockNumber, Flag, SystemConfig},
};
use consensus_bft::message::LeaderVote as BftProof;
use overlord::types::Proof as OverlordProof;
use rlp::Decodable;
use rlp::Rlp;

pub type ControllerClient =
    cita_cloud_proto::controller::rpc_service_client::RpcServiceClient<Channel>;

pub struct CompactBlockWithStaterootProof {
    pub compact_block: CompactBlock,
    pub state_root: StateRoot,
    pub proof: Proof,
}

pub enum ProofType {
    BftProof(BftProof),
    OverlordProof(OverlordProof),
}

pub struct ProofWithValidators {
    pub proof: ProofType,
    pub validators: Vec<Vec<u8>>,
}

#[tonic::async_trait]
pub trait ControllerBehaviour {
    // TODO: should I use the protobuf type instead of concrete type? e.g. u64 -> BlockNumber

    async fn send_raw(&self, raw: RawTransaction) -> Result<Hash>;

    async fn get_version(&self) -> Result<String>;
    async fn get_system_config(&self) -> Result<SystemConfig>;
    async fn get_system_config_by_number(&self, block_number: u64) -> Result<SystemConfig>;

    async fn get_block_number(&self, for_pending: bool) -> Result<u64>;
    async fn get_block_hash(&self, block_number: u64) -> Result<Hash>;

    async fn get_height_by_hash(&self, hash: Hash) -> Result<BlockNumber>;
    async fn get_block_by_number(
        &self,
        block_number: u64,
    ) -> Result<CompactBlockWithStaterootProof>;
    async fn get_block_detail_by_number(&self, block_number: u64) -> Result<Block>;

    async fn get_tx(&self, tx_hash: Hash) -> Result<RawTransaction>;
    async fn get_tx_index(&self, tx_hash: Hash) -> Result<u64>;
    async fn get_tx_block_number(&self, tx_hash: Hash) -> Result<u64>;

    async fn get_peer_count(&self) -> Result<u64>;
    async fn get_peers_info(&self) -> Result<TotalNodeInfo>;

    async fn add_node(&self, multiaddr: String) -> Result<u32>;
    async fn parse_bft_proof(
        &self,
        proof_bytes: Vec<u8>,
        crypto_type: CryptoType,
    ) -> Result<ProofWithValidators>;
    async fn parse_overlord_proof(&self, proof_bytes: Vec<u8>) -> Result<ProofWithValidators>;
}

#[tonic::async_trait]
impl ControllerBehaviour for ControllerClient {
    async fn send_raw(&self, raw: RawTransaction) -> Result<Hash> {
        let resp = self.clone().send_raw_transaction(raw).await?.into_inner();

        Hash::try_from_slice(&resp.hash)
            .context("controller returns an invalid transaction hash, maybe we are using a wrong signing algorithm?")
    }

    async fn get_version(&self) -> Result<String> {
        let version = ControllerClient::get_version(&mut self.clone(), Empty {})
            .await?
            .into_inner()
            .version;

        Ok(version)
    }

    async fn get_system_config(&self) -> Result<SystemConfig> {
        let resp = ControllerClient::get_system_config(&mut self.clone(), Empty {})
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_system_config_by_number(&self, block_number: u64) -> Result<SystemConfig> {
        let block_number = BlockNumber { block_number };
        let resp = ControllerClient::get_system_config_by_number(&mut self.clone(), block_number)
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_block_number(&self, for_pending: bool) -> Result<u64> {
        let flag = Flag { flag: for_pending };
        let resp = ControllerClient::get_block_number(&mut self.clone(), flag)
            .await?
            .into_inner();

        Ok(resp.block_number)
    }

    async fn get_block_hash(&self, block_number: u64) -> Result<Hash> {
        let block_number = BlockNumber { block_number };
        let resp = ControllerClient::get_block_hash(&mut self.clone(), block_number)
            .await?
            .into_inner();

        Hash::try_from_slice(&resp.hash)
            .context("controller returns an invalid block hash, maybe we are using a different signing algorithm?")
    }

    async fn get_height_by_hash(&self, hash: Hash) -> Result<BlockNumber> {
        let hash = CloudHash {
            hash: hash.to_vec(),
        };
        let resp = ControllerClient::get_height_by_hash(&mut self.clone(), hash)
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_block_by_number(
        &self,
        block_number: u64,
    ) -> Result<CompactBlockWithStaterootProof> {
        let block_number = BlockNumber { block_number };
        let compact_block =
            ControllerClient::get_block_by_number(&mut self.clone(), block_number.clone())
                .await?
                .into_inner();
        let proof = ControllerClient::get_proof_by_number(&mut self.clone(), block_number.clone())
            .await?
            .into_inner();
        let state_root =
            ControllerClient::get_state_root_by_number(&mut self.clone(), block_number)
                .await?
                .into_inner();
        Ok(CompactBlockWithStaterootProof {
            compact_block,
            state_root,
            proof,
        })
    }

    async fn get_block_detail_by_number(&self, block_number: u64) -> Result<Block> {
        let block_number = BlockNumber { block_number };
        let resp = ControllerClient::get_block_detail_by_number(&mut self.clone(), block_number)
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_tx(&self, tx_hash: Hash) -> Result<RawTransaction> {
        let resp = self
            .clone()
            .get_transaction(CloudHash {
                hash: tx_hash.to_vec(),
            })
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_tx_index(&self, tx_hash: Hash) -> Result<u64> {
        let resp = self
            .clone()
            .get_transaction_index(CloudHash {
                hash: tx_hash.to_vec(),
            })
            .await?
            .into_inner();

        Ok(resp.tx_index)
    }

    async fn get_tx_block_number(&self, tx_hash: Hash) -> Result<u64> {
        let resp = self
            .clone()
            .get_transaction_block_number(CloudHash {
                hash: tx_hash.to_vec(),
            })
            .await?
            .into_inner();

        Ok(resp.block_number)
    }

    async fn get_peer_count(&self) -> Result<u64> {
        let resp = ControllerClient::get_peer_count(&mut self.clone(), Empty {})
            .await?
            .into_inner();

        Ok(resp.peer_count)
    }

    async fn get_peers_info(&self) -> Result<TotalNodeInfo> {
        let resp = ControllerClient::get_peers_info(&mut self.clone(), Empty {})
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn add_node(&self, multiaddr: String) -> Result<u32> {
        let node_info = NodeNetInfo {
            multi_address: multiaddr,
            ..Default::default()
        };
        let resp = ControllerClient::add_node(&mut self.clone(), node_info)
            .await?
            .into_inner();

        Ok(resp.code)
    }

    async fn parse_bft_proof(
        &self,
        proof_bytes: Vec<u8>,
        crypto_type: CryptoType,
    ) -> Result<ProofWithValidators> {
        let bft_proof: BftProof = deserialize(&proof_bytes).unwrap_or_default();
        let validators = recover_validators(crypto_type, bft_proof.clone());
        Ok(ProofWithValidators {
            proof: ProofType::BftProof(bft_proof),
            validators,
        })
    }

    async fn parse_overlord_proof(&self, proof_bytes: Vec<u8>) -> Result<ProofWithValidators> {
        let overlord_proof: OverlordProof = OverlordProof::decode(&Rlp::new(&proof_bytes)).unwrap();
        let validators: Vec<Vec<u8>> = self
            .get_system_config_by_number(overlord_proof.height)
            .await
            .map_or_else(|_| vec![], |v| v.validators);
        Ok(ProofWithValidators {
            proof: ProofType::OverlordProof(overlord_proof),
            validators,
        })
    }
}

pub trait SignerBehaviour {
    fn hash(&self, msg: &[u8]) -> Vec<u8>;
    fn address(&self) -> &[u8];
    fn sign(&self, msg: &[u8]) -> Vec<u8>;

    fn sign_raw_tx(&self, tx: CloudNormalTransaction) -> RawTransaction {
        // calc tx hash
        let tx_hash = {
            // build tx bytes
            let tx_bytes = {
                let mut buf = Vec::with_capacity(tx.encoded_len());
                tx.encode(&mut buf).unwrap();
                buf
            };
            self.hash(tx_bytes.as_slice())
        };

        // sign tx hash
        let sender = self.address().to_vec();
        let signature = self.sign(tx_hash.as_slice()).to_vec();

        // build raw tx
        let raw_tx = {
            let witness = Witness { sender, signature };

            let unverified_tx = UnverifiedTransaction {
                transaction: Some(tx),
                transaction_hash: tx_hash.to_vec(),
                witness: Some(witness),
            };

            RawTransaction {
                tx: Some(Tx::NormalTx(unverified_tx)),
            }
        };

        raw_tx
    }

    fn sign_raw_utxo(&self, utxo: CloudUtxoTransaction) -> RawTransaction {
        // calc utxo hash
        let utxo_hash = {
            // build utxo bytes
            let utxo_bytes = {
                let mut buf = Vec::with_capacity(utxo.encoded_len());
                utxo.encode(&mut buf).unwrap();
                buf
            };
            self.hash(utxo_bytes.as_slice())
        };

        // sign utxo hash
        let sender = self.address().to_vec();
        let signature = self.sign(utxo_hash.as_slice()).to_vec();

        // build raw utxo
        let raw_utxo = {
            let witness = Witness { sender, signature };

            let unverified_utxo = UnverifiedUtxoTransaction {
                transaction: Some(utxo),
                transaction_hash: utxo_hash.to_vec(),
                witnesses: vec![witness],
            };

            RawTransaction {
                tx: Some(Tx::UtxoTx(unverified_utxo)),
            }
        };

        raw_utxo
    }
}

// It's actually the implementation details of the current controller service.
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum UtxoType {
    Admin = 1002,
    BlockInterval = 1003,
    Validators = 1004,
    EmergencyBrake = 1005,
    QuotaLimit = 1007,
}

#[tonic::async_trait]
pub trait TransactionSenderBehaviour {
    async fn send_raw_tx<S>(&self, signer: &S, raw_tx: CloudNormalTransaction) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
    async fn send_raw_utxo<S>(&self, signer: &S, raw_utxo: CloudUtxoTransaction) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;

    async fn send_tx<S>(
        &self,
        signer: &S,
        // Use Vec<u8> instead of Address to allow empty address for creating contract
        to: Vec<u8>,
        data: Vec<u8>,
        value: Vec<u8>,
        quota: u64,
        valid_until_block: u64,
    ) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
    async fn send_utxo<S>(&self, signer: &S, output: Vec<u8>, utxo_type: UtxoType) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
}

#[tonic::async_trait]
impl<T> TransactionSenderBehaviour for T
where
    T: ControllerBehaviour + Send + Sync,
{
    async fn send_raw_tx<S>(&self, signer: &S, raw_tx: CloudNormalTransaction) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let raw = signer.sign_raw_tx(raw_tx);
        self.send_raw(raw).await.context("failed to send raw")
    }

    async fn send_raw_utxo<S>(&self, signer: &S, raw_utxo: CloudUtxoTransaction) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let raw = signer.sign_raw_utxo(raw_utxo);
        self.send_raw(raw).await.context("failed to send raw")
    }

    async fn send_tx<S>(
        &self,
        signer: &S,
        to: Vec<u8>,
        data: Vec<u8>,
        value: Vec<u8>,
        quota: u64,
        valid_until_block: u64,
    ) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let system_config = self
            .get_system_config()
            .await
            .context("failed to get system config")?;

        let raw_tx = CloudNormalTransaction {
            version: system_config.version,
            to,
            data,
            value,
            nonce: rand::random::<u64>().to_string(),
            quota,
            valid_until_block,
            chain_id: system_config.chain_id.clone(),
        };

        self.send_raw_tx(signer, raw_tx).await
    }

    async fn send_utxo<S>(&self, signer: &S, output: Vec<u8>, utxo_type: UtxoType) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let system_config = self
            .get_system_config()
            .await
            .context("failed to get system config")?;
        let raw_utxo = {
            let lock_id = utxo_type as u64;
            let pre_tx_hash = match utxo_type {
                UtxoType::Admin => &system_config.admin_pre_hash,
                UtxoType::BlockInterval => &system_config.block_interval_pre_hash,
                UtxoType::Validators => &system_config.validators_pre_hash,
                UtxoType::EmergencyBrake => &system_config.emergency_brake_pre_hash,
                UtxoType::QuotaLimit => &system_config.quota_limit_pre_hash,
            }
            .clone();

            CloudUtxoTransaction {
                version: system_config.version,
                pre_tx_hash,
                output,
                lock_id,
            }
        };

        self.send_raw_utxo(signer, raw_utxo).await
    }
}
