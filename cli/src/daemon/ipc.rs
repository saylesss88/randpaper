use crate::daemon_lock::session_key;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use xdg::{BaseDirectories, BaseDirectoriesError};

#[derive(Debug)]
pub enum DaemonCommand {
    Next,
    Pause,
    Resume,
    Status(tokio::sync::oneshot::Sender<String>),
}

pub struct DaemonState {
    pub paused: bool,
}

pub fn find_socket() -> Result<PathBuf, BaseDirectoriesError> {
    let xdg_dirs = BaseDirectories::with_prefix("randpaper");
    Ok(xdg_dirs
        .get_runtime_directory()?
        .join("randpaper")
        .join(format!("randpaper-{}.sock", session_key())))
}

pub async fn listen_for_ipc(tx: mpsc::Sender<DaemonCommand>) -> anyhow::Result<()> {
    let socket_path = find_socket()?;
    let _ = std::fs::remove_file(&socket_path); // clean up stale socket
    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            continue;
        }
        let cmd = std::str::from_utf8(&buf[..n])?.trim();
        match cmd {
            "next" => {
                let _ = tx.send(DaemonCommand::Next).await;
            }
            "pause" => {
                let _ = tx.send(DaemonCommand::Pause).await;
            }
            "resume" => {
                let _ = tx.send(DaemonCommand::Resume).await;
            }
            "status" => {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                let _ = tx.send(DaemonCommand::Status(reply_tx)).await;
                if let Ok(reply) = reply_rx.await {
                    stream.write_all(reply.as_bytes()).await?;
                }
            }
            other => {
                log::warn!("Unknown IPC command: {other}");
            }
        }
    }
}
