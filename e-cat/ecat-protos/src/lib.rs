// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
pub mod errors {
    tonic::include_proto!("ecat.errors");
}

pub mod metadata {
    tonic::include_proto!("ecat.metadata");
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn error_roundtrips_through_prost() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("retryable".to_string(), "true".to_string());
        let err = errors::Error {
            code: errors::ErrorCode::PermissionDenied as i32,
            reason: "PERMISSION_DENIED".into(),
            message: "no access".into(),
            metadata,
        };
        let bytes = err.encode_to_vec();
        let decoded = errors::Error::decode(bytes.as_slice()).unwrap();
        assert_eq!(decoded.code, 1004);
        assert_eq!(decoded.reason, "PERMISSION_DENIED");
        assert_eq!(decoded.message, "no access");
        assert_eq!(
            decoded.metadata.get("retryable").map(String::as_str),
            Some("true")
        );
        assert_eq!(decoded.metadata.len(), 1);
    }

    #[test]
    fn error_code_enum_values_match_proto() {
        assert_eq!(errors::ErrorCode::Ok as i32, 0);
        assert_eq!(errors::ErrorCode::InvalidArgument as i32, 1001);
        assert_eq!(errors::ErrorCode::PermissionDenied as i32, 1004);
        assert_eq!(errors::ErrorCode::DeadlineExceeded as i32, 1009);
    }

    #[test]
    fn metadata_roundtrips_through_prost() {
        let mut pairs = std::collections::HashMap::new();
        pairs.insert("env".to_string(), "prod".to_string());
        pairs.insert("region".to_string(), "cn-east".to_string());
        let md = metadata::Metadata { pairs };
        let decoded = metadata::Metadata::decode(md.encode_to_vec().as_slice()).unwrap();
        assert_eq!(decoded, md);
        assert_eq!(decoded.pairs.len(), 2);
    }

    #[test]
    fn empty_message_roundtrips() {
        let err = errors::Error {
            code: 0,
            reason: String::new(),
            message: String::new(),
            metadata: Default::default(),
        };
        let decoded = errors::Error::decode(err.encode_to_vec().as_slice()).unwrap();
        assert_eq!(decoded, err);
        assert!(decoded.metadata.is_empty());
    }

    #[test]
    fn all_error_code_values_match_proto() {
        assert_eq!(errors::ErrorCode::Unknown as i32, 1000);
        assert_eq!(errors::ErrorCode::AlreadyExists as i32, 1003);
        assert_eq!(errors::ErrorCode::Unauthenticated as i32, 1005);
        assert_eq!(errors::ErrorCode::ResourceExhausted as i32, 1006);
        assert_eq!(errors::ErrorCode::Internal as i32, 1007);
        assert_eq!(errors::ErrorCode::Unavailable as i32, 1008);
    }

    #[test]
    fn error_decode_truncated_fails() {
        let err = errors::Error {
            code: 1,
            reason: "a-reason-longer-than-truncation".into(),
            message: "m".into(),
            metadata: Default::default(),
        };
        let bytes = err.encode_to_vec();
        // 截断到 reason 字段中间（缺 3 字节可能正好落在字段边界，prost 容错）
        let truncated = &bytes[..10];
        assert!(errors::Error::decode(truncated).is_err());
    }

    #[test]
    fn decode_empty_buffer_gives_default() {
        let err = errors::Error::decode(&b""[..]).unwrap();
        assert_eq!(err.code, 0);
        assert!(err.reason.is_empty());
        assert!(err.message.is_empty());
    }

    #[test]
    fn metadata_multiple_pairs_roundtrip() {
        let mut pairs = std::collections::HashMap::new();
        pairs.insert("a".to_string(), "1".to_string());
        pairs.insert("b".to_string(), "2".to_string());
        pairs.insert("c".to_string(), "3".to_string());
        let md = metadata::Metadata { pairs };
        let decoded = metadata::Metadata::decode(md.encode_to_vec().as_slice()).unwrap();
        assert_eq!(decoded, md);
        assert_eq!(decoded.pairs.len(), 3);
    }
}
