use thiserror::Error;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("XDG base directory error: {0}")]
    Xdg(#[from] xdg::BaseDirectoriesError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}
