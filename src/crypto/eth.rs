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


/// Please refer to [kms_eth](https://github.com/cita-cloud/kms_eth).
/// This crypto impl must be compatible with `kms_eth` to work with it.

use tiny_keccak::{Hasher, Keccak};

use secp256k1::rand::rngs::OsRng;
use secp256k1::SecretKey as RawSecretKey;
use secp256k1::PublicKey as RawPublicKey;
use secp256k1::Secp256k1;
use secp256k1::Message;

use ctr::cipher::{NewCipher, StreamCipher};

use super::Crypto;


pub const HASH_BYTES_LEN: usize = 32;
pub type Hash = [u8; HASH_BYTES_LEN];

pub const ADDR_BYTES_LEN: usize = 20;
pub type Address = [u8; ADDR_BYTES_LEN];

pub const PUBLIC_KEY_BYTES_LEN: usize = 64;
pub type PublicKey = [u8; PUBLIC_KEY_BYTES_LEN];

pub const SECRET_KEY_BYTES_LEN: usize = 32;
pub type SecretKey = [u8; SECRET_KEY_BYTES_LEN];

pub const SIGNATURE_BYTES_LEN: usize = 65;
pub type Signature= [u8; SIGNATURE_BYTES_LEN];

lazy_static::lazy_static! {
    pub static ref SECP256K1: Secp256k1<secp256k1::All> = Secp256k1::new();
}


fn keccak_hash(input: &[u8]) -> Hash {
    let mut hasher = Keccak::v256();
    hasher.update(input);

    let mut output = [0u8; HASH_BYTES_LEN];
    hasher.finalize(&mut output);

    output
}

fn aes(data: &[u8], pw: &[u8]) -> Vec<u8> {
    type Aes128Ctr = ctr::Ctr128BE<aes::Aes128>;

    let mut output = data.to_vec();

    let pw_hash = keccak_hash(pw);
    let (key, nonce) = pw_hash.split_at(16);

    let mut cipher = Aes128Ctr::new(key.into(), nonce.into());
    cipher.apply_keystream(&mut output);

    output
}

fn secp256k1_generate_secret_key() -> SecretKey {
    let mut rng = OsRng::new().expect("failed to get OsRng");
    let raw_sk = RawSecretKey::new(&mut rng);
    raw_sk.serialize_secret()
}

fn secp256k1_sk2pk(sk: &SecretKey) -> PublicKey {
    let raw_sk = RawSecretKey::from_slice(sk).unwrap();
    let raw_pk = RawPublicKey::from_secret_key(&SECP256K1, &raw_sk);
    raw_pk.serialize_uncompressed()[1..65].try_into().unwrap()
}

fn secp256k1_pk2addr(pk: &PublicKey) -> Address {
    keccak_hash(pk)[HASH_BYTES_LEN - ADDR_BYTES_LEN..].try_into().unwrap()
}

fn secp256k1_sign(msg: &[u8], sk: &SecretKey) -> Signature {
    let hashed_msg = keccak_hash(msg);
    let raw_sk = RawSecretKey::from_slice(sk).unwrap();
    let raw_sig = SECP256K1.sign_ecdsa_recoverable(&Message::from_slice(&hashed_msg).unwrap(), &raw_sk);
    let (recovery_id, sig) = raw_sig.serialize_compact();

    // [<sig><recovery_id>]
    let mut output = [0u8; SIGNATURE_BYTES_LEN];
    output[..SIGNATURE_BYTES_LEN - 1].copy_from_slice(&sig);
    output[SIGNATURE_BYTES_LEN - 1] = recovery_id.to_i32() as u8;
    
    output
}


#[derive(Debug)]
pub struct EthCrypto;


impl Crypto for EthCrypto {
    type Hash = Hash;
    type Address = Address;
    type PublicKey = PublicKey;
    type SecretKey = SecretKey;
    type Signature = Signature;

    fn hash(msg: &[u8]) -> Self::Hash {
        keccak_hash(msg)
    }

    fn encrypt(plaintext: &[u8], pw: &[u8]) -> Vec<u8> {
        aes(plaintext, pw)
    }

    fn decrypt(ciphertext: &[u8], pw: &[u8]) -> Option<Vec<u8>> {
        Some(aes(ciphertext, pw))
    }

    fn generate_secret_key() -> Self::SecretKey {
        secp256k1_generate_secret_key()
    }

    fn sign(msg: &[u8], sk: &Self::SecretKey) -> Self::Signature {
        secp256k1_sign(msg, sk)
    }

    fn pk2addr(pk: &Self::PublicKey) -> Self::Address {
        secp256k1_pk2addr(pk)
    }

    fn sk2pk(sk: &Self::SecretKey) -> Self::PublicKey {
        secp256k1_sk2pk(sk)
    }
}

