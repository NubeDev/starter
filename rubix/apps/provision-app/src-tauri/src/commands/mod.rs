//! Tauri command surface — one verb per file. Barrel re-exports each
//! `#[command]` for the invoke_handler in `lib.rs`.

pub mod auth_login;
pub mod auth_logout;
pub mod auth_me;
pub mod ping;
pub mod queue_drop;
pub mod queue_enqueue;
pub mod queue_flush;
pub mod queue_list;
pub mod saved_base_url;
pub mod tool_dispatch;
