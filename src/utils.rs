use env_logger::builder as log_builder;
use std::env;

pub use log::{debug, error, info, log, warn};
pub use uuid::Uuid;

const RUST_LOG: &str = "RUST_LOG";

pub fn init_logger() {
    if env::var(RUST_LOG).is_err() {
        #[cfg(debug_assertions)]
        env::set_var(RUST_LOG, "debug");
        #[cfg(not(debug_assertions))]
        env::set_var(RUST_LOG, "info");
    }

    log_builder()
        .default_format()
        .format_timestamp_nanos()
        .format_indent(Some(4))
        .init();
}
