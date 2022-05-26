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

// WARNING: efficient_sm2 and libsm didn't handle the potential security risk that
// privkey/secret leaks from un-zeroized memory.

/// Please refer to [kms_sm](https://github.com/cita-cloud/kms_sm).
/// This crypto impl must be compatible with `kms_sm` to work with it.
use super::Crypto;
use efficient_sm2::KeyPair;
use rand::Rng;

pub const SM3_HASH_BYTES_LEN: usize = 32;
pub type Hash = [u8; SM3_HASH_BYTES_LEN];

pub const ADDR_BYTES_LEN: usize = 20;
pub type Address = [u8; ADDR_BYTES_LEN];

pub const SM2_PUBKEY_BYTES_LEN: usize = 64;
pub type PublicKey = [u8; SM2_PUBKEY_BYTES_LEN];

pub const SM2_SECKEY_BYTES_LEN: usize = 32;
pub type SecretKey = [u8; SM2_SECKEY_BYTES_LEN];

pub const SM2_SIGNATURE_BYTES_LEN: usize = 128;
pub type Signature = [u8; SM2_SIGNATURE_BYTES_LEN];

pub fn sm3_hash(input: &[u8]) -> Hash {
    libsm::sm3::hash::Sm3Hash::new(input).get_hash()
}

pub fn sm4_encrypt(plaintext: &[u8], password: &[u8]) -> Vec<u8> {
    let pw_hash = sm3_hash(password);
    let (key, iv) = pw_hash.split_at(16);
    let cipher = libsm::sm4::Cipher::new(key, libsm::sm4::Mode::Cfb).unwrap();

    cipher.encrypt(plaintext, iv).unwrap()
}

pub fn sm4_decrypt(ciphertext: &[u8], password: &[u8]) -> Option<Vec<u8>> {
    let pw_hash = sm3_hash(password);
    let (key, iv) = pw_hash.split_at(16);
    let cipher = libsm::sm4::Cipher::new(key, libsm::sm4::Mode::Cfb).unwrap();

    cipher.decrypt(ciphertext, iv).ok()
}

pub fn sm2_generate_secret_key() -> SecretKey {
    rand::thread_rng().gen()
}

// FIXME: sk -> kp is an expensive operation, use keypair directly.
// (that will need a wrapper type and impl some traits)
pub fn sm2_sign(msg: &[u8], sk: &SecretKey) -> Signature {
    let keypair = efficient_sm2::KeyPair::new(sk).unwrap();
    let sig = keypair.sign(msg).expect("sm2 sign failed");

    let mut sig_bytes = [0u8; SM2_SIGNATURE_BYTES_LEN];
    sig_bytes[..32].copy_from_slice(&sig.r());
    sig_bytes[32..64].copy_from_slice(&sig.s());
    sig_bytes[64..].copy_from_slice(&keypair.public_key().bytes_less_safe()[1..]);
    sig_bytes
}

#[allow(unused)]
pub fn sm2_recover_signature(msg: &[u8], signature: &Signature) -> Option<PublicKey> {
    let r = &signature[0..32];
    let s = &signature[32..64];
    let pk = &signature[64..];

    let pubkey = efficient_sm2::PublicKey::new(&pk[..32], &pk[32..]);
    let sig = efficient_sm2::Signature::new(r, s).ok()?;

    sig.verify(&pubkey, msg).ok()?;

    Some(pk.try_into().unwrap())
}

pub fn kp2pk(keypair: &KeyPair) -> PublicKey {
    keypair.public_key().bytes_less_safe()[1..]
        .try_into()
        .unwrap()
}

pub fn sk2pk(sk: &SecretKey) -> PublicKey {
    let keypair = efficient_sm2::KeyPair::new(sk).unwrap();
    kp2pk(&keypair)
}

pub fn pk2addr(pk: &PublicKey) -> Address {
    let hash = sm3_hash(pk);
    hash[SM3_HASH_BYTES_LEN - ADDR_BYTES_LEN..]
        .try_into()
        .unwrap()
}

pub struct SmCrypto;

impl Crypto for SmCrypto {
    type PublicKey = PublicKey;
    type SecretKey = SecretKey;
    type Signature = Signature;

    fn hash(msg: &[u8]) -> Hash {
        sm3_hash(msg)
    }

    fn encrypt(plaintext: &[u8], pw: &[u8]) -> Vec<u8> {
        sm4_encrypt(plaintext, pw)
    }

    fn decrypt(ciphertext: &[u8], pw: &[u8]) -> Option<Vec<u8>> {
        sm4_decrypt(ciphertext, pw)
    }

    fn generate_secret_key() -> Self::SecretKey {
        sm2_generate_secret_key()
    }

    fn sign(msg: &[u8], sk: &Self::SecretKey) -> Self::Signature {
        sm2_sign(msg, sk)
    }

    fn pk2addr(pk: &Self::PublicKey) -> Address {
        pk2addr(pk)
    }

    fn sk2pk(sk: &Self::SecretKey) -> Self::PublicKey {
        sk2pk(sk)
    }
}
