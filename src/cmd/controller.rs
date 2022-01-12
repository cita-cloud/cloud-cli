use prost::Message;
// use crate::context::Context;
// use crate::wallet::Account;
use super::wallet::AccountBehaviour;

use crate::proto::{
    blockchain::{
        raw_transaction::Tx, CompactBlock, RawTransaction, Transaction as CloudTransaction,
        UnverifiedTransaction, UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction,
        Witness,
    },
    common::{Address, Empty, Hash, NodeInfo, NodeNetInfo},
    controller::{
        rpc_service_client::RpcServiceClient, BlockNumber, Flag, SystemConfig,
        TransactionIndex,
    },
    evm::{
        rpc_service_client::RpcServiceClient as EvmClient, Balance, ByteAbi, ByteCode, Nonce,
        Receipt,
    },
    executor::{executor_service_client::ExecutorServiceClient as ExecutorClient, CallRequest},
};

use anyhow::Result;
use anyhow::anyhow;
use anyhow::Context;
use crate::crypto::{Crypto, ArrayLike};

use tonic::transport::Channel;

type ControllerClient = RpcServiceClient<Channel>;

#[tonic::async_trait]
pub trait ControllerBehaviour<C: Crypto> {
    async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash>;

    async fn get_system_config(&self) -> Result<SystemConfig>;

    async fn get_block_number(&self, for_pending: bool) -> Result<u64>;
    async fn get_block_hash(&self, block_number: u64) -> Result<C::Hash>;

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock>;
    async fn get_block_by_hash(&self, hash: C::Hash) -> Result<CompactBlock>;

    async fn get_tx(&self, tx_hash: C::Hash) -> Result<RawTransaction>;
    async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<u64>;
    async fn get_tx_block_number(&self, tx_hash: C::Hash) -> Result<u64>;

    async fn get_peer_count(&self) -> Result<u64>;
    async fn get_peers_info(&self) -> Result<Vec<NodeInfo>>;

    async fn add_node(&self, multiaddr: String) -> Result<u32>;
}

#[tonic::async_trait]
impl<C: Crypto> ControllerBehaviour<C> for ControllerClient {
    async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash> {
        let resp = self
            .clone()
            .send_raw_transaction(raw)
            .await?
            .into_inner();

        C::Hash::try_from_slice(&resp.hash)
            .context("controller returns an invalid transaction hash, maybe we are using a wrong signing algorithm?")
    }

    async fn get_system_config(&self) -> Result<SystemConfig> {
        let resp = ControllerClient::get_system_config(&mut self.clone(), Empty{}).await?.into_inner();

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
            .context("controller returns an invalid block hash, maybe we are using a wrong signing algorithm?")
    }

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock> {
        let block_number = BlockNumber { block_number };
        let resp = ControllerClient::get_block_by_number(&mut self.clone(), block_number).await?.into_inner();

        Ok(resp)
    }

    async fn get_block_by_hash(&self, hash: C::Hash) -> Result<CompactBlock> {
        let hash = Hash{ hash: hash.to_vec() };
        let resp = ControllerClient::get_block_by_hash(&mut self.clone(), hash).await?.into_inner();

        Ok(resp)
    }

    async fn get_tx(&self, tx_hash: C::Hash) -> Result<RawTransaction> {
        let resp = self
            .clone()
            .get_transaction(Hash { hash: tx_hash.to_vec() })
            .await?
            .into_inner();

        Ok(resp)
    }

    async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<u64> {
        let resp = self
            .clone()
            .get_transaction_index(Hash { hash: tx_hash.to_vec() })
            .await?
            .into_inner();

        Ok(resp.tx_index)
    }

    async fn get_tx_block_number(&self, tx_hash: C::Hash) -> Result<u64> {
        let resp = self
            .clone()
            .get_transaction_block_number(Hash { hash: tx_hash.to_vec() })
            .await?
            .into_inner();

        Ok(resp.block_number)
    }

    async fn get_peer_count(&self) -> Result<u64> {
        let resp = ControllerClient::get_peer_count(&mut self.clone(), Empty{})
            .await?
            .into_inner();
        
        Ok(resp.peer_count)
    }

    async fn get_peers_info(&self) -> Result<Vec<NodeInfo>> {
        let resp = ControllerClient::get_peers_info(&mut self.clone(), Empty{})
            .await?
            .into_inner();
        
        Ok(resp.nodes)
    }

    async fn add_node(&self, multiaddr: String) -> Result<u32> {
        let node_info = NodeNetInfo { multi_address: multiaddr, ..Default::default() };
        let resp = ControllerClient::add_node(&mut self.clone(), node_info)
            .await?
            .into_inner();
        
        Ok(resp.code)
    }
}

