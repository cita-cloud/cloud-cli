// use std::collections::HashMap;

use rustbreak::deser::Ron;
use rustbreak::PathDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Result;

use crate::crypto::Crypto;

// TODO: encrypt it!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account<C: Crypto> {
    pub addr: C::Address,
    pub keypair: (C::PublicKey, C::SecretKey),
}

impl<C: Crypto> Account<C> {
    pub fn sign(&self, msg: &[u8]) -> C::Signature {
        C::sign(msg, &self.keypair.1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalletInner<C> {
    default_account: String,
    accounts: HashMap<String, Account<C>>,
}

impl<C> Default for WalletInner<C> {
    fn default() -> Self {
        Self {
            default_account: "default".to_string(),
            accounts: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Wallet<C: Crypto> {
    db: Arc<PathDatabase<WalletInner<C>, Ron>>,
}

// FIXME:
// Those db operations may block the async runtime, but that doesn't matter for now.
// We can solve this by using tokio::spawn_blocking.
impl<C: Crypto> Wallet<C> {
    pub fn open(data_dir: impl AsRef<Path>) -> Self {
        let db = PathDatabase::load_from_path_or_default(data_dir.as_ref().to_path_buf()).unwrap();
        Self { db: Arc::new(db) }
    }

    pub fn load_account(&self, account_id: &str) -> Option<Account<C>> {
        self.db
            .borrow_data()
            .unwrap()
            .accounts
            .get(account_id)
            .cloned()
    }

    pub fn store_account(&self, account_id: &str, account: Account<C>) {
        self.db
            .borrow_data_mut()
            .unwrap()
            .accounts
            .insert(account_id.to_string(), account);
        self.db.save().unwrap();
    }

    pub fn create_account(&self, account_id: &str) -> Vec<u8> {
        let account = {
            let (pk, sk) = C::gen_keypair();
            let addr = C::pk2addr(&pk);
            Account {
                addr,
                keypair: (pk, sk),
            }
        };
        let addr = account.addr.clone();

        self.db
            .borrow_data_mut()
            .unwrap()
            .accounts
            .insert(account_id.to_string(), account);
        self.db.save().unwrap();

        addr
    }

    pub fn delete_account(&self, account_id: &str) {
        self.db
            .write(|w| {
                if w.default_account == account_id {
                    w.default_account = "default".to_string();
                }
                w.accounts.remove(account_id);
            })
            .unwrap();
        self.db.save().unwrap();
    }

    pub fn list_account(&self) -> Vec<(String, Vec<u8>)> {
        self.db
            .borrow_data()
            .unwrap()
            .accounts
            .iter()
            .map(|(k, v)| (k.clone(), v.addr.clone()))
            .collect()
    }

    pub fn import_account(&self, account_id: &str, pk: Vec<u8>, sk: Vec<u8>) {
        let account = {
            let addr = C::pk2addr(&pk);
            Account {
                addr,
                keypair: (pk, sk),
            }
        };
        self.store_account(account_id, account);
    }

    pub fn set_default_account(&self, account_id: &str) -> Result<Vec<u8>> {
        let mut wallet = self.db.borrow_data_mut().unwrap();
        if let Some(account) = wallet.accounts.get(account_id) {
            let addr = account.addr.clone();

            wallet.default_account = account_id.to_string();

            // It's actually a rwlock, so drop it here to avoid deadlock with save.
            drop(wallet);
            self.db.save()?;

            Ok(addr)
        } else {
            Err(anyhow!("user doesn't exist"))
        }
    }

    pub fn default_account(&self) -> Result<Account<C>> {
        let wallet = self.db.borrow_data()?;
        if let Some(account) = wallet.accounts.get(&wallet.default_account) {
            Ok(account.clone())
        } else {
            // avoid deadlock
            drop(wallet);
            self.create_account("default");
            Ok(self.load_account("default").unwrap())
        }
    }
}
