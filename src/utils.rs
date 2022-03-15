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

use std::io::Write;
use std::path::Path;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use crossbeam::atomic::AtomicCell;
use tempfile::NamedTempFile;
use time::UtcOffset;

use crate::crypto::{Address, ArrayLike, Crypto, Hash};

// Use an Option because UtcOffset::from_hms returns a Result
// that cannot be unwraped in constant expr...
static LOCAL_UTC_OFFSET: AtomicCell<Option<UtcOffset>> = AtomicCell::new(None);

pub fn parse_addr(s: &str) -> Result<Address> {
    let input = parse_data(s)?;
    Address::try_from_slice(&input)
}

pub fn parse_pk<C: Crypto>(s: &str) -> Result<C::PublicKey> {
    let input = parse_data(s)?;
    C::PublicKey::try_from_slice(&input)
}

pub fn parse_sk<C: Crypto>(s: &str) -> Result<C::SecretKey> {
    let input = parse_data(s)?;
    C::SecretKey::try_from_slice(&input)
}

pub fn parse_hash(s: &str) -> Result<Hash> {
    let input = parse_data(s)?;
    Hash::try_from_slice(&input)
}

// TODO: Should we do the padding?
pub fn parse_value(s: &str) -> Result<[u8; 32]> {
    let s = remove_0x(s);
    if s.len() > 64 {
        return Err(anyhow!("can't parse value, the given str is too long"));
    }
    // padding 0 to 32 bytes
    let padded = format!("{:0>64}", s);
    hex::decode(&padded)
        .map(|v| v.try_into().unwrap())
        .map_err(|e| anyhow!("invalid value: {e}"))
}

pub fn parse_data(s: &str) -> Result<Vec<u8>> {
    hex::decode(remove_0x(s)).context("invalid hex input")
}

pub fn hex(data: &[u8]) -> String {
    format!("0x{}", hex::encode(data))
}

/// This should be called without any other concurrent running threads.
pub fn init_local_utc_offset() {
    let local_utc_offset =
        UtcOffset::current_local_offset().unwrap_or_else(|_| UtcOffset::from_hms(8, 0, 0).unwrap());

    LOCAL_UTC_OFFSET.store(Some(local_utc_offset));
}

/// Call init_utc_offset first without any other concurrent running threads. Otherwise UTC+8 is used.
/// This is due to a potential race condition.
/// [CVE-2020-26235](https://github.com/chronotope/chrono/issues/602)
pub fn display_time(timestamp: u64) -> String {
    let local_offset = LOCAL_UTC_OFFSET
        .load()
        .unwrap_or_else(|| UtcOffset::from_hms(8, 0, 0).unwrap());
    let format = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour sign:mandatory]:[offset_minute]",
    )
    .unwrap();
    time::OffsetDateTime::from_unix_timestamp((timestamp / 1000) as i64)
        .expect("invalid timestamp")
        .to_offset(local_offset)
        .format(&format)
        .unwrap()
}

pub fn remove_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

// Safe in the sense of file integrity, not cryptography.
pub fn safe_save(path: impl AsRef<Path>, content: &[u8], override_existing: bool) -> Result<()> {
    let path = path.as_ref();
    let dir = path
        .parent()
        .ok_or_else(|| anyhow!("cannot load containing dir"))?;

    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(content)?;

    let mut f = if override_existing {
        tmp.persist(path)?
    } else {
        tmp.persist_noclobber(path)?
    };
    f.flush()?;

    Ok(())
}
