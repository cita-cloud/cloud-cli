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
use std::num::ParseIntError;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use crossbeam::atomic::AtomicCell;
use tempfile::NamedTempFile;
use time::UtcOffset;

use crate::{
    core::controller::ControllerBehaviour,
    crypto::{Address, ArrayLike, Crypto, Hash, ADDR_BYTES_LEN, BLS_ADDR_BYTES_LEN},
};

// Use an Option because UtcOffset::from_hms returns a Result
// that cannot be unwraped in constant expr...
static LOCAL_UTC_OFFSET: AtomicCell<Option<UtcOffset>> = AtomicCell::new(None);

pub fn parse_u64(s: &str) -> Result<u64, ParseIntError> {
    s.parse::<u64>()
}

pub fn parse_addr(s: &str) -> Result<Address> {
    let input = parse_data(s)?;
    Address::try_from_slice(&input)
}

pub fn parse_validator_addr(s: &str) -> Result<Vec<u8>> {
    let input = parse_data(s)?;
    if input.len() == BLS_ADDR_BYTES_LEN || input.len() == ADDR_BYTES_LEN {
        Ok(input)
    } else {
        bail!("Invalid length of validator address: {}", input.len());
    }
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

#[derive(Debug, Clone, Copy)]
pub enum Position {
    // v
    Absolute(u64),
    // current + v
    FromCurrent(u64),
    // current - v
    ToCurrent(u64),
}

impl Position {
    pub fn with_current(self, current: u64) -> u64 {
        match self {
            Self::Absolute(v) => v,
            Self::FromCurrent(v) => current.saturating_add(v),
            Self::ToCurrent(v) => current.saturating_sub(v),
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Position::FromCurrent(0)
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Absolute(a), Self::Absolute(b)) => a == b,
            (Self::FromCurrent(a), Self::FromCurrent(b)) => a == b,
            (Self::ToCurrent(a), Self::ToCurrent(b)) => a == b,
            (Self::FromCurrent(0), Self::ToCurrent(0)) => true,
            (Self::ToCurrent(0), Self::FromCurrent(0)) => true,
            _ => false,
        }
    }
}

impl Eq for Position {}

pub fn parse_position(s: &str) -> Result<Position> {
    let v = s.strip_prefix(['+', '-']).unwrap_or(s).parse::<u64>()?;
    let pos = if s.starts_with('+') {
        Position::FromCurrent(v)
    } else if s.starts_with('-') {
        Position::ToCurrent(v)
    } else {
        Position::Absolute(v)
    };

    Ok(pos)
}

pub async fn get_block_height_at<Co: ControllerBehaviour>(
    controller: &Co,
    pos: Position,
) -> Result<u64> {
    if let Position::Absolute(v) = pos {
        Ok(v)
    } else {
        let current = controller.get_block_number(false).await?;
        Ok(pos.with_current(current))
    }
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
pub fn safe_save(path: impl AsRef<Path>, content: &[u8], overwrite_existing: bool) -> Result<()> {
    let path = path.as_ref();
    let dir = path
        .parent()
        .ok_or_else(|| anyhow!("cannot load containing dir"))?;

    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(content)?;

    let mut f = if overwrite_existing {
        tmp.persist(path)?
    } else {
        tmp.persist_noclobber(path)?
    };
    f.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position() -> Result<()> {
        assert_eq!(parse_position("100")?, Position::Absolute(100));
        assert_eq!(parse_position("+100")?, Position::FromCurrent(100));
        assert_eq!(parse_position("-100")?, Position::ToCurrent(100));
        assert_eq!(parse_position("-0")?, Position::ToCurrent(0));
        assert_eq!(parse_position("+0")?, Position::FromCurrent(0));

        let current = 100u64;
        assert_eq!(parse_position("+1")?.with_current(current), 101);
        assert_eq!(parse_position("-1")?.with_current(current), 99);
        assert_eq!(parse_position("+0")?.with_current(current), 100);
        assert_eq!(parse_position("-0")?.with_current(current), 100);
        assert_eq!(parse_position("0")?.with_current(current), 0);
        assert_eq!(parse_position("-100")?.with_current(current), 0);
        assert_eq!(parse_position("-101")?.with_current(current), 0);
        assert_eq!(
            parse_position(&format!("+{}", u64::MAX))?.with_current(current),
            u64::MAX,
        );

        Ok(())
    }
}
