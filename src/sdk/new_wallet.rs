use crate::crypto::{ArrayLike, Crypto};
use serde::{Deserialize, Serialize};

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
        struct HexVisitor<T>(PhantomData<fn(T)>);
        impl<'de, V: ArrayLike> Visitor<'de> for HexVisitor<V> {
            type Value = V;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a hex-encoded bytes with the crypto specific bit length")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> 
            {
                let err_fn = |e| de::Error::invalid_value(de::Unexpected::Str(v),&self);
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

    fn address(&self) -> &C::Address {
        &self.address
    }

    fn public_key(&self) -> &C::PublicKey {
        &self.public_key
    }

    fn expose_secret_key(&self) -> &C::SecretKey {
        &self.secret_key
    }

    fn sign(&self, msg: &[u8]) -> C::Signature {
        C::sign(msg, self.expose_secret_key())
    }
}


#[derive(Serialize, Deserialize)]
struct LockedAccount<C: Crypto> {
    #[serde(with = "hex_repr")]
    address: C::Address,
    #[serde(with = "hex_repr")]
    public_key: C::PublicKey,
    encrypted_sk: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum MaybeLockedAccount<C: Crypto> {
    Unlocked(Account<C>),
    Locked(LockedAccount<C>),
}
