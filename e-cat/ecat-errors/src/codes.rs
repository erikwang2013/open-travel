// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use ecat_protos::errors::ErrorCode;
use http::StatusCode;

pub trait ErrorCodeExt {
    fn http_status(&self) -> StatusCode;
}

impl ErrorCodeExt for ErrorCode {
    fn http_status(&self) -> StatusCode {
        match self {
            ErrorCode::Ok => StatusCode::OK,
            ErrorCode::InvalidArgument => StatusCode::BAD_REQUEST,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::AlreadyExists => StatusCode::CONFLICT,
            ErrorCode::PermissionDenied => StatusCode::FORBIDDEN,
            ErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
            ErrorCode::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::Internal | ErrorCode::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
        }
    }
}
