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

use crate::core::cross_chain::CrossChainResultCode;
use crate::{
    core::controller::{CompactBlockWithStaterootProof, ProofType, ProofWithValidators},
    crypto::{Address, Hash},
    utils::{display_time, hex},
};
use cita_cloud_proto::blockchain::{BlockHeader, CompactBlock, CompactBlockBody};
use cita_cloud_proto::common::ProposalInner;
use cita_cloud_proto::controller::CrossChainProof;
use cita_cloud_proto::evm::{ReceiptProof, RootsInfo};
use cita_cloud_proto::{
    blockchain::{
        raw_transaction::Tx, Block, RawTransaction, Transaction, UnverifiedTransaction,
        UnverifiedUtxoTransaction, UtxoTransaction, Witness,
    },
    common::{NodeNetInfo, NodeStatus, PeerStatus},
    controller::SystemConfig,
    evm::{Balance, ByteAbi, ByteCode, ByteQuota, Log, Nonce, Receipt},
    executor::CallResponse,
};
use ethabi::ethereum_types::U256;
use serde_json::json;
use serde_json::map::Map;
use serde_json::Value as Json;
use tentacle_multiaddr::{Multiaddr, Protocol};

pub trait Display {
    fn to_json(&self) -> Json;
    fn display(&self) -> String {
        serde_json::to_string_pretty(&self.to_json()).unwrap()
    }
}

impl Display for Json {
    fn to_json(&self) -> Json {
        self.clone()
    }

    fn display(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }
}

impl<T: Display> Display for &T {
    fn to_json(&self) -> Json {
        (**self).to_json()
    }
}

impl Display for Address {
    fn to_json(&self) -> Json {
        json!(hex(self.as_slice()))
    }

    fn display(&self) -> String {
        hex(self.as_slice())
    }
}

impl Display for Hash {
    fn to_json(&self) -> Json {
        json!(hex(self.as_slice()))
    }

    fn display(&self) -> String {
        hex(self.as_slice())
    }
}

impl Display for CallResponse {
    fn to_json(&self) -> Json {
        json!(hex(&self.value))
    }

    // don't display ""
    fn display(&self) -> String {
        hex(&self.value)
    }
}

impl Display for CompactBlockWithStaterootProof {
    fn to_json(&self) -> Json {
        let tx_hashes = match self.compact_block.body.as_ref() {
            Some(body) => body.tx_hashes.iter().map(|h| hex(h)).collect(),
            None => Vec::new(),
        };

        match &self.compact_block.header {
            Some(header) => {
                json!({
                    "version": self.compact_block.version,
                    "height": header.height,
                    "prev_hash": hex(&header.prevhash),
                    "tx_count": tx_hashes.len(),
                    "tx_hashes": tx_hashes,
                    "timestamp": header.timestamp,
                    "time": display_time(header.timestamp),
                    "transaction_root": hex(&header.transactions_root),
                    "proposer": hex(&header.proposer),
                    "proof": hex(&self.proof.proof),
                    "state_root": hex(&self.state_root.state_root),
                })
            }
            None => json!({}),
        }
    }
}

impl Display for BlockHeader {
    fn to_json(&self) -> Json {
        json!({
            "height": self.height,
            "prev_hash": hex(&self.prevhash),
            "timestamp": self.timestamp,
            "time": display_time(self.timestamp),
            "transaction_root": hex(&self.transactions_root),
            "proposer": hex(&self.proposer),
        })
    }
}

impl Display for CompactBlockBody {
    fn to_json(&self) -> Json {
        let tx_hashes: Vec<String> = self.tx_hashes.iter().map(|h| hex(h)).collect();
        json!({
            "tx_count": tx_hashes.len(),
            "tx_hashes": tx_hashes,
        })
    }
}

impl Display for CompactBlock {
    fn to_json(&self) -> Json {
        json!({
            "version": self.version,
            "header": self.header.as_ref().map(|header| header.to_json()).unwrap_or_else(|| json!({})),
            "body": self.body.as_ref().map(|body| body.to_json()).unwrap_or_else(|| json!({})),
        })
    }
}

