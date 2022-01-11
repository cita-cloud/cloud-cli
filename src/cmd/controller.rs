use prost::Message;
use crate::context::Context;
use crate::wallet::Account;
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
use crate::crypto::{Crypto, ArrayLike};

use tonic::transport::Channel;

type ControllerClient = RpcServiceClient<Channel>;

#[tonic::async_trait]
pub trait ControllerBehaviour<C: Crypto, A: AccountBehaviour<C>> {
    async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash>;
    async fn send_raw_tx(&self, tx: CloudTransaction) -> Result<C::Hash>;
    async fn send_raw_utxo(&self, utxo: CloudUtxoTransaction) -> Result<C::Hash>;

    async fn get_system_config(&self) -> Result<SystemConfig>;

    async fn get_block_number(&self, for_pending: bool) -> Result<u64>;
    async fn get_block_hash(&self, block_number: u64) -> Result<Hash>;

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock>;
    async fn get_block_by_hash(&self, hash: C::Hash) -> Result<CompactBlock>;

    async fn get_tx(&self, tx_hash: C::Hash) -> Result<RawTransaction>;
    async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<TransactionIndex>;
    async fn get_tx_block_number(&self, tx_hash: C::Hash) -> Result<BlockNumber>;

    async fn get_peer_count(&self) -> Result<u64>;
    async fn get_peers_info(&self) -> Result<Vec<NodeInfo>>;

    async fn add_node(&self, multiaddr: String) -> Result<u32>;
}


#[tonic::async_trait]
impl<C: Crypto, A: AccountBehaviour<C>> ControllerBehaviour<C, A> for ControllerClient {

    async fn send_raw_tx(&self, tx: CloudTransaction) -> Result<Self::Hash> {
        let raw = self.prepare_raw_tx(tx);
        self.send_raw(raw).await.context("failed to send transaction")
    }

    async fn send_raw_utxo(&self, utxo: CloudUtxoTransaction) -> Result<Self::Hash> {
        let raw = self.prepare_raw_utxo(utxo);
        self.send_raw(raw).await.context("fail to send utxo")
    }

    async fn send_raw(&mut self, raw: RawTransaction) -> Result<C::Hash> {
        self
            .send_raw_transaction(raw)
            .await
            .map(|resp| Ok(resp.into_inner().hash))
            .map_err(|status| anyhow!("failed to send raw transaction, status: `{}`", status))
    }

    async fn get_system_config(&mut self) -> Result<SystemConfig> {
        self
            .get_system_config(Empty {})
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get system config, status: `{}`", status))
    }

    async fn get_block_number(&mut self, for_pending: bool) -> Result<u64> {
        let flag = Flag { flag: for_pending };
        self
            .get_block_number(flag)
            .await
            .map(|resp| resp.into_inner().block_number)
            .map_err(|status| anyhow!("failed to get block number, status: `{}`", status))
    }

    async fn get_block_hash(&mut self, block_number: u64) -> Result<C::Hash> {
        self
            .get_block_hash(BlockNumber { block_number })
            .await
            .map(|resp| resp.into_inner().hash)
            .map_err(|status| anyhow!("failed to get block hash, status: `{}`", status))
    }

    async fn get_block_by_number(&mut self, block_number: u64) -> Result<CompactBlock> {
        self
            .get_block_by_number(BlockNumber { block_number })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get block by number, status: `{}`", status))
    }

    async fn get_block_by_hash(&mut self, hash: C::Hash) -> Result<CompactBlock> {
        self
            .get_block_by_hash(Hash { hash })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get block by hash, status: `{}`", status))
    }

    async fn get_tx(&mut self, tx_hash: C::Hash) -> Result<RawTransaction> {
        self
            .get_transaction(Hash { hash: tx_hash })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get tx, status: `{}`", status))
    }

    async fn get_tx_index(&self, tx_hash: C::Hash) -> Result<TransactionIndex> {
        self
            .get_transaction_index(Hash { hash: tx_hash })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get tx index, status: `{}`", status))
    }

    async fn get_tx_block_number(&mut self, tx_hash: C::Hash) -> Result<BlockNumber> {
        self
            .get_transaction_block_number(Hash { hash: tx_hash })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get tx block number, status: `{}`", status))
    }

    async fn get_peer_count(&mut self) -> Result<u64> {
        self
            .get_peer_count(Empty {})
            .await
            .map(|resp| resp.into_inner().peer_count)
            .map_err(|status| anyhow!("failed to get peer count, status: `{}`", status))
    }

    async fn get_peers_info(&mut self) -> Result<Vec<NodeInfo>> {
        self
            .get_peers_info(Empty {})
            .await
            .map(|resp| resp.into_inner().nodes)
            .map_err(|status| anyhow!("failed to get peers info, status: `{}`", status))
    }

    async fn add_node(&mut self, multiaddr: String) -> Result<u32> {
        self
            .add_node(NodeNetInfo { multi_address: multiaddr, ..Default::default() })
            .await
            .unwrap()
            .map(|resp| resp.into_inner().code)
            .map_err(|status| anyhow!("failed to add node, status: `{}`", status))
    }
}


pub trait SignerBehaviour<C: Crypto>: AccountBehaviour<C> {
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
        let signature = self.sign(&tx_hash).as_slice().to_vec();

        // build raw tx
        let raw_tx = {
            let sender = self.address().as_slice().to_vec();
            let witness = Witness {
                signature,
                sender,
            };

            let unverified_tx = UnverifiedTransaction {
                transaction: Some(tx),
                transaction_hash: tx_hash,
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
        let signature = self.sign(&utxo_hash);

        // build raw utxo
        let raw_utxo = {
            let sender = self.address().as_slice().to_vec();
            let witness = Witness {
                signature,
                sender,
            };

            let unverified_utxo = UnverifiedUtxoTransaction {
                transaction: Some(utxo),
                transaction_hash: utxo_hash,
                witnesses: vec![witness],
            };

            RawTransaction {
                tx: Some(Tx::UtxoTx(unverified_utxo)),
            }
        };

        raw_utxo
    }
}
