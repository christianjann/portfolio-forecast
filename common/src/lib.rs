mod format;
pub mod analysis;
pub(crate) mod proto;
pub mod views;
#[cfg(target_os = "android")]
pub(crate) mod android_content_reader;

use anyhow::Result;
use std::path::Path;

pub use analysis::NavPoint;
pub use proto::PClient;

/// Load a Portfolio Performance `.portfolio` file into a `PClient`.
///
/// Detects format automatically:
/// - ZIP (PK\x03\x04 magic) containing a `*.portfolio` entry
/// - Raw binary (PPPBV1 magic) written by ProtobufWriter
pub fn load_file(path: &Path) -> Result<PClient> {
    format::load_file(path)
}

/// Load a `PClient` from raw bytes (e.g. read from an Android content URI).
///
/// Accepts the same formats as [`load_file`]: ZIP-wrapped binary or raw PPPBV1.
pub fn load_from_bytes(bytes: &[u8]) -> Result<PClient> {
    format::load_bytes(bytes)
}
