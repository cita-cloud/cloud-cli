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
use crate::config::Config;

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
use std::collections::BTreeMap;
use std::io::ErrorKind;
use crate::utils::*;

use serde::de::DeserializeOwned;

use tokio::fs;
use tokio::io::AsyncWriteExt;


// TODO: should I use one single trait for locakable account?
pub trait LockableAccount {
    type Locked: UnlockableAccount<Unlocked = Self>;

    fn lock(self, pw: &str) -> Self::Locked;
}

pub trait UnlockableAccount: Sized {
    type Unlocked: LockableAccount<Locked = Self>;

    fn unlock(self, pw: &str) -> Result<Self::Unlocked, Self>;
}

pub enum MaybeLockedAccount<L, U>
// where
//     L: UnlockableAccount<Unlocked = U>,
//     U: LockableAccount<Locked = L>,
{
    Locked(L),
    Unlocked(U),
}

impl<L, U> MaybeLockedAccount<L, U>
where
    L: UnlockableAccount<Unlocked = U>,
    U: LockableAccount<Locked = L>,
{
    pub fn unlock(self, pw: &str) -> Result<Self, Self> {
        match self {
            Self::Locked(locked) => locked.unlock(pw).map(Self::Unlocked).map_err(Self::Locked),
            unlocked => Ok(unlocked),
        }
    }
}

// We cannot impl both From<L> and From<U> because potential L == U
impl<L, U> MaybeLockedAccount<L, U>
{
    pub fn from_locked(locked: L) -> Self {
        Self::Locked(locked)
    }

    pub fn from_unlocked(unlocked: U) -> Self {
        Self::Unlocked(unlocked)
    }
}

// impl<L, U> From<L> for MaybeLockedAccount<L, U> {
//     fn from(locked: L) -> Self {
//         Self::Locked(locked)
//     }
// }

// impl<L, U> From<U> for MaybeLockedAccount<L, U> {
//     fn from(unlocked: U) -> Self {
//         Self::Unlocked(locked)
//     }
// }



#[tonic::async_trait]
pub trait WalletBehaviour<C: Crypto> {
    type Locked: UnlockableAccount<Unlocked = Self::Unlocked> + Send + Sync + 'static;
    type Unlocked: LockableAccount<Locked = Self::Locked> + AccountBehaviour<SigningAlgorithm = C> + Send + Sync + 'static;

    // We don't return a Result<&Self::Unlocked> for some api that takes &mut self.
    // Because a & obtained from &mut is still exclusive.

    async fn generate_account(&mut self, id: &str, pw: Option<&str>) -> Result<()>;
    async fn import_account(&mut self, id: &str, maybe_locked: MaybeLockedAccount<Self::Locked, Self::Unlocked>) -> Result<()>;
    async fn unlock_account(&mut self, id: &str, pw: &str) -> Result<()>;
    async fn delete_account(&mut self, id: &str) -> Result<()>;

    async fn get_account(&self, id: &str) -> Result<&Self::Unlocked>;
    // Return a Vec since GAT is unstable
    async fn list_account(&self) -> Vec<(&str, &MaybeLockedAccount<Self::Locked, Self::Unlocked>)>;

    async fn current_account(&self) -> Result<(&str, &Self::Unlocked)>;
    async fn set_current_account(&mut self, id: &str) -> Result<()>;
}


#[derive(Serialize, Deserialize)]
pub struct LockedAccount<C: Crypto> {
    address: C::Address,
    encrypted_sk: Vec<u8>,
}

impl<C: Crypto> UnlockableAccount for LockedAccount<C>
{
    type Unlocked = Account<C>;

    fn unlock(self, pw: &str) -> Result<Self::Unlocked, Self> {
        C::decrypt(self.encrypted_sk.as_slice(), pw.as_bytes())
            .and_then(|decrypted| {
                C::SecretKey::try_from_slice(&decrypted).ok()
            })
            .and_then(|sk| Account::<C>::from_secret_key(sk).into())
            .and_then(|unlocked| {
                if unlocked.address() == &self.address {
                    Some(unlocked)
                } else {
                    None
                }
            })
            .ok_or(self)
    }
}

impl<C: Crypto> LockableAccount for Account<C>
{
    type Locked = LockedAccount<C>;

    fn lock(self, pw: &str) -> Self::Locked {
        let encrypted_sk = C::encrypt(self.secret_key.as_slice(), pw.as_bytes());

        LockedAccount {
            address: self.address,
            encrypted_sk,
        }
    }
}


// TODO: use rustbreak/sled or something or just keep it?

// helper struct for serde
#[derive(Serialize, Deserialize)]
struct Locked {
    address: String,
    encrypted_sk: String,
}

