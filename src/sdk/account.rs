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

use crate::crypto::{ ArrayLike, Crypto };

use serde::{Deserialize, Serialize};

pub trait AccountBehaviour {
    type SigningAlgorithm: Crypto;

    fn from_secret_key(sk: <Self::SigningAlgorithm as Crypto>::SecretKey) -> Self;

    fn address(&self) -> &<Self::SigningAlgorithm as Crypto>::Address;
    fn public_key(&self) -> &<Self::SigningAlgorithm as Crypto>::PublicKey;
    fn expose_secret_key(&self) -> &<Self::SigningAlgorithm as Crypto>::SecretKey;

    fn sign(&self, msg: &[u8]) -> <Self::SigningAlgorithm as Crypto>::Signature {
        <Self::SigningAlgorithm as Crypto>::sign(msg, self.expose_secret_key())
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account<C: Crypto> {
    address: C::Address,
    public_key: C::PublicKey,
    secret_key: C::SecretKey,
}

impl<C: Crypto> AccountBehaviour for Account<C> {
    type SigningAlgorithm = C;

    fn from_secret_key(sk: C::SecretKey) -> Self {
        let public_key = C::sk2pk(&sk);
        let address = C::pk2addr(&public_key);
        Self {
            address,
            public_key,
            secret_key: sk,
        }
    }

    fn address(&self) -> &C::Address {
        &self.address
    }

    fn public_key(&self) -> &C::PublicKey {
        &self.public_key
    }

    fn expose_secret_key(&self) -> &C::SecretKey {
        &self.secret_key
    }
}

// pub trait WalletBehaviour<C: Crypto> {
//     type Account: AccountBehaviour<C>;

//     fn generate_account(&self, id: &str) -> Self::Account;
//     fn import_account(&self, id: &str, sk: C::SecretKey);
//     fn export_account(&self, id: &str) -> Option<&Self::Account>;
//     fn delete_account(&self, id: &str) -> Option<Self::Account>;

//     // TODO: better API
//     fn list_account(&self) -> Vec<String>;
// }