pub trait SignerBehaviour<C: Crypto> {
    fn prepare_raw_tx(&self, tx: CloudTransaction) -> RawTransaction;
    fn prepare_raw_utxo(&self, utxo: CloudUtxoTransaction) -> RawTransaction;
}

impl<C, A> SignerBehaviour<C> for A
where
    C: Crypto,
    A: AccountBehaviour<SigningAlgorithm = C>,
{
    fn prepare_raw_tx(&self, tx: CloudTransaction) -> RawTransaction {
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
            let witness = Witness {
                sender,
                signature,
            };

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

    fn prepare_raw_utxo(&self, utxo: CloudUtxoTransaction) -> RawTransaction {
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
            let witness = Witness {
                sender,
                signature,
            };

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


#[tonic::async_trait]
pub trait RawTransactionSenderBehaviour<C: Crypto> {
    async fn send_raw_tx(&self, tx: CloudTransaction) -> Result<C::Hash>;
    async fn send_raw_utxo(&self, utxo: CloudUtxoTransaction) -> Result<C::Hash>;
}
    
#[tonic::async_trait]
impl<C, T> RawTransactionSenderBehaviour<C> for T
where
    C: Crypto,
    T: HasController<C> + HasSigner<C> + Send + Sync + 'static
{
    async fn send_raw_tx(&self, tx: CloudTransaction) -> Result<C::Hash> {
        let raw = self.signer().prepare_raw_tx(tx);
        let controller = self.controller();
        controller.send_raw(raw).await.context("failed to send raw tx")
    }

    async fn send_raw_utxo(&self, utxo: CloudUtxoTransaction) -> Result<C::Hash> {
        let raw = self.signer().prepare_raw_utxo(utxo);
        let controller = self.controller();
        controller.send_raw(raw).await.context("failed to send raw utxo")
    }
}

#[tonic::async_trait]
pub trait TransactionSenderBehaviour<C: Crypto> {
    async fn send_tx(
        &self,
        to: C::Address,
        data: Vec<u8>,
        value: Vec<u8>,
        nonce: String,
        quota: u64,
        valid_until_block: u64,
    ) -> Result<C::Hash>;

    async fn send_utxo(
        &self,
        pre_tx_hash: Vec<u8>, // TODO: Or C::Hash?
        output: Vec<u8>,
        lock_id: u64,
    ) -> Result<C::Hash>;
}

#[tonic::async_trait]
impl<C, T> TransactionSenderBehaviour<C> for T
where
    C: Crypto,
    T: RawTransactionSenderBehaviour<C> + HasSystemConfig + Send + Sync + 'static
{
    async fn send_tx(
        &self,
        to: C::Address,
        data: Vec<u8>,
        value: Vec<u8>,
        nonce: String,
        quota: u64,
        valid_until_block: u64,
    ) -> Result<C::Hash> {
        let system_config = self.system_config();
        let raw_tx = CloudTransaction {
            to: to.to_vec(),
            data,
            value,
            nonce,
            quota,
            valid_until_block,
            version: system_config.version,
            chain_id: system_config.chain_id.clone(),
        };
        self.send_raw_tx(raw_tx).await
    }

    async fn send_utxo(
        &self,
        pre_tx_hash: Vec<u8>, // TODO: Or C::Hash?
        output: Vec<u8>,
        lock_id: u64,
    ) -> Result<C::Hash> {
        let system_config = self.system_config();
        let raw_utxo = CloudUtxoTransaction{
            version: system_config.version,
            pre_tx_hash, 
            output,
            lock_id,
        };
        self.send_raw_utxo(raw_utxo).await
    }
}

pub trait HasController<C: Crypto> {
    type Controller: ControllerBehaviour<C> + Send + Sync + 'static;

    fn controller(&self) -> &Self::Controller;
}

impl<C, T> HasController<C> for T
where
    C: Crypto,
    T: ControllerBehaviour<C> + Send + Sync + 'static,
{
    type Controller = T;

    fn controller(&self) -> &Self::Controller {
        self
    }
}

pub trait HasSystemConfig {
    fn system_config(&self) -> &SystemConfig;
}

impl HasSystemConfig for SystemConfig {
    fn system_config(&self) -> &SystemConfig {
        self
    }
}

pub trait HasAccount<C: Crypto> {
    type Account: AccountBehaviour<SigningAlgorithm = C>;

    fn account(&self) -> &Self::Account;
}

pub trait HasSigner<C: Crypto> {
    type Signer: SignerBehaviour<C>;

    fn signer(&self) -> &Self::Signer;
}

impl<C, A, T> HasSigner<C> for T
where
    C: Crypto,
    A: AccountBehaviour<SigningAlgorithm = C>,
    T: HasAccount<C, Account = A>,
{
    type Signer = A;

    fn signer(&self) -> &Self::Signer {
        self.account()
    }
}
