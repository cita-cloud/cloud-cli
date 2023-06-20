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

use crate::crypto::sm::sm3_hash;
use crate::crypto::ArrayLike;
use crate::types::clean_0x;
use bytes::Bytes;
use cita_cloud_proto::controller::{CrossChainProof, SystemConfig};
use ethabi::ethereum_types::H256;
use ophelia::{BlsSignatureVerify, HashValue};
use ophelia_blst::{BlsPublicKey, BlsSignature};
use overlord::extract_voters;
use overlord::types::{Node, Proof, Vote, VoteType};
use prost::Message;
use rlp::{Decodable, Rlp};
use std::fmt;

#[derive(Clone)]
pub enum CrossChainResultCode {
    Success,
    SuccessWithOutChainInfo,
    SuccessWithOutVotes,
    SuccessWithOutChainInfoAndVotes,
    SuccessNoneVotes,
    SuccessWithOutChainInfoNoneVotes,
    NoneProposal,
    NoneReceiptProof,
    NoneRootsInfo,
    StatRootCheckError,
    DecodeError(String),
    EncodeError(String),
    ReceiptProofCheckError,
    ProposalHeightOrHashCheckError,
    ConsensusInnerError(String),
    BlsInnerError(String),
    ThirdsInnerError(String),
    TransactionRootCheckError,
    NoneBlockBody,
    OutsideBlockBody,
    NoneBlockHeader,
    ChainIdVersionCheckError,
}

impl CrossChainResultCode {
    pub fn code(&self) -> u64 {
        match self {
            CrossChainResultCode::Success => 0,
            CrossChainResultCode::SuccessWithOutChainInfo => 1,
            CrossChainResultCode::SuccessWithOutVotes => 2,
            CrossChainResultCode::SuccessWithOutChainInfoAndVotes => 3,
            CrossChainResultCode::SuccessNoneVotes => 4,
            CrossChainResultCode::SuccessWithOutChainInfoNoneVotes => 5,
            CrossChainResultCode::NoneProposal => 100,
            CrossChainResultCode::NoneReceiptProof => 101,
            CrossChainResultCode::NoneRootsInfo => 102,
            CrossChainResultCode::StatRootCheckError => 103,
            CrossChainResultCode::DecodeError(_) => 104,
            CrossChainResultCode::EncodeError(_) => 105,
            CrossChainResultCode::ReceiptProofCheckError => 106,
            CrossChainResultCode::ProposalHeightOrHashCheckError => 107,
            CrossChainResultCode::ConsensusInnerError(_) => 108,
            CrossChainResultCode::BlsInnerError(_) => 109,
            CrossChainResultCode::ThirdsInnerError(_) => 110,
            CrossChainResultCode::TransactionRootCheckError => 111,
            CrossChainResultCode::NoneBlockBody => 112,
            CrossChainResultCode::OutsideBlockBody => 113,
            CrossChainResultCode::NoneBlockHeader => 114,
            CrossChainResultCode::ChainIdVersionCheckError => 115,
        }
    }
}

