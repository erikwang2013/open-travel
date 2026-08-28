// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use crate::{Registration, Registry, RegistryError, ServiceInfo};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct MemoryRegistry {
    services: Arc<RwLock<HashMap<String, Arc<ServiceInfo>>>>,
}

impl Default for MemoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Registry for MemoryRegistry {
    async fn register(&self, service: ServiceInfo) -> Result<Registration, RegistryError> {
        let id = Uuid::new_v4().to_string();
        let mut services = self.services.write().await;
        services.insert(id.clone(), Arc::new(service.clone()));
        Ok(Registration::new(
            id,
            service,
            Arc::new(MemoryRegistry {
                services: Arc::clone(&self.services),
            }),
        ))
    }

    async fn deregister(&self, id: &str) -> Result<(), RegistryError> {
        let mut services = self.services.write().await;
        services
            .remove(id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        Ok(())
    }

    async fn discover(&self, name: &str) -> Result<Vec<ServiceInfo>, RegistryError> {
        let services = self.services.read().await;
        let results: Vec<ServiceInfo> = services
            .values()
            .filter(|s| s.name == name)
            .map(|s| s.as_ref().clone())
            .collect();
        Ok(results)
    }

    async fn list_services(&self) -> Result<Vec<String>, RegistryError> {
        let services = self.services.read().await;
        let names: Vec<String> = services.values().map(|s| s.name.clone()).collect();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service(name: &str) -> ServiceInfo {
        ServiceInfo::new(name, "1.0.0").with_endpoint("http://localhost:8080")
    }

    #[tokio::test]
    async fn register_and_discover() {
        let reg = MemoryRegistry::new();
        let r = reg.register(test_service("auth")).await.unwrap();
        assert!(!r.id.is_empty());

        let found = reg.discover("auth").await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "auth");
    }

    #[tokio::test]
    async fn deregister_removes_service() {
        let reg = MemoryRegistry::new();
        let r = reg.register(test_service("auth")).await.unwrap();
        reg.deregister(&r.id).await.unwrap();
        assert!(reg.discover("auth").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn deregister_not_found() {
        let reg = MemoryRegistry::new();
        assert!(reg.deregister("nope").await.is_err());
    }

    #[tokio::test]
    async fn list_services_returns_names() {
        let reg = MemoryRegistry::new();
        reg.register(test_service("auth")).await.unwrap();
        reg.register(test_service("gw")).await.unwrap();

        let names = reg.list_services().await.unwrap();
        assert!(names.contains(&"auth".to_string()));
        assert!(names.contains(&"gw".to_string()));
    }

    #[tokio::test]
    async fn discover_filters_by_name() {
        let reg = MemoryRegistry::new();
        reg.register(test_service("auth")).await.unwrap();
        reg.register(test_service("gw")).await.unwrap();

        let found = reg.discover("gw").await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "gw");
    }

    #[tokio::test]
    async fn discover_returns_all_instances_of_same_name() {
        let reg = MemoryRegistry::new();
        reg.register(test_service("dup")).await.unwrap();
        reg.register(test_service("dup")).await.unwrap();
        assert_eq!(reg.discover("dup").await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn registration_drop_auto_deregisters() {
        let reg = MemoryRegistry::new();
        let registration = reg.register(test_service("ephemeral")).await.unwrap();
        drop(registration);
        // drop 在运行时内 spawn 异步 deregister；yield 循环等待其生效
        for _ in 0..10_000 {
            if reg.discover("ephemeral").await.unwrap().is_empty() {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("auto-deregister on drop never ran");
    }

    #[test]
    fn service_info_builder_defaults() {
        let svc = ServiceInfo::new("s", "1.0");
        assert!(svc.endpoints.is_empty());
        assert!(svc.metadata.is_empty());
        let svc = svc.with_endpoint("http://x:1");
        assert_eq!(svc.endpoints, vec!["http://x:1"]);
        assert_eq!(svc.name, "s");
        assert_eq!(svc.version, "1.0");
    }
}
