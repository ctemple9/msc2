//! DuckDNS is a stored player-facing hostname, not an updater integration.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuckDnsError {
    InvalidHostname,
}

pub fn normalize_hostname(input: Option<&str>) -> Result<Option<String>, DuckDnsError> {
    let Some(hostname) = input.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let hostname = hostname.to_ascii_lowercase();
    if hostname.len() > 253
        || !hostname.ends_with(".duckdns.org")
        || hostname.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || !label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                || label.starts_with('-')
                || label.ends_with('-')
        })
    {
        return Err(DuckDnsError::InvalidHostname);
    }
    Ok(Some(hostname))
}