impl fmt::Display for CrossChainResultCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CrossChainResultCode::Success => write!(f, "Success"),
            CrossChainResultCode::SuccessWithOutChainInfo => {
                write!(f, "Success with out chain info")
            }
            CrossChainResultCode::NoneProposal => {
                write!(f, "None proposal field in cross chain proof")
            }
            CrossChainResultCode::NoneReceiptProof => {
                write!(f, "None receipt proof field in cross chain proof")
            }
            CrossChainResultCode::NoneRootsInfo => {
                write!(f, "None roots info field in receipt proof")
            }
            CrossChainResultCode::StatRootCheckError => write!(f, "Stat root check error"),
            CrossChainResultCode::DecodeError(s) => write!(f, "DecodeError: {s}"),
            CrossChainResultCode::EncodeError(s) => write!(f, "EncodeError: {s}"),
            CrossChainResultCode::ReceiptProofCheckError => write!(f, "Receipt proof check error"),
            CrossChainResultCode::ProposalHeightOrHashCheckError => {
                write!(f, "Proposal height or hash check error")
            }
            CrossChainResultCode::ConsensusInnerError(s) => write!(f, "ConsensusInnerError: {s}"),
            CrossChainResultCode::BlsInnerError(s) => write!(f, "BlsInnerError: {s}"),
            CrossChainResultCode::ThirdsInnerError(s) => write!(f, "ThirdsInnerError: {s}"),
            CrossChainResultCode::TransactionRootCheckError => {
                write!(f, "Transaction root check error")
            }
            CrossChainResultCode::NoneBlockBody => write!(f, "None block body"),
            CrossChainResultCode::OutsideBlockBody => write!(f, "Outside block body"),
            CrossChainResultCode::NoneBlockHeader => write!(f, "None block header"),
            CrossChainResultCode::ChainIdVersionCheckError => {
                write!(f, "Chain id or version check error")
            }
            CrossChainResultCode::SuccessWithOutVotes => write!(f, "Success with out votes"),
            CrossChainResultCode::SuccessWithOutChainInfoAndVotes => {
                write!(f, "Success with out chain info and votes")
            }
            CrossChainResultCode::SuccessNoneVotes => write!(f, "Success none votes"),
            CrossChainResultCode::SuccessWithOutChainInfoNoneVotes => {
                write!(f, "Success with out chain info none votes")
            }
        }
    }
}

impl fmt::Debug for CrossChainResultCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

