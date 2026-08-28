// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
mod json;
mod proto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Json,
    Protobuf,
}

pub trait Codec: Send + Sync {
    fn encode<T: serde::Serialize>(&self, val: &T) -> Result<Vec<u8>, CodecError>;
    fn decode<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T, CodecError>;
    fn content_type(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("encode error: {0}")]
    Encode(String),
    #[error("decode error: {0}")]
    Decode(String),
}

#[derive(Debug)]
pub enum CodecBox {
    Json(json::JsonCodec),
    Proto(proto::ProtoCodec),
}

impl Codec for CodecBox {
    fn encode<T: serde::Serialize>(&self, val: &T) -> Result<Vec<u8>, CodecError> {
        match self {
            Self::Json(c) => c.encode(val),
            Self::Proto(c) => c.encode(val),
        }
    }

    fn decode<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T, CodecError> {
        match self {
            Self::Json(c) => c.decode(data),
            Self::Proto(c) => c.decode(data),
        }
    }

    fn content_type(&self) -> &str {
        match self {
            Self::Json(c) => c.content_type(),
            Self::Proto(c) => c.content_type(),
        }
    }
}

pub fn codec_for(encoding: Encoding) -> CodecBox {
    match encoding {
        Encoding::Json => CodecBox::Json(json::JsonCodec),
        Encoding::Protobuf => CodecBox::Proto(proto::ProtoCodec),
    }
}

pub fn codec_from_content_type(ct: &str) -> Result<CodecBox, CodecError> {
    match ct {
        "application/json" => Ok(CodecBox::Json(json::JsonCodec)),
        "application/protobuf" | "application/x-protobuf" => Ok(CodecBox::Proto(proto::ProtoCodec)),
        other => Err(CodecError::Decode(format!(
            "unsupported content type: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestPayload {
        name: String,
        count: u32,
    }

    // --- JsonCodec ---

    #[test]
    fn json_codec_encode_decode_roundtrip() {
        let codec = json::JsonCodec;
        let payload = TestPayload {
            name: "test".into(),
            count: 42,
        };
        let encoded = codec.encode(&payload).unwrap();
        let decoded: TestPayload = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn json_codec_content_type() {
        assert_eq!(json::JsonCodec.content_type(), "application/json");
    }

    #[test]
    fn json_codec_decode_invalid_data() {
        let result: Result<TestPayload, _> = json::JsonCodec.decode(b"not json");
        assert!(result.is_err());
    }

    // --- ProtoCodec ---

    #[derive(Clone, PartialEq, prost::Message)]
    struct ProtoPayload {
        #[prost(string, tag = "1")]
        name: String,
        #[prost(uint32, tag = "2")]
        count: u32,
    }

    #[test]
    fn proto_codec_encode_returns_error() {
        let payload = TestPayload {
            name: "test".into(),
            count: 1,
        };
        let result = proto::ProtoCodec.encode(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn proto_codec_encode_message_roundtrip() {
        let codec = proto::ProtoCodec;
        let payload = ProtoPayload {
            name: "proto-test".into(),
            count: 42,
        };
        let bytes = codec.encode_message(&payload).unwrap();
        let decoded: ProtoPayload = codec.decode_message(&bytes).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn proto_codec_decode_message_rejects_garbage() {
        let codec = proto::ProtoCodec;
        let result: Result<ProtoPayload, _> = codec.decode_message(b"\xff\xff\xff");
        assert!(result.is_err());
    }

    #[test]
    fn proto_codec_content_type() {
        assert_eq!(proto::ProtoCodec.content_type(), "application/protobuf");
    }

    // --- CodecBox ---

    #[test]
    fn codec_box_json_roundtrip() {
        let codec = CodecBox::Json(json::JsonCodec);
        let payload = TestPayload {
            name: "boxed".into(),
            count: 7,
        };
        let encoded = codec.encode(&payload).unwrap();
        let decoded: TestPayload = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn codec_box_json_content_type() {
        let codec = CodecBox::Json(json::JsonCodec);
        assert_eq!(codec.content_type(), "application/json");
    }

    #[test]
    fn codec_box_proto_returns_errors() {
        let codec = CodecBox::Proto(proto::ProtoCodec);
        let payload = TestPayload {
            name: "p".into(),
            count: 1,
        };
        assert!(codec.encode(&payload).is_err());
        assert!(codec.decode::<TestPayload>(b"data").is_err());
    }

    // --- codec_for ---

    #[test]
    fn codec_for_json() {
        let codec = codec_for(Encoding::Json);
        assert_eq!(codec.content_type(), "application/json");
    }

    #[test]
    fn codec_for_protobuf() {
        let codec = codec_for(Encoding::Protobuf);
        assert_eq!(codec.content_type(), "application/protobuf");
    }

    // --- codec_from_content_type ---

    #[test]
    fn codec_from_ct_json() {
        let codec = codec_from_content_type("application/json").unwrap();
        assert_eq!(codec.content_type(), "application/json");
    }

    #[test]
    fn codec_from_ct_protobuf() {
        let codec = codec_from_content_type("application/protobuf").unwrap();
        assert_eq!(codec.content_type(), "application/protobuf");
    }

    #[test]
    fn codec_from_ct_x_protobuf() {
        let codec = codec_from_content_type("application/x-protobuf").unwrap();
        assert_eq!(codec.content_type(), "application/protobuf");
    }

    #[test]
    fn codec_from_ct_unknown_returns_error() {
        let result = codec_from_content_type("application/msgpack");
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CodecError::Decode(msg) => assert!(msg.contains("msgpack")),
            _ => panic!("expected Decode error"),
        }
    }

    // --- Encoding ---

    #[test]
    fn encoding_variants() {
        assert_eq!(Encoding::Json, Encoding::Json);
        assert_eq!(Encoding::Protobuf, Encoding::Protobuf);
        assert_ne!(Encoding::Json, Encoding::Protobuf);
    }

    // --- 边界情况 ---

    #[test]
    fn json_codec_encode_nan_as_null() {
        // serde_json 默认将非有限浮点编码为 null（不会报错）
        let bytes = json::JsonCodec.encode(&f64::NAN).unwrap();
        assert_eq!(bytes, b"null");
        let decoded: Option<f64> = json::JsonCodec.decode(&bytes).unwrap();
        assert_eq!(decoded, None);
    }

    #[test]
    fn json_codec_decode_empty_bytes_fails() {
        let result: Result<TestPayload, _> = json::JsonCodec.decode(b"");
        assert!(result.is_err());
    }

    #[test]
    fn codec_box_decode_rejects_invalid_json() {
        let codec = CodecBox::Json(json::JsonCodec);
        assert!(codec.decode::<TestPayload>(b"{").is_err());
    }

    #[test]
    fn proto_codec_empty_message_roundtrip() {
        let payload = ProtoPayload {
            name: String::new(),
            count: 0,
        };
        let bytes = proto::ProtoCodec.encode_message(&payload).unwrap();
        assert!(bytes.is_empty());
        let decoded: ProtoPayload = proto::ProtoCodec.decode_message(&bytes).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn codec_error_variants_display() {
        assert!(
            CodecError::Encode("e".into())
                .to_string()
                .contains("encode")
        );
        assert!(
            CodecError::Decode("d".into())
                .to_string()
                .contains("decode")
        );
    }
}
