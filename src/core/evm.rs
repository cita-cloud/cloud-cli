use crate::{
    crypto::EthCrypto,
    proto::{
        common::{Address as CloudAddress, Empty, Hash as CloudHash, NodeInfo, NodeNetInfo},
        evm::{rpc_service_client::RpcServiceClient, Balance, ByteAbi, ByteCode, Nonce, Receipt},
    },
};

use super::controller::ControllerBehaviour;
use super::controller::SignerBehaviour;
use super::controller::TransactionSenderBehaviour;

use crate::crypto::ArrayLike;
use crate::crypto::Crypto;
use crate::crypto::{Address, Hash};

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
pub trait EvmBehaviour {
    // TODO: better address name

    async fn get_receipt(&self, hash: Hash) -> Result<Receipt>;
    async fn get_code(&self, addr: Address) -> Result<ByteCode>;
    async fn get_balance(&self, addr: Address) -> Result<Balance>;
    async fn get_tx_count(&self, addr: Address) -> Result<Nonce>;
    async fn get_abi(&self, addr: Address) -> Result<ByteAbi>;
}

#[tonic::async_trait]
impl EvmBehaviour for EvmClient {
    async fn get_receipt(&self, hash: Hash) -> Result<Receipt> {
        let hash = CloudHash {
            hash: hash.to_vec(),
        };
        self.clone()
            .get_transaction_receipt(hash)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get receipt")
    }

    async fn get_code(&self, addr: Address) -> Result<ByteCode> {
        let addr = CloudAddress {
            address: addr.to_vec(),
        };
        EvmClient::get_code(&mut self.clone(), addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get code")
    }

    async fn get_balance(&self, addr: Address) -> Result<Balance> {
        let addr = CloudAddress {
            address: addr.to_vec(),
        };
        EvmClient::get_balance(&mut self.clone(), addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get balance")
    }

    async fn get_tx_count(&self, addr: Address) -> Result<Nonce> {
        let addr = CloudAddress {
            address: addr.to_vec(),
        };
        self.clone()
            .get_transaction_count(addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get tx count")
    }

    async fn get_abi(&self, addr: Address) -> Result<ByteAbi> {
        let addr = CloudAddress {
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
pub trait EvmBehaviourExt {
    async fn store_contract_abi<S>(
        &self,
        signer: &S,
        contract_addr: Address,
        abi: &[u8],
    ) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync;
}

#[tonic::async_trait]
impl<T> EvmBehaviourExt for T
where
    T: TransactionSenderBehaviour + Send + Sync,
{
    // The binary protocol is the implementation details of the current EVM service.
    async fn store_contract_abi<S>(
        &self,
        signer: &S,
        contract_addr: Address,
        abi: &[u8],
    ) -> Result<Hash>
    where
        S: SignerBehaviour + Send + Sync,
    {
        let abi_addr = parse_addr(ABI_ADDRESS)?;
        let data = [contract_addr.as_slice(), abi].concat();
        let tx_hash = self.send_tx(signer, abi_addr, data, vec![0; 32]).await?;

        Ok(tx_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parse_data;

    #[test]
    fn test_constant() -> Result<()> {
        // TODO: add sm crypto test
        parse_addr(STORE_ADDRESS)?;
        parse_addr(ABI_ADDRESS)?;
        parse_addr(AMEND_ADDRESS)?;

        parse_data(AMEND_ABI)?;
        parse_data(AMEND_CODE)?;
        parse_data(AMEND_KV_H256)?;
        parse_data(AMEND_BALANCE)?;
        Ok(())
    }
}
