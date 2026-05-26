use anyhow::Result;
use crate::proto::PClient;

/// Binary format: 6-byte "PPPBV1" magic + raw protobuf bytes.
const SIGNATURE: &[u8] = b"PPPBV1";

pub fn load(data: &[u8]) -> Result<PClient> {
    if !data.starts_with(SIGNATURE) {
        return Err(anyhow::anyhow!("Invalid binary signature (expected PPPBV1)"));
    }
    let payload = &data[SIGNATURE.len()..];
    prost::Message::decode(payload)
        .map_err(|e| anyhow::anyhow!("Protobuf decode error: {}", e))
}
