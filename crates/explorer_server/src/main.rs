mod migrations;
mod server;
mod watcher;

use std::process;

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
            process::exit(1);
        }
    }
    let mut tasks = Vec::new();
    tasks.push(tokio::spawn(async move {
        watcher::start_watcher().await;
    }));
    debug!("spawned watcher");
    tasks.push(tokio::spawn(async move {
        if let Err(e) = server::serve().await {
            error!("Error in HTTP server: {e}");
        }
        warn!("HTTP server exited");
    }));
    debug!("spawned HTTP server");
    info!("Explorer server started");
    for task in tasks {
        if let Err(e) = task.await {
            error!("Task failed: {e}");
            process::exit(1);
        }
    }
}
