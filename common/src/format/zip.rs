use anyhow::Result;
use crate::proto::PClient;
use std::io::Read;

/// ZIP format: PK\x03\x04 magic; contains a "data.portfolio" entry in binary format.
pub fn load(data: &[u8]) -> Result<PClient> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| anyhow::anyhow!("Failed to open ZIP: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| anyhow::anyhow!("Failed to read ZIP entry {}: {}", i, e))?;

        if entry.name().ends_with(".portfolio") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return super::binary::load(&buf);
        }
    }

    Err(anyhow::anyhow!(
        "No .portfolio entry found inside ZIP archive"
    ))
}
