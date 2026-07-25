use smithay_client_toolkit::shm::CreatePoolError;
use thiserror::Error;
use wayland_client::{ConnectError, DispatchError, globals::GlobalError};

#[derive(Error, Debug)]
pub enum RenderError {
    /// Image decoding/processing failed.
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    /// Standard IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Wayland event dispatch failed (roundtrip / `blocking_dispatch`).
    #[error("Wayland dispatch error: {0}")]
    Dispatch(#[from] DispatchError),

    /// Failed to connect to the Wayland compositor.
    #[error("Wayland connect error: {0}")]
    Connect(#[from] ConnectError),

    /// Failed to bind a Wayland global (compositor, layer shell, shm).
    #[error("Wayland global error: {0}")]
    Global(#[from] GlobalError),

    /// Failed to create the shared memory pool.
    #[error("SHM pool error: {0}")]
    Pool(#[from] CreatePoolError),

    /// Any other Wayland/SCT protocol error that doesn't have its own type
    /// (e.g. `BindError` from `CompositorState::bind` / `LayerShell::bind`).
    #[error("Wayland error: {0}")]
    Wayland(String),

    #[error("Integer overflow in buffer size calculation")]
    Overflow,
}
