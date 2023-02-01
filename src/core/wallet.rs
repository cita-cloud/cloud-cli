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

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::CryptoType,
    core::controller::SignerBehaviour,
    crypto::{Address, ArrayLike, Crypto, EthCrypto, SmCrypto},
    utils::{hex, parse_addr, parse_data, parse_pk, parse_sk, safe_save},
};

pub struct Account<C: Crypto> {
    address: Address,
    public_key: C::PublicKey,
    secret_key: C::SecretKey,
}

impl<C: Crypto> Account<C> {
    pub fn generate() -> Self {
        let (public_key, secret_key) = C::generate_keypair();
        let address = C::pk2addr(&public_key);

        Self {
            address,
            public_key,
            secret_key,
        }
    }

    #[allow(dead_code)]
    pub fn from_secret_key(sk: C::SecretKey) -> Self {
        let public_key = C::sk2pk(&sk);
        let address = C::pk2addr(&public_key);
        Self {
            address,
            public_key,
            secret_key: sk,
        }
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn public_key(&self) -> &C::PublicKey {
        &self.public_key
    }

    // TODO: maybe remove the `expose_`
    #[allow(dead_code)]
    pub fn expose_secret_key(&self) -> &C::SecretKey {
        &self.secret_key
    }

    pub fn sign(&self, msg: &[u8]) -> C::Signature {
        C::sign(msg, &self.secret_key)
    }

    pub fn lock(self, pw: &[u8]) -> LockedAccount<C> {
        let encrypted_sk = C::encrypt(self.secret_key.as_slice(), pw);
        LockedAccount {
            address: self.address,
            public_key: self.public_key,
            encrypted_sk,
        }
    }

    // We don't want to impl Serialize for it directly in case of leaking secret key without noticing.
    pub fn serialize_with_secret_key<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        SerializedAccount {
            address: hex(self.address.as_slice()),
            public_key: hex(self.public_key.as_slice()),
            secret_key: hex(self.secret_key.as_slice()),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        use serde::de::Unexpected;

        let serialized: SerializedAccount = Deserialize::deserialize(deserializer)?;
        let address = parse_addr(&serialized.address).map_err(|e| {
            D::Error::invalid_value(
                Unexpected::Str(&serialized.address),
                &e.to_string().as_str(),
            )
        })?;
        let public_key = parse_pk::<C>(&serialized.public_key).map_err(|e| {
            D::Error::invalid_value(
                Unexpected::Str(&serialized.public_key),
                &e.to_string().as_str(),
            )
        })?;
        let secret_key = parse_sk::<C>(&serialized.secret_key).map_err(|e| {
            D::Error::invalid_value(
                Unexpected::Str("/* secret-key omitted */"),
                &e.to_string().as_str(),
            )
        })?;

        if public_key != C::sk2pk(&secret_key) {
            return Err(D::Error::invalid_value(
                Unexpected::Str(&serialized.public_key),
                &"The serialized account's public key mismatched with the one computed from secret key. Data may be corrupted.",
            ));
        }
        if address != C::pk2addr(&public_key) {
            return Err(D::Error::invalid_value(
                Unexpected::Str(&serialized.address),
                    &"The serialized account's address mismatched with the one computed from public key. Data may be corrupted.",
            ));
        }

        Ok(Self {
            address,
            public_key,
            secret_key,
        })
    }
}

// We recorded the address and pubkey for better human-readability
#[derive(Serialize, Deserialize)]
struct SerializedAccount {
    address: String,
    public_key: String,
    secret_key: String,
}

impl<C: Crypto> TryFrom<SerializedAccount> for Account<C> {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedAccount) -> Result<Self, Self::Error> {
        let address = parse_addr(&serialized.address)?;
        let public_key = parse_pk::<C>(&serialized.public_key)?;
        let secret_key = parse_sk::<C>(&serialized.secret_key)?;

        ensure!(
            public_key == C::sk2pk(&secret_key),
            "The serialized account's public key mismatched with the one computed from secret key. Data may be corrupted.",
        );
        ensure!(
            address == C::pk2addr(&public_key),
            "The serialized account's address mismatched with the one computed from public key. Data may be corrupted.",
        );

        Ok(Self {
            address,
            public_key,
            secret_key,
        })
    }
}

#[derive(Deserialize)]
#[serde(try_from = "SerializedLockedAccount")]
pub struct LockedAccount<C: Crypto> {
    address: Address,
    public_key: C::PublicKey,
    encrypted_sk: Vec<u8>,
}

