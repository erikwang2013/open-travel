// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    #[serde(default)]
    pub sub: String,
    #[serde(default)]
    pub exp: Option<u64>,
    #[serde(default)]
    pub iat: Option<u64>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl AuthClaims {
    pub fn subject(&self) -> &str {
        &self.sub
    }

    pub fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.role.as_deref() == Some(role)
    }
}