impl Display for Block {
    fn to_json(&self) -> Json {
        let raw_transactions = match self.body.as_ref() {
            Some(body) => body.body.iter().map(|t| t.to_json()).collect(),
            None => Vec::new(),
        };

        match &self.header {
            Some(header) => {
                json!({
                    "version": self.version,
                    "height": header.height,
                    "prev_hash": hex(&header.prevhash),
                    "tx_count": raw_transactions.len(),
                    "raw_transactions": raw_transactions,
                    "timestamp": header.timestamp,
                    "time": display_time(header.timestamp),
                    "transaction_root": hex(&header.transactions_root),
                    "proposer": hex(&header.proposer),
                    "proof": hex(&self.proof),
                    "state_root": hex(&self.state_root),
                })
            }
            None => json!({}),
        }
    }
}

impl Display for Transaction {
    fn to_json(&self) -> Json {
        json!({
            "version": self.version,
            "to": hex(&self.to),
            "nonce": self.nonce,
            "quota": self.quota,
            "valid_until_block": self.valid_until_block,
            "data": hex(&self.data),
            "value": hex(&self.value),
            "chain_id": hex(&self.chain_id),
        })
    }
}

impl Display for UnverifiedTransaction {
    fn to_json(&self) -> Json {
        json!({
            "transaction": self.transaction.as_ref().map(|tx| tx.to_json()).unwrap_or_else(|| json!({})),
            "transaction_hash": hex(&self.transaction_hash),
            "witness": self.witness.as_ref().map(|tx| tx.to_json()).unwrap_or_else(|| json!({})),
        })
    }
}

impl Display for SystemConfig {
    fn to_json(&self) -> Json {
        let validators = self.validators.iter().map(|v| hex(v)).collect::<Vec<_>>();
        json!({
            "version": self.version,
            "chain_id": hex(&self.chain_id),
            "admin": hex(&self.admin),
            "block_interval": self.block_interval,
            "block_limit": self.block_limit,
            "quota_limit": self.quota_limit,
            "validators": validators,
            "emergency_brake": self.emergency_brake,
            "version_pre_hash": hex(&self.version_pre_hash),
            "chain_id_pre_hash": hex(&self.chain_id_pre_hash),
            "admin_pre_hash": hex(&self.admin_pre_hash),
            "block_interval_pre_hash": hex(&self.block_interval_pre_hash),
            "validators_pre_hash": hex(&self.validators_pre_hash),
            "emergency_brake_pre_hash": hex(&self.emergency_brake_pre_hash),
            "block_limit_pre_hash": hex(&self.block_limit_pre_hash),
            "quota_limit_pre_hash": hex(&self.quota_limit_pre_hash),
        })
    }
}

impl Display for UtxoTransaction {
    fn to_json(&self) -> Json {
        json!({
            "version": self.version,
            "pre_tx_hash": hex(&self.pre_tx_hash),
            "output": hex(&self.output),
            "lock_id": self.lock_id,
        })
    }
}

impl Display for UnverifiedUtxoTransaction {
    fn to_json(&self) -> Json {
        let witnesses = self
            .witnesses
            .iter()
            .map(|w| w.to_json())
            .collect::<Vec<_>>();
        json!({
            "transaction": self.transaction.as_ref().map(|tx| tx.to_json()).unwrap_or_else(|| json!({})),
            "transaction_hash": hex(&self.transaction_hash),
            "witnesses": witnesses,
        })
    }
}

impl Display for Witness {
    fn to_json(&self) -> Json {
        json!({
            "signature": hex(&self.signature),
            "sender": hex(&self.sender),
        })
    }
}

impl Display for RawTransaction {
    fn to_json(&self) -> Json {
        match &self.tx {
            Some(Tx::NormalTx(tx)) => {
                json!({
                    "type": "Normal",
                    "transaction": tx.to_json()
                })
            }
            Some(Tx::UtxoTx(utxo)) => {
                json!({
                    "type": "Utxo",
                    "transaction": utxo.to_json()
                })
            }
            None => json!({}),
        }
    }
}

