// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! Protobuf codec.
//!
//! The `Codec` trait methods (`encode`/`decode`) use serde bounds and always
//! return an error. Use `ProtoCodec::encode_message()` and
//! `ProtoCodec::decode_message()` for protobuf serialization — they accept
//! `prost::Message` types when the `prost-codec` feature is enabled.
//!
//! ```ignore
//! // Requires `prost-codec` feature and a prost::Message type
//! # #[cfg(feature = "prost-codec")] {
//! let codec = ProtoCodec;
//! let bytes = codec.encode_message(&my_proto_msg)?;
//! let msg: MyProto = codec.decode_message(&bytes)?;
//! # }
//! ```
use super::{Codec, CodecError};

#[derive(Debug)]
/// Protobuf codec. Implements `Codec` for serde-compat, but the primary
/// API is `encode_message()` / `decode_message()` with `prost::Message`.
pub struct ProtoCodec;

impl ProtoCodec {
    /// Encode a protobuf message using prost.
    #[cfg(feature = "prost-codec")]
    pub fn encode_message<T: prost::Message>(&self, val: &T) -> Result<Vec<u8>, CodecError> {
        let mut buf = Vec::with_capacity(val.encoded_len());
        val.encode(&mut buf)
            .map_err(|e| CodecError::Encode(e.to_string()))?;
        Ok(buf)
    }

    /// Decode a protobuf message using prost.
    #[cfg(feature = "prost-codec")]
    pub fn decode_message<T: prost::Message + Default>(
        &self,
        data: &[u8],
    ) -> Result<T, CodecError> {
        T::decode(data).map_err(|e| CodecError::Decode(e.to_string()))
    }

    /// Encode using prost (stub without feature).
    #[cfg(not(feature = "prost-codec"))]
    pub fn encode_message<T>(&self, _val: &T) -> Result<Vec<u8>, CodecError> {
        Err(CodecError::Encode(
            "enable 'prost-codec' feature and implement prost::Message".into(),
        ))
    }

    /// Decode using prost (stub without feature).
    #[cfg(not(feature = "prost-codec"))]
    pub fn decode_message<T: Default>(&self, _data: &[u8]) -> Result<T, CodecError> {
        Err(CodecError::Decode(
            "enable 'prost-codec' feature and implement prost::Message".into(),
        ))
    }
}

impl Codec for ProtoCodec {
    // Codec trait 对 encode/decode 施加 serde 约束（Serialize / DeserializeOwned），
    // 而 prost 序列化要求 prost::Message——两个 trait 在泛型层面无法桥接，
    // 因此 trait 方法只能返回明确错误。真实的 protobuf 编解码请使用
    // encode_message() / decode_message()（要求 T: prost::Message）。
    fn encode<T: serde::Serialize>(&self, _val: &T) -> Result<Vec<u8>, CodecError> {
        Err(CodecError::Encode(
            "ProtoCodec: Codec trait requires serde bounds, which prost cannot encode generically; \
             use encode_message() with a prost::Message type"
                .into(),
        ))
    }

    fn decode<T: serde::de::DeserializeOwned>(&self, _data: &[u8]) -> Result<T, CodecError> {
        Err(CodecError::Decode(
            "ProtoCodec: Codec trait requires serde bounds, which prost cannot decode generically; \
             use decode_message() with a prost::Message type"
                .into(),
        ))
    }

    fn content_type(&self) -> &str {
        "application/protobuf"
    }
}
