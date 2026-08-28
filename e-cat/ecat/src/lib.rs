// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
mod hook;
mod reexports;
mod signal;

pub use hook::LifecycleHook;
// --no-default-features 时 glob 为空，allow 掉 unused import 告警。
#[allow(unused_imports)]
pub use reexports::*;
pub use signal::wait_for_shutdown;

use ecat_transport::Server;
use std::sync::Arc;

pub struct App {
    name: String,
    version: String,
    servers: Vec<Arc<dyn Server>>,
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::default()
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 用户若已先初始化 OTLP/ecat-tracing 等 subscriber，则不再重复初始化，
        // 避免 "a global default trace dispatcher has already been set" panic。
        if !::tracing::dispatcher::has_been_set() {
            ecat_logging::init();
        }

        ::tracing::info!(
            name = self.name,
            version = self.version,
            "starting application"
        );

        for hook in &self.start_hooks {
            hook.on_start().await?;
        }

        let mut handles = Vec::new();
        for server in &self.servers {
            let server = Arc::clone(server);
            handles.push(tokio::spawn(async move {
                if let Err(e) = server.start().await {
                    ::tracing::error!(error = %e, "server error");
                }
            }));
        }

        wait_for_shutdown().await;

        ::tracing::info!("shutting down");
        for hook in &self.stop_hooks {
            hook.on_stop().await?;
        }
        for server in &self.servers {
            if let Err(e) = server.stop().await {
                ::tracing::error!(error = %e, "server stop error");
            }
        }
        for handle in handles {
            if let Err(e) = handle.await {
                ::tracing::error!(error = %e, "server task panicked");
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct AppBuilder {
    name: Option<String>,
    version: Option<String>,
    servers: Vec<Arc<dyn Server>>,
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
}

impl AppBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn server(mut self, server: impl Server + 'static) -> Self {
        self.servers.push(Arc::new(server));
        self
    }

    pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
        self.start_hooks.push(Box::new(hook));
        self
    }

    pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
        self.stop_hooks.push(Box::new(hook));
        self
    }

    pub fn build(self) -> Result<App, Box<dyn std::error::Error + Send + Sync>> {
        Ok(App {
            name: self.name.unwrap_or_else(|| "ecat-app".into()),
            version: self.version.unwrap_or_else(|| "0.1.0".into()),
            servers: self.servers,
            start_hooks: self.start_hooks,
            stop_hooks: self.stop_hooks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecat_transport::Server;

    struct TestServer;
    #[async_trait::async_trait]
    impl Server for TestServer {
        async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
        async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn builder_defaults() {
        let app = App::builder().build().unwrap();
        assert_eq!(app.name, "ecat-app");
        assert_eq!(app.version, "0.1.0");
    }

    #[tokio::test]
    async fn builder_custom_name_version() {
        let app = App::builder()
            .name("myapp")
            .version("2.0.0")
            .build()
            .unwrap();
        assert_eq!(app.name, "myapp");
        assert_eq!(app.version, "2.0.0");
    }

    #[tokio::test]
    async fn builder_with_server() {
        let app = App::builder().server(TestServer).build().unwrap();
        assert_eq!(app.servers.len(), 1);
    }

    #[tokio::test]
    async fn builder_with_lifecycle_hooks() {
        #[derive(Default)]
        struct TestHook;
        #[async_trait::async_trait]
        impl LifecycleHook for TestHook {}

        let app = App::builder()
            .on_start(TestHook)
            .on_stop(TestHook)
            .build()
            .unwrap();
        assert_eq!(app.start_hooks.len(), 1);
        assert_eq!(app.stop_hooks.len(), 1);
    }
}
