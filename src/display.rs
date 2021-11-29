use serde_json::json;
use serde_json::Value as Json;

use crate::{
    proto::{
        blockchain::{
            raw_transaction::Tx, CompactBlock, RawTransaction, Transaction, UnverifiedTransaction,
            UnverifiedUtxoTransaction, UtxoTransaction, Witness,
        },
        controller::SystemConfig,
        evm::{Log, Receipt},
    },
    utils::{display_time, hex},
    wallet::Account,
};

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

impl Display for Account {
    fn to_json(&self) -> Json {
        json!({
            "account_addr": hex(&self.addr),
            "public_key": hex(&self.keypair.0),
            "private_key": hex(&self.keypair.1),
        })
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

#[cfg(feature = "evm")]
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

#[cfg(feature = "evm")]
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
