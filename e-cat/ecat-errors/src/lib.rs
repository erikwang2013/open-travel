// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
mod codes;

use codes::ErrorCodeExt;
pub use ecat_protos::errors::ErrorCode;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub struct Error {
    pub code: ErrorCode,
    pub reason: String,
    pub message: String,
    #[source]
    pub cause: Option<Box<dyn std::error::Error + Send + Sync>>,
    pub metadata: HashMap<String, String>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}: {}", self.code, self.reason, self.message)
    }
}

impl Error {
    pub fn new(code: ErrorCode, reason: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
            message: message.into(),
            cause: None,
            metadata: HashMap::new(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn from_status(status: tonic::Status) -> Self {
        let code = match status.code() {
            tonic::Code::Ok => ErrorCode::Ok,
            tonic::Code::NotFound => ErrorCode::NotFound,
            tonic::Code::InvalidArgument => ErrorCode::InvalidArgument,
            tonic::Code::PermissionDenied => ErrorCode::PermissionDenied,
            tonic::Code::Unauthenticated => ErrorCode::Unauthenticated,
            tonic::Code::ResourceExhausted => ErrorCode::ResourceExhausted,
            tonic::Code::AlreadyExists => ErrorCode::AlreadyExists,
            tonic::Code::Unavailable => ErrorCode::Unavailable,
            tonic::Code::DeadlineExceeded => ErrorCode::DeadlineExceeded,
            tonic::Code::Unknown => ErrorCode::Unknown,
            _ => ErrorCode::Internal,
        };
        Self::new(code, "grpc_error", status.message().to_string())
    }

    pub fn to_http_status(&self) -> http::StatusCode {
        self.code.http_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;

    #[test]
    fn test_error_code_http_mapping() {
        assert_eq!(ErrorCode::Ok.http_status(), StatusCode::OK);
        assert_eq!(ErrorCode::NotFound.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(
            ErrorCode::InvalidArgument.http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorCode::PermissionDenied.http_status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorCode::Unauthenticated.http_status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ErrorCode::Internal.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ErrorCode::Unavailable.http_status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            ErrorCode::DeadlineExceeded.http_status(),
            StatusCode::GATEWAY_TIMEOUT
        );
    }

    #[test]
    fn test_from_status_maps_codes() {
        let err = Error::from_status(tonic::Status::not_found("missing"));
        assert_eq!(err.code, ErrorCode::NotFound);

        let err = Error::from_status(tonic::Status::permission_denied("nope"));
        assert_eq!(err.code, ErrorCode::PermissionDenied);

        let err = Error::from_status(tonic::Status::unauthenticated("bad token"));
        assert_eq!(err.code, ErrorCode::Unauthenticated);
    }

    #[test]
    fn test_with_metadata_accumulates() {
        let err = Error::new(ErrorCode::Ok, "test", "msg")
            .with_metadata("key1", "val1")
            .with_metadata("key2", "val2");
        assert_eq!(err.metadata.get("key1").unwrap(), "val1");
        assert_eq!(err.metadata.get("key2").unwrap(), "val2");
    }

    #[test]
    fn test_display_format() {
        let err = Error::new(ErrorCode::NotFound, "user_not_found", "user 42 not found");
        let s = err.to_string();
        assert!(s.contains("NotFound"));
        assert!(s.contains("user_not_found"));
        assert!(s.contains("user 42 not found"));
    }

    #[test]
    fn test_remaining_http_mappings() {
        assert_eq!(ErrorCode::AlreadyExists.http_status(), StatusCode::CONFLICT);
        assert_eq!(
            ErrorCode::ResourceExhausted.http_status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            ErrorCode::Unknown.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ErrorCode::Internal.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_from_status_remaining_codes() {
        assert_eq!(
            Error::from_status(tonic::Status::already_exists("dup")).code,
            ErrorCode::AlreadyExists
        );
        assert_eq!(
            Error::from_status(tonic::Status::resource_exhausted("limit")).code,
            ErrorCode::ResourceExhausted
        );
        assert_eq!(
            Error::from_status(tonic::Status::unavailable("down")).code,
            ErrorCode::Unavailable
        );
        assert_eq!(
            Error::from_status(tonic::Status::deadline_exceeded("slow")).code,
            ErrorCode::DeadlineExceeded
        );
        assert_eq!(
            Error::from_status(tonic::Status::unknown("x")).code,
            ErrorCode::Unknown
        );
        assert_eq!(
            Error::from_status(tonic::Status::internal("boom")).code,
            ErrorCode::Internal
        );
    }

    #[test]
    fn test_from_status_unmapped_code_becomes_internal() {
        let err = Error::from_status(tonic::Status::failed_precondition("nope"));
        assert_eq!(err.code, ErrorCode::Internal);
        let err = Error::from_status(tonic::Status::new(tonic::Code::Ok, ""));
        assert_eq!(err.code, ErrorCode::Ok);
    }

    #[test]
    fn test_to_http_status() {
        let err = Error::new(ErrorCode::Unavailable, "x", "y");
        assert_eq!(err.to_http_status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_cause_is_exposed_via_source() {
        use std::error::Error as _;
        let inner = std::io::Error::other("io boom");
        let err = Error {
            cause: Some(Box::new(inner)),
            ..Error::new(ErrorCode::Internal, "io", "failed")
        };
        assert!(err.source().is_some());
        let no_cause = Error::new(ErrorCode::Ok, "x", "y");
        assert!(no_cause.source().is_none());
    }
}
