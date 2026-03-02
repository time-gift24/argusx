use mac_address::get_mac_address;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HostFingerprintError {
    #[error("failed to inspect local machine address: {0}")]
    MacLookup(#[from] mac_address::MacAddressError),
    #[error("no machine address available on this host")]
    NoMacAddress,
    #[error("failed to read hostname: {0}")]
    HostnameLookup(#[from] std::io::Error),
    #[error("hostname is not valid UTF-8")]
    HostnameUtf8,
}

pub fn derive_fingerprint(machine_address: &str, hostname: &str) -> String {
    let normalized_mac = machine_address
        .trim()
        .to_ascii_lowercase()
        .replace('-', ":");
    let normalized_host = hostname.trim().to_ascii_lowercase();

    let mut hasher = Sha256::new();
    hasher.update(normalized_mac.as_bytes());
    hasher.update(b"|");
    hasher.update(normalized_host.as_bytes());
    let digest = hasher.finalize();

    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

pub fn load_host_fingerprint() -> Result<String, HostFingerprintError> {
    let machine_address = get_mac_address()?.ok_or(HostFingerprintError::NoMacAddress)?;
    let hostname = hostname::get()?
        .into_string()
        .map_err(|_| HostFingerprintError::HostnameUtf8)?;
    Ok(derive_fingerprint(&machine_address.to_string(), &hostname))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_for_same_input() {
        let a = derive_fingerprint("00:11:22:33:44:55", "my-host");
        let b = derive_fingerprint("00:11:22:33:44:55", "my-host");
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_changes_when_input_changes() {
        let a = derive_fingerprint("00:11:22:33:44:55", "my-host");
        let b = derive_fingerprint("00:11:22:33:44:56", "my-host");
        assert_ne!(a, b);
    }
}
