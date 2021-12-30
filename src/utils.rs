use anyhow::anyhow;
use anyhow::Result;

pub fn parse_addr(s: &str) -> Result<Vec<u8>> {
    let s = remove_0x(s);
    if s.len() > 40 {
        return Err(anyhow!("can't parse addr, the given str is too long"));
    }
    // padding 0 to 20 bytes
    let padded = format!("{:0>40}", s);
    Ok(hex::decode(&padded)?)
}

pub fn parse_value(s: &str) -> Result<Vec<u8>> {
    let s = remove_0x(s);
    if s.len() > 64 {
        return Err(anyhow!("can't parse value, the given str is too long"));
    }
    // padding 0 to 32 bytes
    let padded = format!("{:0>64}", s);
    Ok(hex::decode(&padded)?)
}

pub fn parse_data(s: &str) -> Result<Vec<u8>> {
    Ok(hex::decode(remove_0x(s))?)
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
    time::OffsetDateTime::from_unix_timestamp(timestamp as i64)
        .expect("invalid timestamp")
        .to_offset(local_offset)
        .format(&format)
        .unwrap()
}

fn remove_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}