impl<C: Crypto> TryFrom<Locked> for LockedAccount<C> {
    type Error = anyhow::Error;

    fn try_from(locked: Locked) -> Result<Self, Self::Error> {
        let address = parse_addr::<C>(&locked.address)?;
        let encrypted_sk = parse_data(&locked.encrypted_sk)?;

        Ok(LockedAccount {
            address,
            encrypted_sk
        })
    }
}

impl<C: Crypto> TryFrom<Unlocked> for Account<C> {
    type Error = anyhow::Error;

    fn try_from(unlocked: Unlocked) -> Result<Self, Self::Error> {
        let address = parse_addr::<C>(&unlocked.address)?;
        let sk = parse_sk::<C>(&unlocked.unencrypted_sk)?;

        let account = Account::from_secret_key(sk);
        ensure!(
            account.address() == &address,
            "account address mismatched with the recorded account address",
        );

        Ok(account)
    }
}

// helper struct for serde
#[derive(Serialize, Deserialize)]
struct Unlocked {
    address: String,
    unencrypted_sk: String,
}


pub struct Wallet<C: Crypto> {
    wallet_dir: PathBuf,

    current_account_id: Option<String>,
    account_map: BTreeMap<String, MaybeLockedAccount<LockedAccount<C>, Account<C>>>,
}

impl<C: Crypto> Wallet<C> {

    pub async fn open(wallet_dir: impl AsRef<Path>) -> Result<Self> {
        let wallet_dir = wallet_dir.as_ref().to_path_buf();
        let accounts_dir = wallet_dir.join("accounts");

        fs::create_dir_all(&accounts_dir).await.context("failed to create accounts dir")?;

        let mut this = Self {
            wallet_dir,
            current_account_id: None,
            account_map: BTreeMap::new(),
        };

        let mut it = fs::read_dir(accounts_dir).await.context("cannot read accounts dir")?;
        while let Ok(Some(ent)) = it.next_entry().await {
            let path = ent.path();
            let is_file = ent.file_type().await?.is_file();
            let is_toml = path.extension().map(|ext| ext == "toml").unwrap_or(false);

            if is_file && is_toml {
                let id = path.file_stem().context("cannot read account id from account file name")?;
                // TODO: log error
                let _ = this.load_account(&id.to_string_lossy()).await;
            }
        }

        Ok(this)
    }

    async fn save_account(&mut self, id: &str, maybe_locked: MaybeLockedAccount<LockedAccount<C>, Account<C>>) -> Result<()> {
        // TODO: validate id
        let accounts_dir = self.wallet_dir.join("accounts");
        fs::create_dir_all(&accounts_dir).await.context("cannot create directory for accounts")?;

        let mut account_file = {
            let account_file = accounts_dir.join(format!("{id}.toml"));
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(account_file)
                .await
                .context("cannot create account file")?
        };
        
        // TODO: use serialize_with in field
        let content = match &maybe_locked {
            MaybeLockedAccount::Locked(locked) => {
                let address = hex(locked.address.as_slice());
                let encrypted_sk = hex(locked.encrypted_sk.as_slice());
                let locked = Locked {
                    address,
                    encrypted_sk,
                };
                toml::to_string_pretty(&locked).unwrap()
            }
            MaybeLockedAccount::Unlocked(unlocked) => {
                let address = hex(unlocked.address().as_slice());
                let unencrypted_sk = hex(unlocked.expose_secret_key().as_slice());
                let unlocked = Unlocked {
                    address,
                    unencrypted_sk,
                };
                toml::to_string_pretty(&unlocked).unwrap()
            }
        };

        account_file.write_all(content.as_bytes()).await.context("cannot write account content")?;

        self.account_map.insert(id.into(), maybe_locked);

        Ok(())
    }

    async fn load_account(&mut self, id: &str) -> Result<()> {
        let content = {
            let path = self.wallet_dir.join("accounts").join(format!("{id}.toml"));
            fs::read_to_string(path).await.context("cannot read account file")?
        };

        if let Ok(unlocked) = toml::from_str::<Unlocked>(&content) {
            let account = Account::try_from(unlocked).context("invalid unlocked account")?;
            self.account_map.insert(id.into(), MaybeLockedAccount::from_unlocked(account));
        } else if let Ok(locked) = toml::from_str::<Locked>(&content) {
            let account = LockedAccount::try_from(locked).context("invalid locked account")?;
            self.account_map.insert(id.into(), MaybeLockedAccount::from_locked(account));
        } else {
            bail!("cannot load account from file, invalid format")
        }

        Ok(())
    }
}

#[tonic::async_trait]
impl<C: Crypto> WalletBehaviour<C> for Wallet<C> {
    type Locked = LockedAccount<C>;
    type Unlocked = Account<C>;

