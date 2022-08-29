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

pub mod eth;
pub mod sm;

use anyhow::Context;
use anyhow::Result;

pub use eth::EthCrypto;
pub use sm::SmCrypto;

// I tried this, but it's not easy to constrain the Error type of TryFrom
// since type bound on generic param's associated type is unstable.
//
// pub trait ArrayLike: AsRef<[u8]> + for<'a> TryFrom<&'a [u8]> { }

// TODO: better name + add Copy?
/// assert_eq!(ArrayLike::try_from_slice(arr.as_slice()), Ok(arr));
pub trait ArrayLike: Clone + PartialEq + Eq + Sized + Send + Sync + 'static {
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

impl ArrayLike for Vec<u8> {
    fn as_slice(&self) -> &[u8] {
        self.as_slice()
    }

    fn try_from_slice(slice: &[u8]) -> Result<Self> {
        Ok(slice.to_vec())
    }
}

pub const HASH_BYTES_LEN: usize = 32;
pub type Hash = [u8; HASH_BYTES_LEN];

pub const ADDR_BYTES_LEN: usize = 20;
pub type Address = [u8; ADDR_BYTES_LEN];

pub const BLS_ADDR_BYTES_LEN: usize = 48;

pub trait Crypto: Send + Sync + 'static {
    type PublicKey: ArrayLike;
    type SecretKey: ArrayLike;

    type Signature: ArrayLike;

    fn hash(msg: &[u8]) -> Hash;

    fn encrypt(plaintext: &[u8], pw: &[u8]) -> Vec<u8>;
    fn decrypt(ciphertext: &[u8], pw: &[u8]) -> Option<Vec<u8>>;

    fn generate_secret_key() -> Self::SecretKey;
    fn generate_keypair() -> (Self::PublicKey, Self::SecretKey) {
        let sk = Self::generate_secret_key();
        let pk = Self::sk2pk(&sk);
        (pk, sk)
    }

    fn sign(msg: &[u8], sk: &Self::SecretKey) -> Self::Signature;

    fn sk2pk(sk: &Self::SecretKey) -> Self::PublicKey;
    fn pk2addr(pk: &Self::PublicKey) -> Address;
    fn sk2addr(sk: &Self::SecretKey) -> Address {
        Self::pk2addr(&Self::sk2pk(sk))
    }
}
