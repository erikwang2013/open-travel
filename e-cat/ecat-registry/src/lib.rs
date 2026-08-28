// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
mod memory;

pub use memory::MemoryRegistry;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub version: String,
    pub endpoints: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ServiceInfo {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            endpoints: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoints.push(endpoint.into());
        self
    }
}

pub struct Registration {
    pub id: String,
    pub service: ServiceInfo,
    registry: Option<std::sync::Arc<dyn Registry>>,
}

impl Registration {
    pub fn new(id: String, service: ServiceInfo, registry: Arc<dyn Registry>) -> Self {
        Self {
            id,
            service,
            registry: Some(registry),
        }
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(reg) = self.registry.take() {
            let id = self.id.clone();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    if let Err(e) = reg.deregister(&id).await {
                        tracing::warn!(service_id = %id, error = %e, "auto-deregister on drop failed");
                    }
                });
            } else {
                tracing::warn!(service_id = %id, "runtime dropped; cannot auto-deregister");
            }
        }
    }
}

#[async_trait]
pub trait Registry: Send + Sync {
    async fn register(&self, service: ServiceInfo) -> Result<Registration, RegistryError>;
    async fn deregister(&self, id: &str) -> Result<(), RegistryError>;
    async fn discover(&self, name: &str) -> Result<Vec<ServiceInfo>, RegistryError>;
    async fn list_services(&self) -> Result<Vec<String>, RegistryError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("service not found: {0}")]
    NotFound(String),
    #[error("registry error: {0}")]
    Other(String),
}
