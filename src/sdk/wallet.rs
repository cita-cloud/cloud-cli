use std::{path::{PathBuf, Path}, collections::{BTreeSet, BTreeMap}};
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use anyhow::bail;

use crate::{crypto::{ArrayLike, Crypto, SmCrypto, EthCrypto, Address, Hash}, utils::{parse_addr, parse_pk, parse_sk, parse_data}};
use serde::{Deserialize, Serialize};
use serde::Serializer;
use serde::Deserializer;
use std::fs;
use crate::utils::hex;
use anyhow::Context;
use crate::utils::safe_save;

use super::controller::SignerBehaviour;

// // TODO: use a simpler impl, this is too complex
// mod hex_repr {
//     use serde::Serializer;
//     use serde::Deserializer;
//     use super::ArrayLike;
//     use serde::de;
//     use serde::de::Visitor;
//     use crate::utils::hex;
//     use crate::utils::parse_data;
//     use std::fmt;
//     use std::marker::PhantomData;

//     pub fn serialize<T, S>(array: &T, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         T: ArrayLike,
//         S: Serializer,
//     {
//         let hex_s = hex(array.as_slice());
//         serializer.serialize_str(&hex_s)
//     }

//     pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
//     where
//         T: ArrayLike,
//         D: Deserializer<'de>,
//     {
//         struct HexVisitor<T>(PhantomData<fn() -> T>);
//         impl<'de, V: ArrayLike> Visitor<'de> for HexVisitor<V> {
//             type Value = V;

//             fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
//                 write!(formatter, "A crypto specific hex-encoded bit array")
//             }

//             fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> 
//             {
//                 let err_fn = |_| de::Error::invalid_value(de::Unexpected::Str(v),&self);
//                 let d = parse_data(v).map_err(err_fn)?;
//                 let value = V::try_from_slice(&d).map_err(err_fn)?;

//                 Ok(value)
//             }
//         }

