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

use crate::crypto::{ArrayLike, Crypto};

use serde::{Deserialize, Serialize};
use anyhow::Result;

pub trait AccountBehaviour: Sized {
    type SigningAlgorithm: Crypto;

    // TODO: consider this Self: Sized
    fn generate() -> Result<Self>
        // where Self: Sized
    {
        let sk = Self::SigningAlgorithm::generate_secret_key();
        Self::from_secret_key(sk)
    }

    fn from_secret_key(sk: <Self::SigningAlgorithm as Crypto>::SecretKey) -> Result<Self>;

    fn address(&self) -> Result<&<Self::SigningAlgorithm as Crypto>::Address>;
    fn public_key(&self) -> Result<&<Self::SigningAlgorithm as Crypto>::PublicKey>;
    fn expose_secret_key(&self) -> Result<&<Self::SigningAlgorithm as Crypto>::SecretKey>;

    fn sign(&self, msg: &[u8]) -> Result<<Self::SigningAlgorithm as Crypto>::Signature> {
        Ok(<Self::SigningAlgorithm as Crypto>::sign(msg, self.expose_secret_key()?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account<C: Crypto> {
    pub(crate) address: C::Address,
    pub(crate) public_key: C::PublicKey,
    pub(crate) secret_key: C::SecretKey,
}

impl<C: Crypto> AccountBehaviour for Account<C> {
    type SigningAlgorithm = C;

    fn generate() -> Result<Self> {
        let (public_key, secret_key) = C::generate_keypair();
        let address = C::pk2addr(&public_key);

        Ok(Self {
            address,
            public_key,
            secret_key,
        })
    }

    fn from_secret_key(sk: C::SecretKey) -> Result<Self> {
        let public_key = C::sk2pk(&sk);
        let address = C::pk2addr(&public_key);
        Ok(Self {
            address,
            public_key,
            secret_key: sk,
        })
    }

    fn address(&self) -> Result<&C::Address> {
        Ok(&self.address)
    }

    fn public_key(&self) -> Result<&C::PublicKey> {
        Ok(&self.public_key)
    }

    fn expose_secret_key(&self) -> Result<&C::SecretKey> {
        Ok(&self.secret_key)
    }
}