impl<C: Crypto> LockedAccount<C> {
    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn public_key(&self) -> &C::PublicKey {
        &self.public_key
    }

    pub fn unlock(&self, pw: &[u8]) -> Result<Account<C>> {
        let decrypted =
            C::decrypt(&self.encrypted_sk, pw).ok_or_else(|| anyhow!("invalid password"))?;
        let secret_key = C::SecretKey::try_from_slice(&decrypted)
            .map_err(|_| anyhow!("the decrypted secret key is invalid"))?;
        let public_key = C::sk2pk(&secret_key);
        let address = C::pk2addr(&public_key);

        ensure!(
            public_key == self.public_key,
            "The public key computed from the unlocked account mismatch with the recorded one"
        );
        ensure!(
            address == self.address,
            "The address computed from the unlocked account mismatch with the recorded one"
        );

        Ok(Account {
            address,
            public_key,
            secret_key,
        })
    }

    // We don't want to impl Serialize for it directly in case of leaking secret key without noticing.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedLockedAccount {
            address: hex(self.address.as_slice()),
            public_key: hex(self.public_key.as_slice()),
            encrypted_sk: hex(self.encrypted_sk.as_slice()),
        }
        .serialize(serializer)
    }

    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        use serde::de::Unexpected;

        let serialized: SerializedLockedAccount = Deserialize::deserialize(deserializer)?;
        let address = parse_addr(&serialized.address).map_err(|e| {
            D::Error::invalid_value(
                Unexpected::Str(&serialized.address),
                &e.to_string().as_str(),
            )
        })?;
        let public_key = parse_pk::<C>(&serialized.public_key).map_err(|e| {
            D::Error::invalid_value(
                Unexpected::Str(&serialized.public_key),
                &e.to_string().as_str(),
            )
        })?;
        let encrypted_sk = parse_data(&serialized.encrypted_sk).map_err(|e| {
            D::Error::invalid_value(
                Unexpected::Str(&serialized.encrypted_sk),
                &e.to_string().as_str(),
            )
        })?;

        if address != C::pk2addr(&public_key) {
            return Err(D::Error::invalid_value(
                Unexpected::Str(&serialized.address),
                &"the serialized account's address mismatched with the one computed from public key",
            ));
        }

        Ok(Self {
            address,
            public_key,
            encrypted_sk,
        })
    }
}

// We recorded the address and pubkey for better human-readability
#[derive(Serialize, Deserialize)]
struct SerializedLockedAccount {
    address: String,
    public_key: String,
    encrypted_sk: String,
}

impl<C: Crypto> TryFrom<SerializedLockedAccount> for LockedAccount<C> {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedLockedAccount) -> Result<Self, Self::Error> {
        let address = parse_addr(&serialized.address)?;
        let public_key = parse_pk::<C>(&serialized.public_key)?;
        let encrypted_sk = parse_data(&serialized.encrypted_sk)?;

        ensure!(
            address == C::pk2addr(&public_key),
            "The serialized account's address mismatched with the one computed from public key. Data may be corrupted.",
        );

        Ok(Self {
            address,
            public_key,
            encrypted_sk,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "crypto_type")]
#[serde(rename_all = "UPPERCASE")]
pub enum MultiCryptoAccount {
    Sm(
        #[serde(
            serialize_with = "Account::serialize_with_secret_key",
            deserialize_with = "Account::deserialize"
        )]
        Account<SmCrypto>,
    ),
    Eth(
        #[serde(
            serialize_with = "Account::serialize_with_secret_key",
            deserialize_with = "Account::deserialize"
        )]
        Account<EthCrypto>,
    ),
}

impl MultiCryptoAccount {
    pub fn address(&self) -> &Address {
        match self {
            Self::Sm(ac) => ac.address(),
            Self::Eth(ac) => ac.address(),
        }
    }

    pub fn public_key(&self) -> &[u8] {
        match self {
            Self::Sm(ac) => ac.public_key().as_slice(),
            Self::Eth(ac) => ac.public_key().as_slice(),
        }
    }

    pub fn crypto_type(&self) -> CryptoType {
        match self {
            Self::Sm(..) => CryptoType::Sm,
            Self::Eth(..) => CryptoType::Eth,
        }
    }

    pub fn lock(self, pw: &[u8]) -> LockedMultiCryptoAccount {
        match self {
            Self::Sm(ac) => LockedMultiCryptoAccount::Sm(ac.lock(pw)),
            Self::Eth(ac) => LockedMultiCryptoAccount::Eth(ac.lock(pw)),
        }
    }
}

