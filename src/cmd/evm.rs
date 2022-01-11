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

use crate::crypto::Crypto;
use super::controller::ControllerBehaviour;
use crate::utils::parse_addr;
use anyhow::Result;
use anyhow::Context as _;


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


#[tonic::async_trait]
pub trait EvmBehaviour {
    type Hash;
    type Address;

    async fn get_receipt(&self, hash: Self::Hash) -> Result<Receipt>;
    async fn get_code(&self, address: Self::Address) -> Result<ByteCode>;
    async fn get_balance(&self, address: Self::Address) -> Result<Balance>;
    async fn get_transaction_count(&self, address: Self::Address) -> Result<Nonce>;
    async fn get_abi(&self, address: Self::Address) -> Result<ByteAbi>;
    async fn store_abi(&self, abi: &[u8]) -> Result<Self::Hash>;
}


#[tonic::async_trait]
impl<C: Crypto> EvmBehaviour for Context<C> {
    type Hash = C::Hash;
    type Address = C::Address;

    async fn get_receipt(&self, hash: Self::Hash) -> Result<Receipt> {
        let hash = Hash { hash };
        self.evm
            .clone()
            .get_transaction_receipt(hash)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get receipt")
    }

    async fn get_code(&self, address: Self::Address) -> Result<ByteCode> {
        let addr = Address { address };
        self.evm.clone().get_code(addr).await
            .map(|resp| resp.into_inner())
            .context("failed to get code")
    }

    async fn get_balance(&self, address: Self::Address) -> Result<Balance> {
        let addr = Address { address };
        self.evm
            .clone()
            .get_balance(addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get balance")
    }

    async fn get_transaction_count(&self, address: Self::Address) -> Result<Nonce> {
        let addr = Address { address };
        self.evm
            .clone()
            .get_transaction_count(addr)
            .await
            .map(|resp| resp.into_inner())
            .context("failed to get tx count")
    }

    async fn get_abi(&self, address: Self::Address) -> Result<ByteAbi> {
        let addr = Address { address };
        self.evm.clone().get_abi(addr).await
            .map(|resp| resp.into_inner())
            .context("failed to get abi")
    }

    async fn store_abi(&self, contract_addr: Self::Address, abi: &[u8]) -> Result<Self::Hash> {
        let abi_to = parse_addr(ABI_ADDRESS)?;
        let payload = [contract_addr.as_slice(), abi].concat();
        let tx_hash = self.controller.send_tx(abi_to, payload)?;

        Ok(())
    }


}
