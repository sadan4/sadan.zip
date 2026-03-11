use napi::{
    Status,
    threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use napi_derive::napi;
use tracing::{info, warn};

use migrations::migrate_if_needed;

use crate::watcher::Channel;
mod migrations;
mod server;
mod util;
mod watcher;
pub mod wrapper_types;

#[napi(object)]
pub struct HandleBuildOpts {
    pub build_hash: String,
    pub html: String,
    pub channel: Channel,
}

#[napi]
#[allow(
    clippy::allow_attributes,
    clippy::unused_async,
    reason = "tokio::spawn tracks caller"
)]
pub async fn start(
    handle_build: ThreadsafeFunction<HandleBuildOpts, (), HandleBuildOpts, Status, false>,
) {
    info!("Starting explorer server...");
    match migrate_if_needed() {
        Ok(()) => info!("Migrations complete"),
        Err(e) => {
            panic!("Error during migration: {e:?}");
        }
    }
    tokio::spawn(async move {
        watcher::start_watcher(move |opts| {
            handle_build.call(opts, ThreadsafeFunctionCallMode::NonBlocking);
        })
        .await;
    });
    tokio::spawn(async move {
        if let Err(e) = server::serve().await {
            panic!("Error in HTTP server: {e}");
        }
        warn!("HTTP server exited");
    });
}

#[napi_derive::module_init]
fn init() {
    tracing_subscriber::fmt().init();
}
