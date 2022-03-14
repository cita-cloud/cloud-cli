// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use rustbreak::deser::Ron;
use rustbreak::PathDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use anyhow::ensure;
use anyhow::Result;

use crate::{
    core::wallet::Account as NewAccount,
    crypto::{ArrayLike, Crypto},
    utils::hex,
};

// TODO: encrypt it!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub addr: Vec<u8>,
    pub keypair: (Vec<u8>, Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalletInner {
    default_account: String,
    accounts: HashMap<String, Account>,
}

impl Default for WalletInner {
    fn default() -> Self {
        Self {
            default_account: "default".to_string(),
            accounts: HashMap::new(),
        }
    }
}

pub struct Wallet {
    db: PathDatabase<WalletInner, Ron>,
}

impl Wallet {
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self> {
        let db = PathDatabase::load_from_path(data_dir.as_ref().to_path_buf())?;
        Ok(Self { db })
    }
}

pub fn load_info_from_legacy_wallet<C: Crypto, P: AsRef<Path>>(
    path: P,
) -> Result<(String, HashMap<String, NewAccount<C>>)> {
    let wallet = Wallet::open(path)?;
    let w = wallet.db.borrow_data()?;
    let accounts = w.accounts
        .iter()
        .map(|(name, legacy_account)| {
            let addr = legacy_account.addr.clone();
            let sk = C::SecretKey::try_from_slice(&legacy_account.keypair.1)?;

            let new_account = NewAccount::<C>::from_secret_key(sk);
            ensure!(
                new_account.address() != addr.as_slice(),
                "failed to migrate legacy account `{}`, the recorded address `{}` mismatched with computed one `{}`", name, hex(&addr), hex(new_account.address())
            );
            Ok((name.clone(), new_account))
        })
        .collect::<Result<HashMap<String, NewAccount<C>>>>()?;
    Ok((w.default_account.to_owned(), accounts))
}
