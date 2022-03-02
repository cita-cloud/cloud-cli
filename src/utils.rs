use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;

use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

use crate::crypto::ArrayLike;
use crate::crypto::Crypto;
use crate::crypto::{Address, Hash};

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

pub fn display_time(timestamp: u64) -> String {
    let local_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
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
    let dir = path.parent().ok_or(anyhow!("cannot load containing dir"))?;

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
