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

use rand::RngCore;

pub const HASH_BYTES_LEN: usize = 32;

fn sm3_hash(input: &[u8]) -> [u8; HASH_BYTES_LEN] {
    let mut result = [0u8; HASH_BYTES_LEN];
    result.copy_from_slice(libsm::sm3::hash::Sm3Hash::new(input).get_hash().as_ref());
    result
}

const SM2_PUBKEY_BYTES_LEN: usize = 64;
const SM2_PRIVKEY_BYTES_LEN: usize = 32;
pub const SM2_SIGNATURE_BYTES_LEN: usize = 128;

fn sm2_gen_keypair() -> ([u8; SM2_PUBKEY_BYTES_LEN], [u8; SM2_PRIVKEY_BYTES_LEN]) {
    let mut private_key = [0; SM2_PRIVKEY_BYTES_LEN];
    let mut public_key = [0u8; SM2_PUBKEY_BYTES_LEN];

    rand::thread_rng().fill_bytes(&mut private_key);
    let key_pair = efficient_sm2::KeyPair::new(&private_key).unwrap();
    let pubkey = key_pair.public_key();
    public_key.copy_from_slice(&pubkey.bytes_less_safe()[1..]);

    (public_key, private_key)
}

fn sm2_sign(pubkey: &[u8], privkey: &[u8], msg: &[u8]) -> [u8; SM2_SIGNATURE_BYTES_LEN] {
    let key_pair = efficient_sm2::KeyPair::new(privkey).unwrap();
    let sig = key_pair.sign(msg).unwrap();

    let mut sig_bytes = [0u8; SM2_SIGNATURE_BYTES_LEN];
    sig_bytes[..32].copy_from_slice(&sig.r());
    sig_bytes[32..64].copy_from_slice(&sig.s());
    sig_bytes[64..].copy_from_slice(pubkey);
    sig_bytes
}

pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
    let (pk, sk) = sm2_gen_keypair();
    (pk.to_vec(), sk.to_vec())
}

pub fn hash_data(data: &[u8]) -> Vec<u8> {
    sm3_hash(data).to_vec()
}

pub const ADDR_BYTES_LEN: usize = 20;

pub fn pk2address(pk: &[u8]) -> Vec<u8> {
    hash_data(pk)[HASH_BYTES_LEN - ADDR_BYTES_LEN..].to_vec()
}

pub fn sign_message(pubkey: &[u8], privkey: &[u8], msg: &[u8]) -> Option<Vec<u8>> {
    if msg.len() != HASH_BYTES_LEN {
        None
    } else {
        Some(sm2_sign(pubkey, privkey, msg).to_vec())
    }
}
