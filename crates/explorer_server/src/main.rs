mod migrations;
mod server;
mod watcher;

use tracing::{debug, error, info, warn};

use migrations::migrate_if_needed;


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Starting explorer server...");
    // TODO: make async
    match migrate_if_needed() {
        Ok(()) => info!("Migrations complete"),
        Err(e) => {
            error!("Error during migration: {e}");
            error!("Exiting.");
            std::process::exit(1);
        }
    }
    tokio::spawn(async move {
        watcher::start_watcher().await;
    });
    debug!("spawned watcher");
    tokio::spawn(async move {
        if let Err(e) = server::serve().await {
            error!("Error in HTTP server: {e}");
        }
        warn!("HTTP server exited");
    });
    debug!("spawned HTTP server");
    info!("Explorer server started");
}