    async fn generate_account(&mut self, id: &str, pw: Option<&str>) -> Result<()> {
        let account = Account::generate();
        let maybe_locked = match pw {
            Some(pw) => MaybeLockedAccount::from_locked(account.lock(pw)),
            None => MaybeLockedAccount::from_unlocked(account),
        };
        self.save_account(id, maybe_locked).await
    }

    async fn import_account(&mut self, id: &str, maybe_locked: MaybeLockedAccount<Self::Locked, Self::Unlocked>) -> Result<()> {
        self.save_account(id, maybe_locked).await.context("cannot save imported account")
    }

    async fn unlock_account(&mut self, id: &str, pw: &str) -> Result<()> {
        let (id, account) = self.account_map.remove_entry(id).ok_or(anyhow!("account not found"))?;
        match account.unlock(pw) {
            Ok(account) => {
                self.account_map.insert(id, account);
            }
            Err(account) => {
                self.account_map.insert(id, account);
                bail!("failed to unlock account");
            }
        };

        Ok(())
    }

    async fn delete_account(&mut self, id: &str) -> Result<()> {
        todo!()
    }

    async fn get_account(&self, id: &str) -> Result<&Self::Unlocked> {
        match self.account_map.get(id) {
            Some(MaybeLockedAccount::Unlocked(unlocked)) => Ok(unlocked),
            Some(MaybeLockedAccount::Locked(..)) => bail!("account locked, please unlock it first"),
            None => bail!("account not found"),
        }
    }

    async fn list_account(&self) -> Vec<(&str, &MaybeLockedAccount<Self::Locked, Self::Unlocked>)> {
        self.account_map.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    async fn current_account(&self) -> Result<(&str, &Self::Unlocked)> {
        let id = self.current_account_id.as_ref().context("no current account selected")?;
        let account = self.get_account(&id).await?;
        Ok((id, account))
    }

    async fn set_current_account(&mut self, id: &str) -> Result<()> {
        self.current_account_id.replace(id.into());
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::EthCrypto;
    use tempfile::tempdir;

    async fn generate_and_load<C: Crypto>() -> Result<()> {
        let wallet_dir = tempdir()?;
        let mut w: Wallet<C> = Wallet::open(&wallet_dir).await?;
        w.generate_account("unlocked", None).await?;
        w.generate_account("locked", Some("pw")).await?;

        assert!(w.get_account("unlocked").await.is_ok());
        assert!(w.get_account("locked").await.is_err());

        w.unlock_account("locked", "pw").await?;
        assert!(w.get_account("locked").await.is_ok());

        // restart
        let mut w: Wallet<C> = Wallet::open(&wallet_dir).await?;
        assert!(w.get_account("unlocked").await.is_ok());
        assert!(w.get_account("locked").await.is_err());

        w.unlock_account("locked", "pw").await?;
        assert!(w.get_account("locked").await.is_ok());

        Ok(())
    }

    async fn save_and_load<C: Crypto>() -> Result<()> {
        let wallet_dir = tempdir()?;
        let mut w: Wallet<C> = Wallet::open(&wallet_dir).await?;

        let hex_sk = "0x05e86b1844ab3ab77adf6e2fe65c704fb3b49861626b140677a7a60f1b7b677a";

        let sk = parse_sk::<C>("0x05e86b1844ab3ab77adf6e2fe65c704fb3b49861626b140677a7a60f1b7b677a")?;
        let unlocked = Account::from_secret_key(sk);
        let sk = parse_sk::<C>("0x05e86b1844ab3ab77adf6e2fe65c704fb3b49861626b140677a7a60f1b7b677a")?;
        let locked = Account::from_secret_key(sk).lock("pw");
        w.save_account("unlocked", MaybeLockedAccount::from_unlocked(unlocked)).await?;
        w.save_account("locked", MaybeLockedAccount::from_locked(locked)).await?;

        assert!(w.get_account("unlocked").await.is_ok());
        assert!(w.get_account("locked").await.is_err());

        w.unlock_account("locked", "pw").await?;

        let account = w.get_account("locked").await?;
        assert_eq!(
            hex(account.expose_secret_key().as_slice()),
            hex_sk,
        );

        // restart
        let mut w: Wallet<C> = Wallet::open(&wallet_dir).await?;
        assert!(w.get_account("unlocked").await.is_ok());
        assert!(w.get_account("locked").await.is_err());

        w.unlock_account("locked", "pw").await?;

        let acc = w.get_account("locked").await?;
        assert_eq!(
            hex(acc.expose_secret_key().as_slice()),
            hex_sk,
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_basic() -> Result<()> {
        generate_and_load::<EthCrypto>().await?;
        save_and_load::<EthCrypto>().await?;
        Ok(())
    }
}
