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
use serde::Serializer;

use super::account::Account;
use super::account::AccountBehaviour;
use anyhow::Result;
use anyhow::Context;
use anyhow::bail;
use anyhow::anyhow;
use anyhow::ensure;
use std::{path::Path, io};
use std::path::PathBuf;
use std::collections::HashMap;
use std::io::ErrorKind;
use crate::utils::*;

use tokio::fs;
use tokio::io::AsyncWriteExt;


#[tonic::async_trait]
pub trait WalletBehaviour<C: Crypto> {
    type Account: AccountBehaviour<SigningAlgorithm = C>;

    async fn generate_account(&mut self, id: &str, pw: Option<&str>) -> Result<()>;
    async fn import_account(&mut self, id: &str, account: Self::Account, pw: Option<&str>) -> Result<()>;
    async fn unlock_account(&mut self, id: &str, pw: Option<&str>) -> Result<()>;
    async fn delete_account(&mut self, id: &str) -> Result<()>;

    async fn use_account(&mut self, id: &str) -> Result<&Self::Account>;
    async fn current_account(&self) -> Result<&Self::Account>;

    // Return a Vec since GAT is unstable
    async fn list_account(&self) -> Vec<(&str, Self::Account)>;

    async fn set_default_account(&mut self, id: &str) -> Result<()>;
}

// TODO: remove these helper, use serialize_with for fields

// helper struct
#[derive(Serialize, Deserialize)]
struct Locked {
    address: String,
    encrypted_sk: String,
}

impl<C: Crypto> TryFrom<Locked> for LockedAccount<C> {
    type Error = anyhow::Error;

    fn try_from(locked: Locked) -> Result<Self, Self::Error> {
        let address = parse_addr::<C>(&locked.address).context("invalid address")?;
        let encrypted_sk = parse_data(&locked.encrypted_sk).context("invalid encrypted sk")?;

        Ok(LockedAccount {
            address,
            encrypted_sk,
        })
    }
}

// helper struct
#[derive(Serialize, Deserialize)]
struct Unlocked {
    address: String,
    unencrypted_sk: String,
}

impl<C: Crypto> TryFrom<Unlocked> for Account<C> {
    type Error = anyhow::Error;

    fn try_from(unlocked: Unlocked) -> Result<Self, Self::Error> {
        let address = parse_addr::<C>(&unlocked.address).context("invalid address")?;
        let sk = parse_sk::<C>(&unlocked.unencrypted_sk).context("invalid unencrypted sk format")?;

        let account = Account::from_secret_key(sk).context("invalid unencrypted sk")?;
        ensure!(
            account.address().unwrap() == &address,
            "account address mismatched with the recorded account address"
        );
        Ok(account)
    }
}


pub struct LockedAccount<C: Crypto> {
    address: C::Address,
    encrypted_sk: Vec<u8>,
}

impl<C: Crypto> LockedAccount<C> {
    fn unlock(&self, pw: &str) -> Result<Account<C>> {
        let decrypted = C::decrypt(self.encrypted_sk.as_slice(), pw.as_bytes());
        let sk = C::SecretKey::try_from_slice(&decrypted)?;

        Account::from_secret_key(sk).context("invalid secret key")
    }
}

impl<C: Crypto> Account<C> {
    fn lock(self, pw: &str) -> LockedAccount<C> {
        let encrypted_sk = C::encrypt(self.secret_key.as_slice(), pw.as_bytes());

        LockedAccount {
            address: self.address,
            encrypted_sk,
        }
    }
}


pub enum MaybeLockedAccount<C: Crypto> {
    Locked(LockedAccount<C>),
    Unlocked(Account<C>),
}

impl<C: Crypto> MaybeLockedAccount<C> {
    fn unlock(&mut self, pw: Option<&str>) -> Result<()> {
        match (&self, pw) {
            (Self::Locked(locked), Some(pw)) => {
                *self = Self::Unlocked(locked.unlock(pw)?);
            }
            (Self::Unlocked(_), _) => (),
            _ => bail!("no passsword provided for a locked account"),
        }

        Ok(())
    }
}

impl<C: Crypto> From<LockedAccount<C>> for MaybeLockedAccount<C> {
    fn from(locked: LockedAccount<C>) -> Self {
        Self::Locked(locked)
    }
}

impl<C: Crypto> From<Account<C>> for MaybeLockedAccount<C> {
    fn from(unlocked: Account<C>) -> Self {
        Self::Unlocked(unlocked)
    }
}

impl<C: Crypto> AccountBehaviour for MaybeLockedAccount<C> {
    type SigningAlgorithm = C;

    fn from_secret_key(sk: C::SecretKey) -> Result<Self> {
        let unlocked = Account::<C>::from_secret_key(sk).context("cannot create account from secret key")?;
        Ok(Self::Unlocked(unlocked))
    }

