use std::{path::{PathBuf, Path}, collections::{BTreeSet, BTreeMap}};
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;

use crate::crypto::{ArrayLike, Crypto, SmCrypto, EthCrypto};
use serde::{Deserialize, Serialize};

use super::controller::SignerBehaviour;

mod hex_repr {
    use serde::Serializer;
    use serde::Deserializer;
    use super::ArrayLike;
    use serde::de;
    use serde::de::Visitor;
    use crate::utils::hex;
    use crate::utils::parse_data;
    use std::fmt;
    use std::marker::PhantomData;

    pub fn serialize<T, S>(array: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: ArrayLike,
        S: Serializer,
    {
        let hex_s = hex(array.as_slice());
        serializer.serialize_str(&hex_s)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: ArrayLike,
        D: Deserializer<'de>,
    {
        struct HexVisitor<T>(PhantomData<fn() -> T>);
        impl<'de, V: ArrayLike> Visitor<'de> for HexVisitor<V> {
            type Value = V;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "A crypto specific hex-encoded bit array")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> 
            {
                let err_fn = |_| de::Error::invalid_value(de::Unexpected::Str(v),&self);
                let d = parse_data(v).map_err(err_fn)?;
                let value = V::try_from_slice(&d).map_err(err_fn)?;

                Ok(value)
            }
        }

        deserializer.deserialize_str(HexVisitor::<T>(PhantomData))
    }
}

#[derive(Serialize, Deserialize)]
pub struct Account<C: Crypto> {
    #[serde(with = "hex_repr")]
    address: C::Address,
    #[serde(with = "hex_repr")]
    public_key: C::PublicKey,
    #[serde(with = "hex_repr")]
    secret_key: C::SecretKey,
}

impl<C: Crypto> Account<C> {
    fn generate() -> Self {
        let (public_key, secret_key) = C::generate_keypair();
        let address = C::pk2addr(&public_key);

        Self {
            address,
            public_key,
            secret_key,
        }
    }

    fn from_secret_key(sk: C::SecretKey) -> Self {
        let public_key = C::sk2pk(&sk);
        let address = C::pk2addr(&public_key);
        Self {
            address,
            public_key,
            secret_key: sk,
        }
    }

    fn sign(&self, msg: &[u8]) -> C::Signature {
        C::sign(msg, &self.secret_key)
    }

    fn lock(self, pw: &[u8]) -> LockedAccount<C> {
        let encrypted_sk = C::encrypt(self.secret_key.as_slice(), pw);
        LockedAccount {
            address: self.address,
            public_key: self.public_key,
            encrypted_sk,
        }

    }
}


#[derive(Serialize, Deserialize)]
pub struct LockedAccount<C: Crypto> {
    #[serde(with = "hex_repr")]
    address: C::Address,
    #[serde(with = "hex_repr")]
    public_key: C::PublicKey,
    #[serde(with = "hex_repr")]
    encrypted_sk: Vec<u8>,
}

impl<C: Crypto> LockedAccount<C> {
    pub fn unlock(self, pw: &[u8]) -> Result<Account<C>> {
        let decrypted = C::decrypt(&self.encrypted_sk, pw).ok_or(anyhow!("invalid password"))?;
        let secret_key = C::SecretKey::try_from_slice(&decrypted)?;
        let public_key = C::sk2pk(&secret_key);
        let address = C::pk2addr(&public_key);
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
}


#[derive(Serialize, Deserialize)]
#[serde(tag = "crypto_type")]
pub enum MultiCryptoAccount {
    Sm(Account<SmCrypto>),
    Eth(Account<EthCrypto>),
}

impl MultiCryptoAccount {
    pub fn lock(self, pw: &[u8]) -> MultiCryptoLockedAccount {
        match self {
            Self::Sm(ac) => MultiCryptoLockedAccount::Sm(ac.lock(pw)),
            Self::Eth(ac) => MultiCryptoLockedAccount::Eth(ac.lock(pw)),
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
pub enum MultiCryptoLockedAccount {
    Sm(LockedAccount<SmCrypto>),
    Eth(LockedAccount<EthCrypto>),
}

impl MultiCryptoLockedAccount {
    pub fn unlock(self, pw: &[u8]) -> Result<MultiCryptoAccount> {
        let unlocked = match self {
            Self::Sm(ac) => MultiCryptoAccount::Sm(ac.unlock(pw)?),
            Self::Eth(ac) => MultiCryptoAccount::Eth(ac.unlock(pw)?),
        };
        Ok(unlocked)
    }
}

impl From<LockedAccount<SmCrypto>> for MultiCryptoLockedAccount {
    fn from(locked: LockedAccount<SmCrypto>) -> Self {
        Self::Sm(locked)
    }
}

impl From<LockedAccount<EthCrypto>> for MultiCryptoLockedAccount {
    fn from(locked: LockedAccount<EthCrypto>) -> Self {
        Self::Eth(locked)
    }
}


#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaybeLocked {
    Unlocked(MultiCryptoAccount),
    Locked(MultiCryptoLockedAccount),
}

impl MaybeLocked {
    pub fn unlocked(&self) -> Option<&MultiCryptoAccount> {
        match self {
            Self::Unlocked(ac) => Some(ac),
            Self::Locked(_) => None,
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
        MultiCryptoLockedAccount::from(locked).into()
    }
}

impl From<LockedAccount<EthCrypto>> for MaybeLocked {
    fn from(locked: LockedAccount<EthCrypto>) -> Self {
        MultiCryptoLockedAccount::from(locked).into()
    }
}

impl From<MultiCryptoLockedAccount> for MaybeLocked {
    fn from(locked: MultiCryptoLockedAccount) -> Self {
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
    account_map: BTreeMap<String, MaybeLocked>,
}

impl Wallet {
    const ACCOUNTS_DIR: &'static str = "accounts";

    pub fn open(wallet_dir: impl AsRef<Path>) -> Result<Self> {
        let wallet_dir = wallet_dir.as_ref().to_path_buf();

        todo!()
    }

    pub fn save(&mut self, id: &str, maybe_locked: impl Into<MaybeLocked>) -> Result<()> {

        todo!()
    }

    pub fn load(&mut self, id: &str) -> Result<MaybeLocked> {

        todo!()
    }

    pub fn get(&self, id: &str) -> Option<MaybeLocked> {
        todo!()
    }

    pub fn unlock(&mut self, id: &str, pw: &[u8]) -> Result<()> {

        todo!()
    }

    pub fn list(&self) -> impl Iterator<Item = (&String, &MaybeLocked)> {
        self.account_map.iter()
    }
}