impl Display for (RawTransaction, u64, u64) {
    fn to_json(&self) -> Json {
        match &self.0.tx {
            Some(Tx::NormalTx(tx)) => {
                json!({
                    "type": "Normal",
                    "height": self.1,
                    "index": self.2,
                    "transaction": tx.to_json()
                })
            }
            Some(Tx::UtxoTx(utxo)) => {
                json!({
                    "type": "Utxo",
                    "height": self.1,
                    "index": self.2,
                    "transaction": utxo.to_json()
                })
            }
            None => json!({}),
        }
    }
}

impl Display for Log {
    fn to_json(&self) -> Json {
        json!({
            "address": hex(&self.address),
            "topics": json!(self.topics.iter().map(|t| hex(t)).collect::<Vec<_>>()),
            "data": hex(&self.data),
            "legacy_cita_block_hash": hex(&self.block_hash),
            "block_number": self.block_number,
            "tx_hash": hex(&self.transaction_hash),
            "tx_index": self.transaction_index,
            "log_index": self.log_index,
            "tx_log_index": self.transaction_log_index,
        })
    }
}

impl Display for Receipt {
    fn to_json(&self) -> Json {
        let logs = self.logs.iter().map(Log::to_json).collect::<Vec<_>>();
        json!({
            "tx_hash": hex(&self.transaction_hash),
            "block_hash": hex(&self.block_hash),
            "block_number": self.block_number,
            "tx_index": self.transaction_index,
            "contract_addr": hex(&self.contract_address),
            "logs": logs,
            "cumulative_quota_used": hex(&self.cumulative_quota_used),
            "quota_used": hex(&self.quota_used),
            "state_root": hex(&self.state_root),
            "logs_bloom": hex(&self.logs_bloom),
            "error_msg": self.error_message,
        })
    }
}

impl Display for ByteCode {
    fn to_json(&self) -> Json {
        json!(hex(&self.byte_code))
    }

    fn display(&self) -> String {
        hex(&self.byte_code)
    }
}

impl Display for Balance {
    fn to_json(&self) -> Json {
        json!(hex(&self.value))
    }

    fn display(&self) -> String {
        hex(&self.value)
    }
}

impl Display for Nonce {
    fn to_json(&self) -> Json {
        json!(hex(&self.nonce))
    }

    fn display(&self) -> String {
        hex(&self.nonce)
    }
}

impl Display for ByteAbi {
    fn to_json(&self) -> Json {
        // ByteAbi and bytes_abi..
        json!(String::from_utf8_lossy(&self.bytes_abi).to_string())
    }

    fn display(&self) -> String {
        // Don't display ""
        String::from_utf8_lossy(&self.bytes_abi).to_string()
    }
}

impl Display for ProofWithValidators {
    fn to_json(&self) -> Json {
        match &self.proof {
            ProofType::OverlordProof(overlord_proof) => {
                let mut validators = self.validators.iter().map(|v| hex(v)).collect::<Vec<_>>();
                let mut address_bitmap = overlord_proof
                    .signature
                    .address_bitmap
                    .iter()
                    .fold(String::new(), |r, &u| format!("{r}{u:b}"));
                if !validators.is_empty() {
                    address_bitmap = address_bitmap.split_at(validators.len()).0.to_string();
                    address_bitmap.chars().enumerate().for_each(|(i, c)| {
                        if c == '1' {
                            validators[i].insert(0, '*');
                        }
                    })
                };
                json!({
                    "height": overlord_proof.height,
                    "round": overlord_proof.round,
                    "proposal_hash": hex(&overlord_proof.block_hash),
                    "signature": hex(&overlord_proof.signature.signature),
                    "address_bitmap": address_bitmap,
                    "validators": validators,
                })
            }
        }
    }
}

impl Display for ByteQuota {
    fn to_json(&self) -> Json {
        json!(hex(self.bytes_quota.as_slice()))
    }

    fn display(&self) -> String {
        U256::from_big_endian(self.bytes_quota.as_slice())
            .as_u64()
            .to_string()
    }
}

