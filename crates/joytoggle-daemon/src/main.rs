mod daemon;
mod toggler;

use std::sync::Arc;

use joytoggle_core::{FileCacheStore, FileStateStore, LinuxSysfsReader};
use tracing::info;
use zbus::connection::Builder as ConnectionBuilder;

use daemon::JoyToggleDaemon;
use toggler::SysfsDeviceToggler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("joytoggle_daemon=info".parse()?),
        )
        .init();

    if !nix::unistd::Uid::effective().is_root() {
        anyhow::bail!("joytoggle-daemon must run as root");
    }

    info!("joytoggle-daemon starting");

    let daemon = JoyToggleDaemon {
        toggler: Arc::new(SysfsDeviceToggler),
        state_store: Arc::new(FileStateStore),
        scanner: Arc::new(LinuxSysfsReader),
        cache: Arc::new(FileCacheStore::default()),
    };

    let _conn = ConnectionBuilder::system()?
        .name("org.joytoggle.Daemon")?
        .serve_at("/org/joytoggle/Daemon", daemon)?
        .build()
        .await?;

    info!("serving on org.joytoggle.Daemon");

    tokio::signal::ctrl_c().await?;
    info!("received ctrl-c, shutting down");

    Ok(())
}
