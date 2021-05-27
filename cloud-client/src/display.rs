use prost::Message;

use cita_cloud_proto::blockchain::CompactBlock;
use cita_cloud_proto::evm::Receipt;

use crate::crypto::hash_data;

pub trait Display {
    fn display(&self) -> String;
}

impl<T: Display> Display for &T {
    fn display(&self) -> String {
        (**self).display()
    }
}

impl Display for CompactBlock {
    fn display(&self) -> String {
        let tx_count = self.body.as_ref().map(|b| b.tx_hashes.len()).unwrap_or(0);
        match &self.header {
            Some(header) => {
                let header_hash = {
                    let mut buf = Vec::with_capacity(header.encoded_len());
                    header.encode(&mut buf).unwrap();
                    hash_data(&buf)
                };
                format!(
                    "hash: 0x{}\nprev_hash: 0x{}\nheight: {}\ntx_count: {}\ntimestamp: {}\ntransaction_root: 0x{}\nproposer: 0x{}\n",
                    hex::encode(header_hash),
                    hex::encode(&header.prevhash),
                    header.height,
                    tx_count,
                    display_time(header.timestamp),
                    hex::encode(&header.transactions_root),
                    hex::encode(&header.proposer),
                )
            }
            None => panic!("no block header"),
        }
    }
}

// impl Display for RawTransaction {
//     fn display(&self) -> String {
//         match self {
//             RawTransaction::NormalTx(tx) => {

//             }
//             RawTransaction::Utxo
//         }

//     }
// }

#[cfg(feature = "evm")]
impl Display for Receipt {
    fn display(&self) -> String {
        format!(
            "tx_hash: 0x{}\nblock_hash: {}\nblock_number: {}\ntx_index: {}\nstate_root: {}\ncontract_addr: 0x{}\nerror_msg: `{}`",
            hex::encode(&self.transaction_hash),
            hex::encode(&self.block_hash),
            self.block_number,
            self.transaction_index,
            hex::encode(&self.state_root),
            hex::encode(&self.contract_address),
            self.error_message,
        )
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
