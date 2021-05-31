use prost::Message;

use cita_cloud_proto::blockchain::Transaction;
use cita_cloud_proto::blockchain::{CompactBlock, UnverifiedTransaction, Witness};
use cita_cloud_proto::controller::{RawTransaction, SystemConfig};
use cita_cloud_proto::evm::Log;
use cita_cloud_proto::evm::Receipt;

use serde_json::json;
use serde_json::Value as Json;

use crate::crypto::hash_data;

pub trait Display {
    fn to_json(&self) -> Json;
    fn display(&self) -> String {
        serde_json::to_string_pretty(&self.to_json()).unwrap()
    }
}

impl<T: Display> Display for &T {
    fn to_json(&self) -> Json {
        (**self).to_json()
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
                let header_hash = {
                    let mut buf = Vec::with_capacity(header.encoded_len());
                    header.encode(&mut buf).unwrap();
                    hash_data(&buf)
                };
                json!({
                    "version": self.version,
                    "hash": hex(&header_hash),
                    "prev_hash": hex(&header.prevhash),
                    "height": header.height,
                    "tx_count": tx_hashes.len(),
                    "tx_hashes": tx_hashes,
                    "timestamp": display_time(header.timestamp),
                    "transaction_root": hex(&header.transactions_root),
                    "proposer": hex(&header.proposer),
                })
            }
            None => json!("no block header"),
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
            "transaction": self.transaction.as_ref().map(|tx| tx.to_json()).unwrap_or_else(|| json!("None")),
            "transaction_hash": hex(&self.transaction_hash),
            "witness": self.witness.as_ref().map(|tx| tx.to_json()).unwrap_or_else(|| json!("None")),
        })
    }
}

impl Display for SystemConfig {
    fn to_json(&self) -> Json {
        let validators = self.validators.iter().map(|v| hex(&v)).collect::<Vec<_>>();
        json!({
            "version": self.version,
            "chain_id": hex(&self.chain_id),
            "admin": hex(&self.admin),
            "block_interval": self.block_interval,
            "validators": validators,
        })
    }
}

// impl Display for UtxoTransaction {
//     fn to_json(&self) -> Json {
//         json!({
//             "transaction": self.transaction.map(|tx| tx.to_json()).unwrap_or(json!("None")),
//             "transaction_hash": hex(&self.transaction_hash),
//             "witness": self.witness.map(|tx| tx.to_json()).unwrap_or(json!("None")),
//         })
//     }
// }

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
        use cita_cloud_proto::controller::raw_transaction::Tx;

        let inner = match &self.tx {
            Some(inner) => inner,
            None => return json!({}),
        };

        match inner {
            Tx::NormalTx(tx) => {
                json!({
                    "type": "Normal",
                    "transaction": tx.to_json()
                })
            }
            Tx::UtxoTx(_utxo) => {
                json!({
                    "type": "Utxo",
                    // "transaction": utxo.to_json()
                    "transaction": "unimplemented" // TODO
                })
            }
        }
    }
}

#[cfg(feature = "evm")]
impl Display for Log {
    fn to_json(&self) -> Json {
        json!({
            "address": hex(&self.address),
            "topics": json!(self.topics.iter().map(|t| hex(t)).collect::<Vec<_>>()),
            "data": hex(&self.data),
            "block_hash": hex(&self.block_hash),
            "block_number": self.block_number,
            "tx_hash": hex(&self.transaction_hash),
            "tx_index": self.transaction_index,
            "log_index": self.log_index,
            "tx_log_index": self.transaction_log_index,
        })
    }
}

#[cfg(feature = "evm")]
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

fn display_time(timestamp: u64) -> String {
    use chrono::offset::Local;
    use chrono::offset::TimeZone;
    use chrono::Utc;

    format!(
        "{}",
        Utc.timestamp_millis(timestamp as i64).with_timezone(&Local)
    )
}

fn hex(data: &[u8]) -> String {
    format!("0x{}", hex::encode(data))
}