impl Display for NodeNetInfo {
    fn to_json(&self) -> Json {
        let mut info_pair = Map::new();
        info_pair.insert(
            String::from("origin"),
            Json::from(hex(&self.origin.to_be_bytes())),
        );
        if let Ok(multi_address) = self.multi_address.parse::<Multiaddr>() {
            for protocol in multi_address.iter() {
                match protocol {
                    Protocol::Dns4(host) => {
                        info_pair.insert(String::from("host"), Json::from(host));
                    }
                    Protocol::Ip4(host) => {
                        info_pair.insert(String::from("host"), Json::from(host.to_string()));
                    }
                    Protocol::Tcp(port) => {
                        info_pair.insert(String::from("port"), Json::from(port));
                    }
                    Protocol::Tls(domain) => {
                        info_pair.insert(String::from("domain"), Json::from(domain));
                    }
                    p => panic!(
                        "multi address({:?}) contains unexpected protocol: {:?}",
                        self.multi_address, p
                    ),
                };
            }
        } else {
            info_pair.insert(
                String::from("multi_address"),
                Json::from(self.multi_address.as_str()),
            );
        }
        Json::from(info_pair)
    }
}

impl Display for PeerStatus {
    fn to_json(&self) -> Json {
        let mut info_pair = Map::new();
        info_pair.insert(String::from("address"), Json::from(hex(&self.address)));
        info_pair.insert(String::from("height"), Json::from(self.height));
        if let Some(node_net_info) = self.node_net_info.as_ref() {
            info_pair.insert(String::from("net_info"), node_net_info.to_json());
        };
        Json::from(info_pair)
    }
}

impl Display for NodeStatus {
    fn to_json(&self) -> Json {
        json!({
            "is_sync": self.is_sync,
            "version": self.version,
            "self_status": self.self_status.as_ref().unwrap().to_json(),
            "peers_count": self.peers_count,
            "peers_status": self.peers_status.iter().map(PeerStatus::to_json).collect::<Vec<_>>(),
            "is_danger": self.is_danger,
            "init_block_number": self.init_block_number,
            "waiting_block": self.waiting_block,
        })
    }
}

impl Display for RootsInfo {
    fn to_json(&self) -> Json {
        json!({
            "height": self.height,
            "state_root": hex(&self.state_root),
            "receipt_root": hex(&self.receipt_root),
        })
    }
}

impl Display for ReceiptProof {
    fn to_json(&self) -> Json {
        let receipt: crate::types::receipt::Receipt = rlp::decode(&self.receipt).unwrap();
        json!({
            "receipt": receipt.to_json(),
            "receipt_proof": hex(&self.receipt_proof),
            "roots_info": self.roots_info.as_ref().unwrap().to_json(),
        })
    }
}

impl Display for ProposalInner {
    fn to_json(&self) -> Json {
        json!({
            "pre_state_root": hex(&self.pre_state_root),
            "proposal": self.proposal.as_ref().unwrap().to_json(),
        })
    }
}

impl Display for CrossChainProof {
    fn to_json(&self) -> Json {
        json!({
            "version": self.version,
            "chain_id": hex(&self.chain_id),
            "proposal": self.proposal.as_ref().unwrap().to_json(),
            "receipt_proof": self.receipt_proof.as_ref().unwrap().to_json(),
            "proof": hex(&self.proof),
            "state_root": hex(&self.state_root),
        })
    }
}

impl Display for crate::types::log::Log {
    fn to_json(&self) -> Json {
        json!({
            "address": hex(self.address.as_bytes()),
            "topics": json!(self.topics.iter().map(|t| hex(t.as_bytes())).collect::<Vec<_>>()),
            "data": hex(&self.data),
        })
    }
}

impl Display for crate::types::receipt::Receipt {
    fn to_json(&self) -> Json {
        let err_msg = self.error.map_or("".to_string(), |e| e.description());
        let logs = self
            .logs
            .iter()
            .map(crate::types::log::Log::to_json)
            .collect::<Vec<_>>();
        json!({
            "quota_used": self.quota_used.as_u64(),
            "log_bloom": hex(self.log_bloom.as_bytes()),
            "logs": logs,
            "error": err_msg,
            "account_nonce": self.account_nonce.as_u64(),
            "transaction_hash": hex(self.transaction_hash.as_bytes()),
        })
    }
}

impl Display for CrossChainResultCode {
    fn to_json(&self) -> Json {
        json!({
            "code": self.code(),
            "message": format!("{self}")
        })
    }
}
