use fslock::LockFile;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn sanitize_component(s: &str) -> String {
    // Keep it ASCII and filesystem-friendly.
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let ok = ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_');
        out.push(if ok { ch } else { '_' });
    }
    out
}

fn truncate_ascii(mut s: String, max_len: usize) -> String {
    // After sanitize_component(), it’s all ASCII, so char count == byte count.
    if s.len() > max_len {
        s.truncate(max_len);
    }
    s
}

fn sway_sock_basename(sock: &str) -> &str {
    Path::new(sock)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("sway")
}

fn session_key() -> String {
    if let Ok(sig) = env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        return truncate_ascii(format!("hypr-{}", sanitize_component(&sig)), 80);
    }

    if let Ok(sock) = env::var("SWAYSOCK") {
        let base = sway_sock_basename(&sock);
        return truncate_ascii(format!("sway-{}", sanitize_component(base)), 80);
    }

    if let Ok(disp) = env::var("WAYLAND_DISPLAY") {
        return truncate_ascii(format!("wayland-{}", sanitize_component(&disp)), 80);
    }

    "unknown".to_string()
}

fn lock_path() -> anyhow::Result<PathBuf> {
    // Best for “per login session” behavior (Wayland compositors normally set this). [web:30]
    let runtime =
        env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| anyhow::anyhow!("XDG_RUNTIME_DIR not set"))?;
    let runtime = PathBuf::from(runtime);

    Ok(runtime
        .join("randpaper")
        .join(format!("randpaper-{}.lock", session_key())))
}

/// Returns Some(lock) if we are the daemon; None if another daemon for this session is running.
pub fn single_instance_guard() -> anyhow::Result<Option<LockFile>> {
    let path = lock_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let mut lock = LockFile::open(&path)?;
    if lock.try_lock_with_pid()? {
        Ok(Some(lock))
    } else {
        Ok(None)
    }
}