    fn address(&self) -> Result<&C::Address> {
        match self {
            Self::Locked(..) => bail!("Account locked, need to be unlocked first"),
            Self::Unlocked(account) => Ok(account.address().unwrap()),
        }
    }

    fn public_key(&self) -> Result<&C::PublicKey> {
        match self {
            Self::Locked(..) => bail!("Account locked, need to be unlocked first"),
            Self::Unlocked(account) => Ok(account.public_key().unwrap()),
        }
    }

    fn expose_secret_key(&self) -> Result<&C::SecretKey> {
        match self {
            Self::Locked(..) => bail!("Account locked, need to be unlocked first"),
            Self::Unlocked(account) => Ok(account.expose_secret_key().unwrap()),
        }
    }

    fn sign(&self, msg: &[u8]) -> Result<C::Signature> {
        match self {
            Self::Locked(..) => bail!("Account locked, need to be unlocked first"),
            Self::Unlocked(account) => Ok(account.sign(msg).unwrap()),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct WalletConfig {
    default_account: String,
}


pub struct Wallet<C: Crypto> {
    wallet_dir: PathBuf,

    current_account_id: String,
    account_map: HashMap<String, MaybeLockedAccount<C>>,
}

impl<C: Crypto> Wallet<C> {
    pub fn open(wallet_dir: impl AsRef<Path>) -> Self {
        todo!()
    }

    async fn save_account(&mut self, id: &str, may_locked: MaybeLockedAccount<C>) -> Result<()> {
        // TODO: validate id
        let accounts_dir = self.wallet_dir.join("accounts");
        fs::create_dir_all(&accounts_dir).await.context("cannot create directory for accounts")?;

        let mut account_file = {
            let account_file = accounts_dir.join(id);
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(account_file)
                .await
                .context("cannot create account file")?
        };
        
        // TODO: use serialize_with in filed
        let content = match &may_locked {
            MaybeLockedAccount::Unlocked(unlocked) => {
                let address = hex(unlocked.address().unwrap().as_slice());
                let unencrypted_sk = hex(unlocked.expose_secret_key().unwrap().as_slice());
                let unlocked = Unlocked {
                    address,
                    unencrypted_sk,
                };
                toml::to_string_pretty(&unlocked).unwrap()
            }
            MaybeLockedAccount::Locked(locked) => {
                let address = hex(locked.address.as_slice());
                let encrypted_sk = hex(locked.encrypted_sk.as_slice());
                let locked =Locked {
                    address,
                    encrypted_sk,
                };
                toml::to_string_pretty(&locked).unwrap()
            }
        };

        account_file.write_all(content.as_bytes()).await.context("cannot write account content")?;

        self.account_map.insert(id.into(), may_locked);
        Ok(())
    }

    async fn load_account(&mut self, id: &str) -> Result<()> {
        let content = {
            let path = self.wallet_dir.join("accounts").join(id);
            fs::read_to_string(path).await.context("cannot read account file")?
        };

        if let Ok(unlocked) = toml::from_str::<Unlocked>(&content) {
            let account = Account::try_from(unlocked).context("invalid unlocked account")?;
            self.account_map.insert(id.into(), account.into());
        } else if let Ok(locked) = toml::from_str::<Locked>(&content) {
            let account = LockedAccount::try_from(locked).context("invalid locked account")?;
            self.account_map.insert(id.into(), account.into());
        } else {
            bail!("cannot load account from file, invalid format")
        }

        Ok(())
    }

    // async fn delete_account()
}

#[tonic::async_trait]
impl<C: Crypto> WalletBehaviour<C> for Wallet<C> {
    type Account = MaybeLockedAccount<C>;

    async fn generate_account(&mut self, id: &str, pw: Option<&str>) -> Result<()> {
        let account = Account::generate().context("cannot generate account")?;
        let may_locked = match pw {
            Some(pw) => account.lock(pw).into(),
            None => account.into()
        };
        self.import_account(id, may_locked, pw).await
    }

    async fn import_account(&mut self, id: &str, account: Self::Account, pw: Option<&str>) -> Result<()> {
        self.save_account(id, account).await.context("cannot save generated account")
    }

    async fn unlock_account(&mut self, id: &str, pw: Option<&str>) -> Result<()> {
        let account = self.account_map.get_mut(id).ok_or(anyhow!("no such an account"))?;
        account.unlock(pw)?;

        Ok(())
    }

    async fn delete_account(&mut self, id: &str) -> Result<()> {

        todo!()
    }

    async fn use_account(&mut self, id: &str) -> Result<&Self::Account> {

        todo!()
    }

    async fn current_account(&self) -> Result<&Self::Account> {

        todo!()
    }

    // Return a Vec since GAT is unstable
    async fn list_account(&self) -> Vec<(&str, Self::Account)> {

        todo!()
    }

    async fn set_default_account(&mut self, id: &str) -> Result<()> {
        todo!()
    }
}
