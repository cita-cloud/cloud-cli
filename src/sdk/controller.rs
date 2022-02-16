use prost::Message;
// use crate::context::Context;
// use crate::wallet::Account;
use super::account::AccountBehaviour;

use crate::proto::{
    blockchain::{
        raw_transaction::Tx, CompactBlock, RawTransaction, Transaction as CloudNormalTransaction,
        UnverifiedTransaction, UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction,
        Witness,
    },
    common::{Address, Empty, Hash, NodeInfo, NodeNetInfo, TotalNodeInfo},
    controller::{
        rpc_service_client::RpcServiceClient, BlockNumber, Flag, SystemConfig, TransactionIndex,
    },
};

use crate::crypto::{ArrayLike, Crypto};
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;

use tonic::transport::Channel;

pub type ControllerClient = crate::proto::controller::rpc_service_client::RpcServiceClient<Channel>;

#[tonic::async_trait]
pub trait ControllerBehaviour<C: Crypto> {
    // TODO: should I use the protobuf type instead of concrete type? e.g. u64 -> BlockNumber

    async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash>;

    async fn get_version(&self) -> Result<String>;
    async fn get_system_config(&self) -> Result<SystemConfig>;

    async fn get_block_number(&self, for_pending: bool) -> Result<u64>;
    async fn get_block_hash(&self, block_number: u64) -> Result<C::Hash>;

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock>;
    async fn get_block_by_hash(&self, hash: C::Hash) -> Result<CompactBlock>;

    async fn get_tx(&self, tx_hash: C::Hash) -> Result<RawTransaction>;
    async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<u64>;
    async fn get_tx_block_number(&self, tx_hash: C::Hash) -> Result<u64>;

    async fn get_peer_count(&self) -> Result<u64>;
    async fn get_peers_info(&self) -> Result<TotalNodeInfo>;

    async fn add_node(&self, multiaddr: String) -> Result<u32>;
}