pub fn verify_cross_chain_proof(
    ccp: CrossChainProof,
    validators: Option<Vec<String>>,
    sys_conf: Option<SystemConfig>,
) -> Result<(), CrossChainResultCode> {
    let proposal = ccp.proposal.ok_or(CrossChainResultCode::NoneProposal)?;
    let receipt_proof = ccp
        .receipt_proof
        .ok_or(CrossChainResultCode::NoneReceiptProof)?;
    let receipt = receipt_proof.receipt;

    let roots_info = receipt_proof
        .roots_info
        .ok_or(CrossChainResultCode::NoneRootsInfo)?;
    let app_hash: Vec<u8> = roots_info
        .state_root
        .iter()
        .zip(roots_info.receipt_root.iter())
        .map(|(x, y)| x ^ y)
        .collect();
    if ccp.state_root != app_hash {
        return Err(CrossChainResultCode::StatRootCheckError);
    }

    let receipt_proof: cita_merklehash::Proof = rlp::decode(&receipt_proof.receipt_proof)
        .map_err(|_| CrossChainResultCode::DecodeError("receipt proof".to_string()))?;
    let receipt_proof: static_merkle_tree::Proof<H256> = receipt_proof.into();

    if !receipt_proof.verify(
        &H256::from_slice(&roots_info.receipt_root),
        H256(sm3_hash(&receipt)),
        cita_merklehash::merge,
    ) {
        return Err(CrossChainResultCode::ReceiptProofCheckError);
    }

    let receipt: crate::types::receipt::Receipt = rlp::decode(&receipt).map_err(|_| {
        CrossChainResultCode::DecodeError("proposal in cross chain proof".to_string())
    })?;
    let compact_block = proposal
        .proposal
        .clone()
        .ok_or(CrossChainResultCode::NoneProposal)?;
    let tx_list = compact_block
        .body
        .ok_or(CrossChainResultCode::NoneBlockBody)?;
    if !tx_list
        .tx_hashes
        .contains(&receipt.transaction_hash.0.to_vec())
    {
        return Err(CrossChainResultCode::OutsideBlockBody);
    }
    let tx_list = tx_list.tx_hashes.iter().fold(
        Vec::with_capacity(tx_list.tx_hashes.len()),
        |mut acc, tx_hash| {
            acc.extend_from_slice(tx_hash);
            acc
        },
    );
    if sm3_hash(&tx_list).to_vec()
        != compact_block
            .header
            .ok_or(CrossChainResultCode::NoneBlockHeader)?
            .transactions_root
    {
        return Err(CrossChainResultCode::TransactionRootCheckError);
    }

    let mut check_chain_info = false;
    if let Some(sys_conf) = sys_conf {
        if sys_conf.chain_id != ccp.chain_id || sys_conf.version != ccp.version {
            return Err(CrossChainResultCode::ChainIdVersionCheckError);
        } else {
            check_chain_info = true;
        }
    }

    if ccp.proof.is_empty() {
        if check_chain_info {
            return Err(CrossChainResultCode::SuccessNoneVotes);
        } else {
            return Err(CrossChainResultCode::SuccessWithOutChainInfoNoneVotes);
        }
    }

    if let Some(validators) = validators {
        let mut ol_nodes = Vec::new();
        for v in validators {
            ol_nodes.push(Node {
                address: Bytes::copy_from_slice(&hex::decode(clean_0x(&v)).map_err(|_| {
                    CrossChainResultCode::DecodeError(format!("hex address: {v:?}"))
                })?),
                propose_weight: 1,
                vote_weight: 1,
            })
        }

        let mut proposal_bytes = Vec::with_capacity(proposal.encoded_len());
        proposal
            .encode(&mut proposal_bytes)
            .map_err(|_| CrossChainResultCode::EncodeError("proposal".to_string()))?;

        let proposal_hash = Bytes::from(sm3_hash(&proposal_bytes).to_vec());
        let proof = Proof::decode(&Rlp::new(&ccp.proof))
            .map_err(|_| CrossChainResultCode::DecodeError("decode proof failed".to_string()))?;

        if proof.block_hash != proposal_hash || proof.height != roots_info.height {
            return Err(CrossChainResultCode::ProposalHeightOrHashCheckError);
        }
        let signed_voters = extract_voters(&mut ol_nodes, &proof.signature.address_bitmap)
            .map_err(|e| {
                CrossChainResultCode::ConsensusInnerError(format!("extract_voters failed: {e:?}"))
            })?;

        let vote = Vote {
            height: proof.height,
            round: proof.round,
            vote_type: VoteType::Precommit,
            block_hash: Bytes::from(proof.block_hash.to_vec()),
        };
        let vote_bytes = rlp::encode(&vote);
        let vote_hash = Bytes::from(sm3_hash(vote_bytes.as_ref()).to_vec());
        let mut pub_keys = Vec::new();

        for voter in signed_voters {
            let pub_key = BlsPublicKey::try_from(voter.as_ref()).map_err(|_| {
                CrossChainResultCode::BlsInnerError(format!(
                    "can't parse {} to Bls public key",
                    hex::encode(voter.as_ref())
                ))
            })?;
            pub_keys.push(pub_key.clone());
        }

        let aggregate_key = BlsPublicKey::aggregate(pub_keys).map_err(|e| {
            CrossChainResultCode::BlsInnerError(format!("BlsPublicKey::aggregate failed: {e:?}"))
        })?;
        let aggregated_signature = BlsSignature::try_from(proof.signature.signature.as_ref())
            .map_err(|e| {
                CrossChainResultCode::BlsInnerError(format!("BlsSignature::try_from failed: {e:?}"))
            })?;
        let hash = HashValue::try_from(vote_hash.as_ref()).map_err(|e| {
            CrossChainResultCode::ThirdsInnerError(format!("try_from failed: {e:?}"))
        })?;

        aggregated_signature
            .verify(&hash, &aggregate_key, &"".to_string())
            .map_err(|e| {
                CrossChainResultCode::BlsInnerError(format!("Verify BlsSignature failed: {e:?}"))
            })?;
        if check_chain_info {
            Ok(())
        } else {
            Err(CrossChainResultCode::SuccessWithOutChainInfo)
        }
    } else if check_chain_info {
        Err(CrossChainResultCode::SuccessWithOutVotes)
    } else {
        Err(CrossChainResultCode::SuccessWithOutChainInfoAndVotes)
    }
}
