use prost::Message;
use crate::context::Context;
use crate::wallet::Account;

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

use anyhow::Result;
use anyhow::anyhow;
use crate::crypto::{Crypto, BytesLike};

#[tonic::async_trait]
pub trait ControllerBehaviour {
    type Hash;

    async fn send_tx(&self, tx: CloudTransaction) -> Result<Self::Hash>;
    async fn send_utxo(&self, utxo: CloudUtxoTransaction) -> Result<Self::Hash>;

    async fn get_system_config(&self) -> Result<SystemConfig>;

    async fn get_block_number(&self, for_pending: bool) -> Result<u64>;
    async fn get_block_hash(&self, block_number: u64) -> Result<Hash>;

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock>;
    async fn get_block_by_hash(&self, hash: Self::Hash) -> Result<CompactBlock>;

    async fn get_tx(&self, tx_hash: Self::Hash) -> Result<RawTransaction>;
    async fn get_tx_index(&self, tx_hash: Self::Hash) -> Result<TransactionIndex>;
    async fn get_tx_block_number(&self, tx_hash: Self::Hash) -> Result<BlockNumber>;

    async fn get_peer_count(&self) -> Result<u64>;
    async fn get_peers_info(&self) -> Result<Vec<NodeInfo>>;

    async fn add_node(&self, multiaddr: String) -> Result<u32>;
}

impl<C: Crypto> Context<C> {
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
        let signature = self.account.sign(&tx_hash);

        // build raw tx
        let raw_tx = {
            let sender = self.account.addr.as_slice().to_vec();
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
        let signature = self.account.sign(&utxo_hash);

        // build raw utxo
        let raw_utxo = {
            let sender = self.account.addr.as_slice().to_vec();
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

    async fn send_raw(&self, raw: RawTransaction) -> Result<C::Hash> {
        self.controller
            .clone()
            .send_raw_transaction(raw)
            .await
            .map(|resp| Ok(resp.into_inner().hash))
            .map_err(|status| anyhow!("failed to send raw transaction, status: `{}`", status))
    }
}

#[tonic::async_trait]
impl<C: Crypto> ControllerBehaviour for Context<C> {
    type Hash = C::Hash;

    async fn send_tx(&self, tx: CloudTransaction) -> Result<Self::Hash> {
        let raw = self.prepare_raw_tx(tx);
        self.send_raw(raw).await.context("failed to send transaction")
    }

    async fn send_utxo(&self, utxo: CloudUtxoTransaction) -> Result<Self::Hash> {
        let raw = self.prepare_raw_utxo(utxo);
        self.send_raw(raw).await.context("fail to send utxo")
    }

    async fn get_system_config(&self) -> Result<SystemConfig> {
        self.controller
            .clone()
            .get_system_config(Empty {})
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get system config, status: `{}`", status))
    }

    async fn get_block_number(&self, for_pending: bool) -> Result<u64> {
        let flag = Flag { flag: for_pending };
        self.controller
            .clone()
            .get_block_number(flag)
            .await
            .map(|resp| resp.into_inner().block_number)
            .map_err(|status| anyhow!("failed to get block number, status: `{}`", status))
    }

    async fn get_block_hash(&self, block_number: u64) -> Result<Self::Hash> {
        self.controller
            .clone()
            .get_block_hash(BlockNumber { block_number })
            .await
            .map(|resp| resp.into_inner().hash)
            .map_err(|status| anyhow!("failed to get block hash, status: `{}`", status))
    }

    async fn get_block_by_number(&self, block_number: u64) -> Result<CompactBlock> {
        self.controller
            .clone()
            .get_block_by_number(BlockNumber { block_number })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get block by number, status: `{}`", status))
    }

    async fn get_block_by_hash(&self, hash: Self::Hash) -> Result<CompactBlock> {
        self.controller
            .clone()
            .get_block_by_hash(Hash { hash })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get block by hash, status: `{}`", status))
    }

    async fn get_tx(&self, tx_hash: Self::Hash) -> Result<RawTransaction> {
        self.controller
            .clone()
            .get_transaction(Hash { hash: tx_hash })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get tx, status: `{}`", status))
    }

    async fn get_tx_index(&self, tx_hash: Self::Hash) -> Result<TransactionIndex> {
        self.controller
            .clone()
            .get_transaction_index(Hash { hash: tx_hash })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get tx index, status: `{}`", status))
    }

    async fn get_tx_block_number(&self, tx_hash: Self::Hash) -> Result<BlockNumber> {
        self.controller
            .clone()
            .get_transaction_block_number(Hash { hash: tx_hash })
            .await
            .map(|resp| resp.into_inner())
            .map_err(|status| anyhow!("failed to get tx block number, status: `{}`", status))
    }

    async fn get_peer_count(&self) -> Result<u64> {
        self.controller
            .clone()
            .get_peer_count(Empty {})
            .await
            .map(|resp| resp.into_inner().peer_count)
            .map_err(|status| anyhow!("failed to get peer count, status: `{}`", status))
    }

    async fn get_peers_info(&self) -> Result<Vec<NodeInfo>> {
        self.controller
            .clone()
            .get_peers_info(Empty {})
            .await
            .map(|resp| resp.into_inner().nodes)
            .map_err(|status| anyhow!("failed to get peers info, status: `{}`", status))
    }

    async fn add_node(&self, multiaddr: String) -> Result<u32> {
        self.controller
            .clone()
            .add_node(NodeNetInfo { multi_address: multiaddr, origin: 0 })
            .await
            .unwrap()
            .map(|resp| resp.into_inner().code)
            .map_err(|status| anyhow!("failed to add node, status: `{}`", status))
    }
}
