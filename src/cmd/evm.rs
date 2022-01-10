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
use anyhow::Result;


#[tonic::async_trait]
pub trait EvmBehaviour {
    type Hash;
    type Address;

    async fn get_receipt(&self, hash: Self::Hash) -> Result<Receipt>;
    async fn get_code(&self, address: Self::Address) -> Result<ByteCode>;
    async fn get_balance(&self, address: Self::Address) -> Result<Balance>;
    async fn get_transaction_count(&self, address: Self::Address) -> Result<Nonce>;
    async fn get_abi(&self, address: Self::Address) -> Result<ByteAbi>;
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
            .unwrap()
            .into_inner()
    }

    async fn get_code(&self, address: Self::Address) -> Result<ByteCode> {
        let addr = Address { address };
        self.evm.clone().get_code(addr).await.unwrap().into_inner()
    }

    async fn get_balance(&self, address: Self::Address) -> Result<Balance> {
        let addr = Address { address };
        self.evm
            .clone()
            .get_balance(addr)
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_transaction_count(&self, address: Self::Address) -> Result<Nonce> {
        let addr = Address { address };
        self.evm
            .clone()
            .get_transaction_count(addr)
            .await
            .unwrap()
            .into_inner()
    }

    async fn get_abi(&self, address: Self::Address) -> Result<ByteAbi> {
        let addr = Address { address };
        self.evm.clone().get_abi(addr).await.unwrap().into_inner()
    }


}
