#[cfg(unix)]
pub async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate =
        signal(SignalKind::terminate()).expect("the SIGTERM handler can always be installed");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => tracing::info!("received SIGINT; draining"),
        _ = terminate.recv() => tracing::info!("received SIGTERM; draining"),
    }
}

#[cfg(not(unix))]
pub async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("received an interrupt; draining");
}

#[cfg(all(test, unix))]
mod tests {
    use std::time::Duration;

    use tokio::signal::unix::{SignalKind, signal};

    use super::*;

    async fn drains_on(kind: SignalKind, flag: &str) {
        let _guard = signal(kind).expect("guard the default action before anything is raised");
        let shutdown = tokio::spawn(shutdown_signal());
        tokio::time::sleep(Duration::from_millis(200)).await;

        let sent = std::process::Command::new("kill")
            .args([flag, &std::process::id().to_string()])
            .status()
            .expect("kill");
        assert!(sent.success());

        tokio::time::timeout(Duration::from_secs(5), shutdown)
            .await
            .expect("the signal was never observed")
            .expect("the shutdown task panicked");
    }

    #[tokio::test]
    async fn sigterm_drains() {
        drains_on(SignalKind::terminate(), "-TERM").await;
    }

    #[tokio::test]
    async fn sigint_drains() {
        drains_on(SignalKind::interrupt(), "-INT").await;
    }
}
