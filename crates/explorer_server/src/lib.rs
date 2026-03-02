use napi::{
    Status,
    threadsafe_function::ThreadsafeFunction,
};
use napi_derive::napi;
mod watcher;

#[napi]
#[allow(
    clippy::allow_attributes,
    clippy::unused_async,
    reason = "tokio::spawn tracks caller"
)]
pub async fn start(handle_build: ThreadsafeFunction<String, (), String, Status, false>) {
    println!("Starting explorer server...");
    tokio::spawn(async move {
        watcher::start_watcher(move |build_hash| {
            handle_build.call(
                build_hash,
                napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
            );
        })
        .await;
    });
}
