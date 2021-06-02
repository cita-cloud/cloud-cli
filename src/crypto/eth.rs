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

use ctr::cipher::{generic_array::GenericArray, NewCipher, StreamCipher};

type Aes128Ctr = ctr::Ctr128BE<aes::Aes128>;

fn aes(password_hash: &[u8], data: Vec<u8>) -> Vec<u8> {
    let mut data = data;

    let key = password_hash[0..16].to_owned();
    let nonce = password_hash[16..32].to_owned();

    let key = GenericArray::from_slice(&key);
    let nonce = GenericArray::from_slice(&nonce);

    let mut cipher = Aes128Ctr::new(key, nonce);

    cipher.apply_keystream(&mut data);

    data
}

pub fn encrypt(password_hash: &[u8], data: Vec<u8>) -> Vec<u8> {
    aes(password_hash, data)
}

pub fn decrypt(password_hash: &[u8], data: Vec<u8>) -> Vec<u8> {
    aes(password_hash, data)
}

use tiny_keccak::{Hasher, Keccak};

pub const HASH_BYTES_LEN: usize = 32;

fn keccak_hash(input: &[u8]) -> [u8; HASH_BYTES_LEN] {
    let mut result = [0u8; HASH_BYTES_LEN];

    let mut keccak = Keccak::v256();
    keccak.update(input);
    keccak.finalize(&mut result);
    result
}

const SECP256K1_PUBKEY_BYTES_LEN: usize = 64;
const SECP256K1_PRIVKEY_BYTES_LEN: usize = 32;
pub const SECP256K1_SIGNATURE_BYTES_LEN: usize = 65;

lazy_static::lazy_static! {
    pub static ref SECP256K1: secp256k1::Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
}

fn secp256k1_gen_keypair() -> (
    [u8; SECP256K1_PUBKEY_BYTES_LEN],
    [u8; SECP256K1_PRIVKEY_BYTES_LEN],
) {
    let context = &SECP256K1;
    let (seckey, pubkey) = context.generate_keypair(&mut rand::thread_rng());

    let serialized = pubkey.serialize_uncompressed();
    let mut pubkey = [0u8; SECP256K1_PUBKEY_BYTES_LEN];
    pubkey.copy_from_slice(&serialized[1..65]);

    let mut privkey = [0u8; SECP256K1_PRIVKEY_BYTES_LEN];
    privkey.copy_from_slice(&seckey[0..32]);

    (pubkey, privkey)
}

fn secp256k1_sign(privkey: &[u8], msg: &[u8]) -> [u8; SECP256K1_SIGNATURE_BYTES_LEN] {
    let context = &SECP256K1;
    // no way to create from raw byte array.
    let sec = secp256k1::SecretKey::from_slice(privkey).unwrap();
    let s = context.sign_recoverable(&secp256k1::Message::from_slice(msg).unwrap(), &sec);
    let (rec_id, data) = s.serialize_compact();
    let mut data_arr = [0; SECP256K1_SIGNATURE_BYTES_LEN];

    // no need to check if s is low, it always is
    data_arr[0..SECP256K1_SIGNATURE_BYTES_LEN - 1]
        .copy_from_slice(&data[0..SECP256K1_SIGNATURE_BYTES_LEN - 1]);
    data_arr[SECP256K1_SIGNATURE_BYTES_LEN - 1] = rec_id.to_i32() as u8;
    data_arr
}

fn secp256k1_recover(signature: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    let context = &SECP256K1;
    if let Ok(rid) = secp256k1::recovery::RecoveryId::from_i32(i32::from(
        signature[SECP256K1_SIGNATURE_BYTES_LEN - 1],
    )) {
        if let Ok(rsig) = secp256k1::recovery::RecoverableSignature::from_compact(
            &signature[0..SECP256K1_SIGNATURE_BYTES_LEN - 1],
            rid,
        ) {
            if let Ok(publ) =
                context.recover(&secp256k1::Message::from_slice(message).unwrap(), &rsig)
            {
                let serialized = publ.serialize_uncompressed();
                return Some(serialized[1..65].to_vec());
            }
        }
    }
    None
}

pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
    let (pk, sk) = secp256k1_gen_keypair();
    (pk.to_vec(), sk.to_vec())
}

pub fn hash_data(data: &[u8]) -> Vec<u8> {
    keccak_hash(data).to_vec()
}

pub fn verify_data_hash(data: Vec<u8>, hash: Vec<u8>) -> bool {
    if hash.len() != HASH_BYTES_LEN {
        false
    } else {
        hash == hash_data(&data)
    }
}

pub const ADDR_BYTES_LEN: usize = 20;

pub fn pk2address(pk: &[u8]) -> Vec<u8> {
    hash_data(pk)[HASH_BYTES_LEN - ADDR_BYTES_LEN..].to_vec()
}

pub fn sign_message(_pubkey: Vec<u8>, privkey: Vec<u8>, msg: Vec<u8>) -> Option<Vec<u8>> {
    if msg.len() != HASH_BYTES_LEN {
        None
    } else {
        Some(secp256k1_sign(&privkey, &msg).to_vec())
    }
}

pub fn recover_signature(msg: Vec<u8>, signature: Vec<u8>) -> Option<Vec<u8>> {
    if signature.len() != SECP256K1_SIGNATURE_BYTES_LEN || msg.len() != HASH_BYTES_LEN {
        None
    } else {
        secp256k1_recover(&signature, &msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_test() {
        let password = "password";
        let data = vec![1u8, 2, 3, 4, 5, 6, 7];

        let cipher_message = aes(password, data.clone());
        let decrypted_message = aes(password, cipher_message);
        assert_eq!(data, decrypted_message);
    }

    #[test]
    fn keccak_test() {
        let hash_empty: [u8; HASH_BYTES_LEN] = [
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7,
            0x03, 0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04,
            0x5d, 0x85, 0xa4, 0x70,
        ];
        assert_eq!(keccak_hash(&[]), hash_empty);
    }

    #[test]
    fn test_data_hash() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7];
        let hash = hash_data(&data);
        assert!(verify_data_hash(data.clone(), hash));
    }

    #[test]
    fn test_signature() {
        // message must be 32 bytes
        let data: [u8; HASH_BYTES_LEN] = [
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7,
            0x03, 0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04,
            0x5d, 0x85, 0xa4, 0x70,
        ];

        let (pubkey, privkey) = generate_keypair();
        let signature = sign_message(pubkey.clone(), privkey, data.to_vec()).unwrap();
        assert_eq!(recover_signature(data.to_vec(), signature), Some(pubkey));
    }
}