#[tonic::async_trait]
impl<C: Crypto> ControllerBehaviour<C> for ControllerClient {
    async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash> {
        let resp = self.clone().send_raw_transaction(raw).await?.into_inner();

        C::Hash::try_from_slice(&resp.hash)
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

    async fn get_block_number(&self, for_pending: bool) -> Result<u64> {
        let flag = Flag { flag: for_pending };
        let resp = ControllerClient::get_block_number(&mut self.clone(), flag)
            .await?
            .into_inner();

        Ok(resp.block_number)
    }

    async fn get_block_hash(&self, block_number: u64) -> Result<C::Hash> {
        let block_number = BlockNumber { block_number };
        let resp = ControllerClient::get_block_hash(&mut self.clone(), block_number)
            .await?
            .into_inner();

        C::Hash::try_from_slice(&resp.hash)
            .context("controller returns an invalid block hash, maybe we are using a different signing algorithm?")
    }

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock> {
        let block_number = BlockNumber { block_number };
        let resp = ControllerClient::get_block_by_number(&mut self.clone(), block_number)
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_block_by_hash(&self, hash: C::Hash) -> Result<CompactBlock> {
        let hash = Hash {
            hash: hash.to_vec(),
        };
        let resp = ControllerClient::get_block_by_hash(&mut self.clone(), hash)
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_tx(&self, tx_hash: C::Hash) -> Result<RawTransaction> {
        let resp = self
            .clone()
            .get_transaction(Hash {
                hash: tx_hash.to_vec(),
            })
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<u64> {
        let resp = self
            .clone()
            .get_transaction_index(Hash {
                hash: tx_hash.to_vec(),
            })
            .await?
            .into_inner();

        Ok(resp.tx_index)
    }

    async fn get_tx_block_number(&self, tx_hash: C::Hash) -> Result<u64> {
        let resp = self
            .clone()
            .get_transaction_block_number(Hash {
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
}

pub trait SignerBehaviour<C: Crypto> {
    fn sign_raw_tx(&self, tx: CloudNormalTransaction) -> RawTransaction;
    fn sign_raw_utxo(&self, utxo: CloudUtxoTransaction) -> RawTransaction;
}

impl<C, A> SignerBehaviour<C> for A
where
    C: Crypto,
    A: AccountBehaviour<SigningAlgorithm = C>,
{
    fn sign_raw_tx(&self, tx: CloudNormalTransaction) -> RawTransaction {
        // calc tx hash
        let tx_hash = {
            // build tx bytes
            let tx_bytes = {
                let mut buf = Vec::with_capacity(tx.encoded_len());
                tx.encode(&mut buf).unwrap();
                buf
            };
            C::hash(tx_bytes.as_slice())
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
            C::hash(utxo_bytes.as_slice())
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
}

#[tonic::async_trait]
pub trait TransactionSenderBehaviour<C: Crypto> {
    async fn send_raw_tx<S>(&self, signer: &S, raw_tx: CloudNormalTransaction) -> Result<C::Hash>
    where
        S: SignerBehaviour<C> + Send + Sync;
    async fn send_raw_utxo<S>(&self, signer: &S, raw_utxo: CloudUtxoTransaction) -> Result<C::Hash>
    where
        S: SignerBehaviour<C> + Send + Sync;

    async fn send_tx<S>(&self, signer: &S, to: C::Address, data: Vec<u8>, value: Vec<u8>) -> Result<C::Hash>
    where
        S: SignerBehaviour<C> + Send + Sync;
    async fn send_utxo<S>(&self, signer: &S, output: Vec<u8>, utxo_type: UtxoType) -> Result<C::Hash>
    where
        S: SignerBehaviour<C> + Send + Sync;
}

#[tonic::async_trait]
impl<C, T> TransactionSenderBehaviour<C> for T
where
    C: Crypto,
    T: ControllerBehaviour<C> + Send + Sync,
{
    async fn send_raw_tx<S>(&self, signer: &S, raw_tx: CloudNormalTransaction) -> Result<C::Hash>
    where
        S: SignerBehaviour<C> + Send + Sync,
    {
        let raw = signer.sign_raw_tx(raw_tx);
        self.send_raw(raw).await.context("failed to send raw")
    }

    async fn send_raw_utxo<S>(&self, signer: &S, raw_utxo: CloudUtxoTransaction) -> Result<C::Hash> 
    where
        S: SignerBehaviour<C> + Send + Sync,
    {
        let raw = signer.sign_raw_utxo(raw_utxo);
        self.send_raw(raw).await.context("failed to send raw")
    }

    async fn send_tx<S>(&self, signer: &S, to: C::Address, data: Vec<u8>, value: Vec<u8>) -> Result<C::Hash>
    where
        S: SignerBehaviour<C> + Send + Sync,
    {
        let (current_block_number, system_config) =
            tokio::try_join!(self.get_block_number(false), self.get_system_config())
                .context("failed to fetch chain status")?;

        let raw_tx = CloudNormalTransaction {
            version: system_config.version,
            to: to.to_vec(),
            data,
            value,
            nonce: rand::random::<u64>().to_string(),
            quota: 3_000_000,
            valid_until_block: current_block_number + 95,
            chain_id: system_config.chain_id.clone(),
        };

        self.send_raw_tx(signer, raw_tx).await
    }

    async fn send_utxo<S>(&self, signer: &S, output: Vec<u8>, utxo_type: UtxoType) -> Result<C::Hash> 
    where
        S: SignerBehaviour<C> + Send + Sync,
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

// #[tonic::async_trait]
// pub trait RawTransactionSenderBehaviour<C: Crypto> {
//     async fn send_raw_tx<A: AccountBehaviour<SigningAlgorithm = C>>(&self, account: &A, raw_tx: CloudNormalTransaction) -> Result<C::Hash>;
//     async fn send_raw_utxo<A: AccountBehaviour<SigningAlgorithm = C>>(&self, account: &A, raw_utxo: CloudUtxoTransaction) -> Result<C::Hash>;
// }

// // It's actually the implementation details of the current controller service.
// #[repr(u64)]
// #[derive(Debug, Clone, Copy)]
// pub enum UtxoType {
//     Admin = 1002,
//     BlockInterval = 1003,
//     Validators = 1004,
//     EmergencyBrake = 1005,
// }

// #[tonic::async_trait]
// pub trait NormalTransactionSenderBehaviour<C: Crypto> {
//     async fn send_tx<A: AccountBehaviour<SigningAlgorithm = C>>(&self, account: &A, to: C::Address, data: Vec<u8>, value: Vec<u8>) -> Result<C::Hash>;
// }

// #[tonic::async_trait]
// pub trait UtxoTransactionSenderBehaviour<C: Crypto> {
//     async fn send_utxo<A: AccountBehaviour<SigningAlgorithm = C>>(&self, account: &A, output: Vec<u8>, utxo_type: UtxoType) -> Result<C::Hash>;
// }
