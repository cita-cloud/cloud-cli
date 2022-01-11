#[cfg(feature = "crypto_sm")]
mod sm;
use serde::Serialize;
use serde::Deserialize;
#[cfg(feature = "crypto_sm")]
pub use sm::{generate_keypair, hash_data, pk2address, sign_message};

#[cfg(feature = "crypto_eth")]
mod eth;
#[cfg(feature = "crypto_eth")]
pub use eth::{generate_keypair, hash_data, pk2address, sign_message};

use anyhow::Result;
use anyhow::Context;

// I tried this, but it's not easy to constrain the Error type of TryFrom
// since type bound on associated type is unstable.
//
// pub trait ArrayLike: AsRef<[u8]> + for<'a> TryFrom<&'a [u8]> { }


// TODO: better name
pub trait ArrayLike {
    fn as_slice(&self) -> &[u8];
    fn try_from_slice(slice: &[u8]) -> Result<Self>;
}

impl<const N: usize> ArrayLike for [u8; N] {
    fn as_slice(&self) -> &[u8] {
        self.as_slice()
    }

    fn try_from_slice(slice: &[u8]) -> Result<Self> {
        slice.try_into().with_context(|| format!("length mismatched, expected: `{}`, got: `{}`", N, slice.len()))
    }
}


pub trait Crypto {
    type Hash: ArrayLike;
    type Address: ArrayLike;

    type PublicKey: ArrayLike;
    type SecretKey: ArrayLike;

    type Signature: ArrayLike;

    fn gen_keypair() -> (Self::PublicKey, Self::SecretKey);

    fn hash(msg: &[u8]) -> Self::Hash;
    fn sign(msg: &[u8], sk: &Self::SecretKey) -> Self::Signature;

    fn pk2addr(pk: &Self::PublicKey) -> Self::Address;
    fn sk2pk(sk: &Self::SecretKey) -> Self::PublicKey;

    fn sk2addr(sk: &Self::SecretKey) -> Self::Address {
        Self::pk2addr(&Self::sk2pk(sk))
    }
}