//         deserializer.deserialize_str(HexVisitor::<T>(PhantomData))
//     }
// }


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
    pub fn serialize_with_secret_key<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        SerializedAccount {
            address: hex(self.address.as_slice()),
            public_key: hex(self.public_key.as_slice()),
            secret_key: hex(self.secret_key.as_slice()),
        }.serialize(serializer)
    }

    pub fn deserialize<'de, D:  Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    {
        use serde::de::Unexpected;
        use serde::de::Error;

        let serialized: SerializedAccount = Deserialize::deserialize(deserializer)?;
        let address = parse_addr(&serialized.address)
            .map_err(|e|{
                D::Error::invalid_value(Unexpected::Str(&serialized.address), &e.to_string().as_str())
            })?;
        let public_key = parse_pk::<C>(&serialized.public_key)
            .map_err(|e|{
                D::Error::invalid_value(Unexpected::Str(&serialized.public_key), &e.to_string().as_str())
            })?;
        let secret_key = parse_sk::<C>(&serialized.secret_key)
            .map_err(|e|{
                D::Error::invalid_value(Unexpected::Str("/* secret-key omitted */"), &e.to_string().as_str())
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

    // fn from_str(s: &str) -> Result<Self> {
    //     let (address, public_key, secret_key) = {
    //         let Serialized{ address, public_key, secret_key } = toml::from_str(s).context("cannot parse Account")?;
    //         let address = parse_addr(address)?;
    //         let public_key = parse_pk(address)?;
    //         let secret_key = parse_sk(secret_key)?;
    //         (address, public_key, secret_key)
    //     };
    //     ensure!(
    //         public_key == C::sk2pk(&secret_key),
    //         "The parsed account's recorded "
    //     )

    // }

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
        let decrypted = C::decrypt(&self.encrypted_sk, pw).ok_or(anyhow!("invalid password"))?;
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
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        SerializedLockedAccount {
            address: hex(self.address.as_slice()),
            public_key: hex(self.public_key.as_slice()),
            encrypted_sk: hex(self.encrypted_sk.as_slice()),
        }.serialize(serializer)
    }

    fn deserialize<'de, D:  Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    {
        use serde::de::Unexpected;
        use serde::de::Error;

        let serialized: SerializedLockedAccount = Deserialize::deserialize(deserializer)?;
        let address = parse_addr(&serialized.address)
            .map_err(|e|{
                D::Error::invalid_value(Unexpected::Str(&serialized.address), &e.to_string().as_str())
            })?;
        let public_key = parse_pk::<C>(&serialized.public_key)
            .map_err(|e|{
                D::Error::invalid_value(Unexpected::Str(&serialized.public_key), &e.to_string().as_str())
            })?;
        let encrypted_sk = parse_data(&serialized.encrypted_sk)
            .map_err(|e|{
                D::Error::invalid_value(Unexpected::Str(&serialized.encrypted_sk), &e.to_string().as_str())
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
pub enum MultiCryptoAccount {
    Sm(
        #[serde(
            serialize_with = "Account::serialize_with_secret_key",
            deserialize_with = "Account::deserialize",
        )]
        Account<SmCrypto>
    ),
    Eth(
        #[serde(
            serialize_with = "Account::serialize_with_secret_key",
            deserialize_with = "Account::deserialize",
        )]
        Account<EthCrypto>
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
pub enum LockedMultiCryptoAccount {
    Sm(
        #[serde(with = "LockedAccount")]
        LockedAccount<SmCrypto>
    ),
    Eth(
        #[serde(with = "LockedAccount")]
        LockedAccount<EthCrypto>
    ),
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
            Self::Unlocked(unlocked) => {
                match unlocked {
                    MultiCryptoAccount::Eth(ac) => Ok(cloned(ac).into()),
                    MultiCryptoAccount::Sm(ac) => Ok(cloned(ac).into()),
                }
            },
            Self::Locked(locked) => locked.unlock(pw),
        }
    }

    pub fn unlocked(&self) -> Result<&MultiCryptoAccount> {
        match self {
            Self::Unlocked(ac) => Ok(ac),
            Self::Locked(_) => bail!("account is locked, please unlock it first"),
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

        fs::create_dir(&accounts_dir)
            .context("failed to create accounts dir")?;

        let mut this = Self {
            wallet_dir,
            accounts: BTreeMap::new(),
        };

        let dir = fs::read_dir(accounts_dir)
            .context("cannot read accounts dir")?;
        for ent in dir {
            let ent = ent.context("cannot read account file")?;
            let path = ent.path();
            let is_file = ent.file_type()?.is_file();
            let is_toml = path.extension().map(|ext| ext == "toml").unwrap_or(false);

            if is_file && is_toml {
                let id = path
                    .file_stem()
                    .context("cannot read account id from account file name")?;
                // TODO: log error
                let _ = this.load(&id.to_string_lossy());
            }
        }

        Ok(this)
    }

    pub fn save(&mut self, account_id: &str, maybe_locked: impl Into<MaybeLocked>) -> Result<()> {
        let maybe_locked = maybe_locked.into();
         
        let accounts_dir = self.wallet_dir.join(Self::ACCOUNTS_DIR);
        let account_file = accounts_dir.join(format!("{account_id}.toml"));

        let content = toml::to_string_pretty(&maybe_locked)?;
        safe_save(account_file, content.as_bytes(), false)?;

        self.accounts.insert(account_id.into(), maybe_locked);
        Ok(())
    }

    fn load(&mut self, account_id: &str) -> Result<()> {
        let content = {
            let accounts_dir = self.wallet_dir.join(Self::ACCOUNTS_DIR);
            let path = accounts_dir.join(format!("{account_id}.toml"));
            fs::read_to_string(path)
                .context("cannot read account file")?
        };

        let maybe_locked: MaybeLocked = toml::from_str(&content)?;
        self.accounts.insert(account_id.into(), maybe_locked);

        Ok(())
    }

    pub fn get(&self, account_id: &str) -> Option<&MaybeLocked> {
        self.accounts.get(account_id)
    }

    pub fn lock(&mut self, account_id: &str, pw: &[u8]) -> Result<()> {
        let (id, maybe_locked) = self.accounts.remove_entry(account_id)
            .ok_or_else(|| anyhow!("account `{}` not found", account_id))?;
        let locked = maybe_locked.lock(pw);
        self.accounts.insert(id, locked.into());

        Ok(())
    }

    pub fn unlock(&mut self, account_id: &str, pw: &[u8]) -> Result<()> {
        let maybe_locked = self.accounts.get_mut(account_id)
            .ok_or_else(|| anyhow!("account `{}` not found", account_id))?;
        *maybe_locked = maybe_locked.unlock(pw)?.into();

        Ok(())
    }

    pub fn list(&self) -> impl Iterator<Item = (&String, &MaybeLocked)> {
        self.accounts.iter()
    }
}
