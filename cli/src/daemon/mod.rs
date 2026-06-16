pub mod event_loop;
pub mod ipc;
mod render;

// Re-exports
pub use event_loop::run_loop;
pub use render::awww::detect_awww_binary;
pub use render::awww::ensure_awww_daemon;
