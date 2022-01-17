use super::context::Context;
use prost::Message;
// use crate::wallet::Account;

use crate::{proto::{
    blockchain::{
        raw_transaction::Tx, CompactBlock, RawTransaction, Transaction as CloudTransaction,
        UnverifiedTransaction, UnverifiedUtxoTransaction, UtxoTransaction as CloudUtxoTransaction,
        Witness,
    },
    common::{Empty, Address, Hash, NodeInfo, NodeNetInfo},
    controller::{
        rpc_service_client::RpcServiceClient as ControllerClient, BlockNumber, Flag, SystemConfig,
        TransactionIndex,
    },
    evm::{rpc_service_client::RpcServiceClient, Balance, ByteAbi, ByteCode, Nonce, Receipt},
    executor::{executor_service_client::ExecutorServiceClient as ExecutorClient, CallRequest},
}, crypto::EthCrypto};

use super::controller::ControllerBehaviour;
use super::controller::NormalTransactionSenderBehaviour;
use crate::crypto::ArrayLike;
use crate::crypto::Crypto;
use crate::utils::parse_addr;
use anyhow::Context as _;
use anyhow::Result;
use tonic::transport::Channel;

// TODO: use constant array for these constant to avoid runtime parsing.

/// Store action target address
const STORE_ADDRESS: &str = "0xffffffffffffffffffffffffffffffffff010000";
/// StoreAbi action target address
const ABI_ADDRESS: &str = "0xffffffffffffffffffffffffffffffffff010001";
/// Amend action target address
const AMEND_ADDRESS: &str = "0xffffffffffffffffffffffffffffffffff010002";

/// amend the abi data
const AMEND_ABI: &str = "0x01";
/// amend the account code
const AMEND_CODE: &str = "0x02";
/// amend the kv of db
const AMEND_KV_H256: &str = "0x03";
/// amend account balance
const AMEND_BALANCE: &str = "0x05";

pub type EvmClient = crate::proto::evm::rpc_service_client::RpcServiceClient<Channel>;


#[tonic::async_trait]
pub trait EvmBehaviour<C: Crypto> {
    // TODO: better address name

    async fn get_receipt(&self, hash: C::Hash) -> Result<Receipt>;
    async fn get_code(&self, addr: C::Address) -> Result<ByteCode>;
    async fn get_balance(&self, addr: C::Address) -> Result<Balance>;
    async fn get_tx_count(&self, addr: C::Address) -> Result<Nonce>;
    async fn get_abi(&self, addr: C::Address) -> Result<ByteAbi>;
}

#[tonic::async_trait]
impl<C: Crypto> EvmBehaviour<C> for EvmClient {
    async fn get_receipt(&self, hash: C::Hash) -> Result<Receipt> {
        let hash = Hash {
            hash: hash.to_vec(),
        };
        self.clone()
            .get_transaction_receipt(hash)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get receipt")
    }

    async fn get_code(&self, addr: C::Address) -> Result<ByteCode> {
        let addr = Address {
            address: addr.to_vec(),
        };
        EvmClient::get_code(&mut self.clone(), addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get code")
    }

    async fn get_balance(&self, addr: C::Address) -> Result<Balance> {
        let addr = Address {
            address: addr.to_vec(),
        };
        EvmClient::get_balance(&mut self.clone(), addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get balance")
    }

    async fn get_tx_count(&self, addr: C::Address) -> Result<Nonce> {
        let addr = Address {
            address: addr.to_vec(),
        };
        self.clone()
            .get_transaction_count(addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get tx count")
    }

    async fn get_abi(&self, addr: C::Address) -> Result<ByteAbi> {
        let addr = Address {
            address: addr.to_vec(),
        };
        EvmClient::get_abi(&mut self.clone(), addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get abi")
    }
}

// TODO: better name and should I add the EvmBehaviour<C> trait bound?
#[tonic::async_trait]
pub trait EvmBehaviourExt<C: Crypto> {
    async fn store_abi(&self, contract_addr: C::Address, abi: &[u8]) -> Result<C::Hash>;
}

#[tonic::async_trait]
impl<C, T> EvmBehaviourExt<C> for T
where
    C: Crypto,
    T: NormalTransactionSenderBehaviour<C> + Send + Sync + 'static,
{
    // The binary protocol is the implementation details of the current EVM service.
    async fn store_abi(&self, contract_addr: C::Address, abi: &[u8]) -> Result<C::Hash> {
        let abi_to = parse_addr::<C>(ABI_ADDRESS)?;
        let data = [contract_addr.as_slice(), abi].concat();
        let tx_hash = self.send_tx(abi_to, data, vec![0; 32]).await?;

        Ok(tx_hash)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parse_data;
    use crate::crypto::EthCrypto;

    #[test]
    fn test_constant() -> Result<()> {
        // TODO: add sm crypto test
        parse_addr::<EthCrypto>(STORE_ADDRESS)?;
        parse_addr::<EthCrypto>(ABI_ADDRESS)?;
        parse_addr::<EthCrypto>(AMEND_ADDRESS)?;

        parse_data(AMEND_ABI)?;
        parse_data(AMEND_CODE)?;
        parse_data(AMEND_KV_H256)?;
        parse_data(AMEND_BALANCE)?;
        Ok(())
    }
}