impl From<Account<SmCrypto>> for MultiCryptoAccount {
    fn from(account: Account<SmCrypto>) -> Self {
        Self::Sm(account)
    }
}

impl From<Account<EthCrypto>> for MultiCryptoAccount {
    fn from(account: Account<EthCrypto>) -> Self {
        Self::Eth(account)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "crypto_type")]
#[serde(rename_all = "UPPERCASE")]
pub enum LockedMultiCryptoAccount {
    Sm(#[serde(with = "LockedAccount")] LockedAccount<SmCrypto>),
    Eth(#[serde(with = "LockedAccount")] LockedAccount<EthCrypto>),
}

impl LockedMultiCryptoAccount {
    pub fn address(&self) -> &Address {
        match self {
            Self::Sm(ac) => ac.address(),
            Self::Eth(ac) => ac.address(),
        }
    }

    pub fn public_key(&self) -> &[u8] {
        match self {
            Self::Sm(ac) => ac.public_key().as_slice(),
            Self::Eth(ac) => ac.public_key().as_slice(),
        }
    }

    pub fn crypto_type(&self) -> CryptoType {
        match self {
            Self::Sm(..) => CryptoType::Sm,
            Self::Eth(..) => CryptoType::Eth,
        }
    }

    pub fn unlock(&self, pw: &[u8]) -> Result<MultiCryptoAccount> {
        let unlocked = match self {
            Self::Sm(ac) => MultiCryptoAccount::Sm(ac.unlock(pw)?),
            Self::Eth(ac) => MultiCryptoAccount::Eth(ac.unlock(pw)?),
        };
        Ok(unlocked)
    }
}

impl From<LockedAccount<SmCrypto>> for LockedMultiCryptoAccount {
    fn from(locked: LockedAccount<SmCrypto>) -> Self {
        Self::Sm(locked)
    }
}

impl From<LockedAccount<EthCrypto>> for LockedMultiCryptoAccount {
    fn from(locked: LockedAccount<EthCrypto>) -> Self {
        Self::Eth(locked)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaybeLocked {
    Unlocked(MultiCryptoAccount),
    Locked(LockedMultiCryptoAccount),
}

impl MaybeLocked {
    pub fn address(&self) -> &Address {
        match self {
            Self::Unlocked(unlocked) => unlocked.address(),
            Self::Locked(locked) => locked.address(),
        }
    }

    pub fn public_key(&self) -> &[u8] {
        match self {
            Self::Unlocked(unlocked) => unlocked.public_key(),
            Self::Locked(locked) => locked.public_key(),
        }
    }

    pub fn is_locked(&self) -> bool {
        match self {
            Self::Unlocked(..) => false,
            Self::Locked(..) => true,
        }
    }

    #[allow(dead_code)]
    pub fn lock(self, pw: &[u8]) -> LockedMultiCryptoAccount {
        match self {
            Self::Unlocked(unlocked) => unlocked.lock(pw),
            Self::Locked(locked) => locked,
        }
    }

    pub fn unlock(&self, pw: &[u8]) -> Result<MultiCryptoAccount> {
        // manually clone here to avoid impl Clone for sensitive data
        fn cloned<C: Crypto>(ac: &Account<C>) -> Account<C> {
            Account::<C> {
                address: ac.address,
                public_key: ac.public_key.clone(),
                secret_key: ac.secret_key.clone(),
            }
        }
        match self {
            Self::Unlocked(unlocked) => match unlocked {
                MultiCryptoAccount::Eth(ac) => Ok(cloned(ac).into()),
                MultiCryptoAccount::Sm(ac) => Ok(cloned(ac).into()),
            },
            Self::Locked(locked) => locked.unlock(pw),
        }
    }

    pub fn unlocked(&self) -> Result<&MultiCryptoAccount> {
        match self {
            Self::Unlocked(ac) => Ok(ac),
            Self::Locked(_) => bail!(
                "account is locked, please unlock it first(e.g. `cldi -p <password> [subcommand]`)"
            ),
        }
    }

    pub fn crypto_type(&self) -> CryptoType {
        match self {
            Self::Unlocked(unlocked) => unlocked.crypto_type(),
            Self::Locked(locked) => locked.crypto_type(),
        }
    }
}

impl From<Account<SmCrypto>> for MaybeLocked {
    fn from(account: Account<SmCrypto>) -> Self {
        MultiCryptoAccount::from(account).into()
    }
}

impl From<Account<EthCrypto>> for MaybeLocked {
    fn from(account: Account<EthCrypto>) -> Self {
        MultiCryptoAccount::from(account).into()
    }
}

impl From<MultiCryptoAccount> for MaybeLocked {
    fn from(unlocked: MultiCryptoAccount) -> Self {
        Self::Unlocked(unlocked)
    }
}

impl From<LockedAccount<SmCrypto>> for MaybeLocked {
    fn from(locked: LockedAccount<SmCrypto>) -> Self {
        LockedMultiCryptoAccount::from(locked).into()
    }
}

impl From<LockedAccount<EthCrypto>> for MaybeLocked {
    fn from(locked: LockedAccount<EthCrypto>) -> Self {
        LockedMultiCryptoAccount::from(locked).into()
    }
}

impl From<LockedMultiCryptoAccount> for MaybeLocked {
    fn from(locked: LockedMultiCryptoAccount) -> Self {
        Self::Locked(locked)
    }
}

impl<C: Crypto> SignerBehaviour for Account<C> {
    fn hash(&self, msg: &[u8]) -> Vec<u8> {
        C::hash(msg).to_vec()
    }

    fn address(&self) -> &[u8] {
        self.address.as_slice()
    }

    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        Self::sign(self, msg).to_vec()
    }
}

impl SignerBehaviour for MultiCryptoAccount {
    fn hash(&self, msg: &[u8]) -> Vec<u8> {
        match self {
            Self::Sm(ac) => ac.hash(msg),
            Self::Eth(ac) => ac.hash(msg),
        }
    }

    fn address(&self) -> &[u8] {
        match self {
            Self::Sm(ac) => ac.address(),
            Self::Eth(ac) => ac.address(),
        }
    }

    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        match self {
            Self::Sm(ac) => <Account<SmCrypto> as SignerBehaviour>::sign(ac, msg),
            Self::Eth(ac) => <Account<EthCrypto> as SignerBehaviour>::sign(ac, msg),
        }
    }
}

pub struct Wallet {
    wallet_dir: PathBuf,
    accounts: BTreeMap<String, MaybeLocked>,
}

impl Wallet {
    const ACCOUNTS_DIR: &'static str = "accounts";

    pub fn open(wallet_dir: impl AsRef<Path>) -> Result<Self> {
        let wallet_dir = wallet_dir.as_ref().to_path_buf();
        let accounts_dir = wallet_dir.join(Self::ACCOUNTS_DIR);

        fs::create_dir_all(&accounts_dir).context("failed to create accounts dir")?;

        let mut this = Self {
            wallet_dir,
            accounts: BTreeMap::new(),
        };

        let dir = fs::read_dir(accounts_dir).context("cannot read accounts dir")?;
        for ent in dir {
            let ent = ent.context("cannot read account file")?;
            let path = ent.path();
            let is_file = ent.file_type()?.is_file();
            let is_toml = path.extension().map(|ext| ext == "toml").unwrap_or(false);

            if is_file && is_toml {
                let name = path
                    .file_stem()
                    .context("cannot read account name from account file name")?
                    .to_string_lossy();

                if let Err(e) = this.load(&name) {
                    // TODO: use logger
                    eprintln!("fail to load account `{name}`: {e}");
                }
            }
        }

        Ok(this)
    }

    fn load(&mut self, account_name: &str) -> Result<()> {
        let content = {
            let accounts_dir = self.wallet_dir.join(Self::ACCOUNTS_DIR);
            let path = accounts_dir.join(format!("{account_name}.toml"));
            fs::read_to_string(path).context("cannot read account file")?
        };

        let maybe_locked: MaybeLocked = toml::from_str(&content)?;
        self.accounts.insert(account_name.into(), maybe_locked);

        Ok(())
    }

    pub fn get(&self, account_name: &str) -> Result<&MaybeLocked> {
        self.accounts
            .get(account_name)
            .ok_or_else(|| anyhow!("account `{}` not found", account_name))
    }

    pub fn list(&self) -> impl Iterator<Item = (&String, &MaybeLocked)> {
        self.accounts.iter()
    }

    /// Save account both in memory and to keystore.
    /// Return error if the account file already exists.
    pub fn save(
        &mut self,
        account_name: String,
        maybe_locked: impl Into<MaybeLocked>,
    ) -> Result<()> {
        let maybe_locked = maybe_locked.into();
        Self::save_account_to_keystore(&self.wallet_dir, &account_name, &maybe_locked, false)?;
        self.accounts.insert(account_name, maybe_locked);
        Ok(())
    }

    /// Same as [`save_to_keystore`], but overwrites existing account file.
    pub fn save_overwrite(
        &mut self,
        account_name: String,
        maybe_locked: impl Into<MaybeLocked>,
    ) -> Result<()> {
        let maybe_locked = maybe_locked.into();
        Self::save_account_to_keystore(&self.wallet_dir, &account_name, &maybe_locked, true)?;
        self.accounts.insert(account_name, maybe_locked);
        Ok(())
    }

    /// Save account in memory without writing to keystore.
    /// If an account with the same name exists, that will be replaced.
    /// Use [`save`] or [`save_overwrite`] if you want to save account to keystore.
    pub fn save_in_memory(&mut self, account_name: String, maybe_locked: impl Into<MaybeLocked>) {
        let maybe_locked = maybe_locked.into();
        self.accounts.insert(account_name, maybe_locked);
    }

    fn save_account_to_keystore(
        wallet_dir: impl AsRef<Path>,
        account_name: &str,
        maybe_locked: &MaybeLocked,
        overwrite_existing: bool,
    ) -> Result<()> {
        let wallet_dir = wallet_dir.as_ref();
        let accounts_dir = wallet_dir.join(Self::ACCOUNTS_DIR);
        let account_file = accounts_dir.join(format!("{account_name}.toml"));

        let content = toml::to_string_pretty(&maybe_locked)?;
        safe_save(account_file, content.as_bytes(), overwrite_existing)?;
        Ok(())
    }

    /// Remove account from both memory and keystore.
    pub fn remove(&mut self, account_name: &str) -> Result<()> {
        self.get(account_name)?;
        Self::remove_account_from_keystore(&self.wallet_dir, account_name)?;
        self.accounts.remove(account_name).unwrap();

        Ok(())
    }

    // FIXME: It's not secure, secret key data may be still in the disk.
    fn remove_account_from_keystore(
        wallet_dir: impl AsRef<Path>,
        account_name: &str,
    ) -> Result<()> {
        let wallet_dir = wallet_dir.as_ref();
        let accounts_dir = wallet_dir.join(Self::ACCOUNTS_DIR);
        let account_file = accounts_dir.join(format!("{account_name}.toml"));

        fs::remove_file(account_file)?;
        Ok(())
    }

    /// Lock the account in both memory and keystore.
    pub fn lock(&mut self, account_name: &str, pw: &[u8]) -> Result<()> {
        let (account_name, maybe_locked) = self
            .accounts
            .remove_entry(account_name)
            .ok_or_else(|| anyhow!("account `{}` not found", account_name))?;
        let locked: MaybeLocked = maybe_locked.lock(pw).into();
        self.save_overwrite(account_name, locked)?;

        Ok(())
    }

    /// Lock the account in memory.
    pub fn lock_in_memory(&mut self, account_name: &str, pw: &[u8]) -> Result<()> {
        let (account_name, maybe_locked) = self
            .accounts
            .remove_entry(account_name)
            .ok_or_else(|| anyhow!("account `{}` not found", account_name))?;
        let locked: MaybeLocked = maybe_locked.lock(pw).into();
        self.save_in_memory(account_name, locked);

        Ok(())
    }

    /// Unlock the account in memory. File in keystore is unchanged.
    /// Usually wallet modification operations without in_memory suffix affects keystore,
    /// but in common use cases, unlock is intended to unlock the account in memory.
    /// So this is the only exception.
    pub fn unlock(&mut self, account_name: &str, pw: &[u8]) -> Result<()> {
        let maybe_locked = self
            .accounts
            .get_mut(account_name)
            .ok_or_else(|| anyhow!("account `{}` not found", account_name))?;
        *maybe_locked = maybe_locked.unlock(pw)?.into();

        Ok(())
    }

    /// Unlock the account in both memory and keystore.
    pub fn unlock_in_keystore(&mut self, account_name: &str, pw: &[u8]) -> Result<()> {
        let maybe_locked = self
            .accounts
            .get_mut(account_name)
            .ok_or_else(|| anyhow!("account `{}` not found", account_name))?;
        *maybe_locked = maybe_locked.unlock(pw)?.into();

        Self::save_account_to_keystore(&self.wallet_dir, account_name, maybe_locked, true)
    }
}
