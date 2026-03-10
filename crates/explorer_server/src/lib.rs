use explorer_types::FullBundle;
use napi::{Env, JsValue, Status, Unknown, bindgen_prelude::JsValuesTuple as _, threadsafe_function::ThreadsafeFunction};
use napi_derive::napi;
mod migrations;
mod util;
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

#[napi]
pub fn write_build<'env>(build_data: Unknown<'env>) -> napi::Result<()> {
    let env = Env::from_raw(build_data.env());
    let build_data: FullBundle = env.from_js_value(build_data)?;
    Ok(())
}

async fn internal_write_build(build: FullBundle) -> anyhow::Result<()> {
    Ok(())
}


#[napi_derive::module_init]
fn init() {
    tracing_subscriber::fmt().init();
}
