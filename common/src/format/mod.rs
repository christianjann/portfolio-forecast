mod binary;
mod zip;

use anyhow::Result;
use crate::proto::PClient;
use std::path::Path;

/// Detect the file format and load a PClient from a `.portfolio` file.
///
/// Supported formats:
/// - Unencrypted ZIP (PK\x03\x04) containing a `data.portfolio` entry
/// - Raw binary (PPPBV1 signature) written directly
pub fn load_file(path: &Path) -> Result<PClient> {
    let data = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {:?}: {}", path, e))?;
    load_bytes(&data)
}

/// Detect the file format and load a PClient from raw bytes.
pub fn load_bytes(data: &[u8]) -> Result<PClient> {
    if data.starts_with(b"PK\x03\x04") {
        zip::load(data)
    } else if data.starts_with(b"PPPBV1") {
        binary::load(data)
    } else if data.starts_with(b"PORTFOLIO") {
        Err(anyhow::anyhow!(
            "Encrypted portfolio files are not supported. \
             Please export as unencrypted from Portfolio Performance."
        ))
    } else {
        Err(anyhow::anyhow!(
            "Unknown file format (not a valid .portfolio file)"
        ))
    }
}
