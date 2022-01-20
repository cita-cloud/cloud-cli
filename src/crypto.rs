mod sm;
use serde::Deserialize;
use serde::Serialize;
pub use sm::{generate_keypair, hash_data, pk2address, sign_message};

mod eth;
#[cfg(feature = "crypto_eth")]
pub use eth::{generate_keypair, hash_data, pk2address, sign_message};

use anyhow::Context;
use anyhow::Result;

pub use eth::EthCrypto;

// I tried this, but it's not easy to constrain the Error type of TryFrom
// since type bound on generic param's associated type is unstable.
//
// pub trait ArrayLike: AsRef<[u8]> + for<'a> TryFrom<&'a [u8]> { }

// TODO: better name + add Copy?
/// assert_eq!(ArrayLike::try_from_slice(arr.as_slice()), Ok(arr));
pub trait ArrayLike: PartialEq + Eq + Sized + Send + Sync + 'static {
    fn as_slice(&self) -> &[u8];
    fn try_from_slice(slice: &[u8]) -> Result<Self>;

    fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl<const N: usize> ArrayLike for [u8; N] {
    fn as_slice(&self) -> &[u8] {
        self.as_slice()
    }

    fn try_from_slice(slice: &[u8]) -> Result<Self> {
        slice.try_into().with_context(|| {
            format!(
                "length mismatched, expected: `{}`, got: `{}`",
                N,
                slice.len()
            )
        })
    }
}

pub trait Crypto: 'static {
    type Hash: ArrayLike;
    type Address: ArrayLike;

    type PublicKey: ArrayLike;
    type SecretKey: ArrayLike;

    type Signature: ArrayLike;

    fn hash(msg: &[u8]) -> Self::Hash;

    fn encrypt(plaintext: &[u8], pw: &[u8]) -> Vec<u8>;
    fn decrypt(ciphertext: &[u8], pw: &[u8]) -> Option<Vec<u8>>;

    fn generate_secret_key() -> Self::SecretKey;
    fn generate_keypair() -> (Self::PublicKey, Self::SecretKey) {
        let sk = Self::generate_secret_key();
        let pk = Self::sk2pk(&sk);
        (pk, sk)
    }

    fn sign(msg: &[u8], sk: &Self::SecretKey) -> Self::Signature;

    fn pk2addr(pk: &Self::PublicKey) -> Self::Address;
    fn sk2pk(sk: &Self::SecretKey) -> Self::PublicKey;

    fn sk2addr(sk: &Self::SecretKey) -> Self::Address {
        Self::pk2addr(&Self::sk2pk(sk))
    }
}
