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

use serde_json::json;
use serde_json::map::Map;
use serde_json::Value as Json;
use tentacle_multiaddr::{Multiaddr, Protocol};

use crate::{
    proto::{
        blockchain::{
            raw_transaction::Tx, CompactBlock, RawTransaction, Transaction, UnverifiedTransaction,
            UnverifiedUtxoTransaction, UtxoTransaction, Witness,
        },
        common::{NodeInfo, TotalNodeInfo},
        controller::SystemConfig,
        evm::{Balance, ByteAbi, ByteCode, Log, Nonce, Receipt},
        executor::CallResponse,
    },
    utils::{display_time, hex},
};

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

impl Display for CallResponse {
    fn to_json(&self) -> Json {
        json!(hex(&self.value))
    }

    // don't display " "
    fn display(&self) -> String {
        hex(&self.value)
    }
}

impl Display for CompactBlock {
    fn to_json(&self) -> Json {
        let tx_hashes = match self.body.as_ref() {
            Some(body) => body.tx_hashes.iter().map(|h| hex(h)).collect(),
            None => vec![],
        };

        match &self.header {
            Some(header) => {
                json!({
                    "version": self.version,
                    "height": header.height,
                    "prev_hash": hex(&header.prevhash),
                    "tx_count": tx_hashes.len(),
                    "tx_hashes": tx_hashes,
                    "timestamp": display_time(header.timestamp),
                    "transaction_root": hex(&header.transactions_root),
                    "proposer": hex(&header.proposer),
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
            "validators": validators,
            "emergency_brake": self.emergency_brake,
            "version_pre_hash": hex(&self.version_pre_hash),
            "chain_id_pre_hash": hex(&self.chain_id_pre_hash),
            "admin_pre_hash": hex(&self.admin_pre_hash),
            "block_interval_pre_hash": hex(&self.block_interval_pre_hash),
            "validators_pre_hash": hex(&self.validators_pre_hash),
            "emergency_brake_pre_hash": hex(&self.emergency_brake_pre_hash),
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

impl Display for NodeInfo {
    fn to_json(&self) -> Json {
        let mut info_pair = Map::new();
        info_pair.insert(
            String::from("address"),
            Json::from(format!("0x{}", hex::encode(&self.address))),
        );
        let net_info = self.net_info.as_ref().unwrap();
        info_pair.insert(String::from("origin"), Json::from(net_info.origin));
        let multi_address: Multiaddr = net_info.multi_address[..].parse().unwrap();
        for ptcl in multi_address.iter() {
            match ptcl {
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
                _ => panic!("multi address({:?}) can't parse", net_info.multi_address),
            };
        }
        Json::from(info_pair)
    }
}

impl Display for TotalNodeInfo {
    fn to_json(&self) -> Json {
        let nodes: Vec<Json> = self.nodes.iter().map(Display::to_json).collect();
        json!({ "nodes": nodes })
    }
}

impl Display for Log {
    fn to_json(&self) -> Json {
        json!({
            "address": hex(&self.address),
            "topics": json!(self.topics.iter().map(|t| hex(t)).collect::<Vec<_>>()),
            "data": hex(&self.data),
            // FIXME: the same as receipt
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
            // FIXME:
            // This is a legacy block_hash from cita.
            // It's not the same as chain's block hash.
            "legacy_cita_block_hash": hex(&self.block_hash),
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
}

impl Display for Balance {
    fn to_json(&self) -> Json {
        json!(hex(&self.value))
    }
}

impl Display for Nonce {
    fn to_json(&self) -> Json {
        json!(hex(&self.nonce))
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
