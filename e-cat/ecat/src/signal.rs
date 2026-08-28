// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
pub async fn wait_for_shutdown() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => ::tracing::info!("received SIGINT"),
        _ = terminate => ::tracing::info!("received SIGTERM"),
    }
}
