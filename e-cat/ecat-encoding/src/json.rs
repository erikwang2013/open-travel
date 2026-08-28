// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use super::{Codec, CodecError};

#[derive(Debug)]
pub struct JsonCodec;

impl Codec for JsonCodec {
    fn encode<T: serde::Serialize>(&self, val: &T) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(val).map_err(|e| CodecError::Encode(e.to_string()))
    }

    fn decode<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T, CodecError> {
        serde_json::from_slice(data).map_err(|e| CodecError::Decode(e.to_string()))
    }

    fn content_type(&self) -> &str {
        "application/json"
    }
}
