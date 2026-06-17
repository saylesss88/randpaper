# randpaper_ipc

Unix socket IPC library for the [randpaper](https://github.com/saylesss88/randpaper) wallpaper daemon.

## Overview

`randpaper_ipc` handles the server-side Unix socket listener and shared types used for communication between the `randpaper` daemon and CLI. It is a workspace member of the `randpaper` project alongside the `cli` crate.

## What it provides

- **`listen_for_ipc`**: async Unix socket listener that accepts connections and forwards parsed commands to the daemon via an `mpsc` channel
- **`find_socket`**: resolves the XDG runtime socket path for a given session key (`$XDG_RUNTIME_DIR/randpaper/randpaper-{key}.sock`)
- **`DaemonCommand`**: enum of commands the daemon understands: `Next`, `Pause`, `Resume`, `Status`
- **`DaemonState`**: shared state struct (currently tracks `paused`)
- **`IpcError`**: typed error enum via `thiserror` covering XDG, IO, and UTF-8 failures

## Socket path

Sockets are placed under `$XDG_RUNTIME_DIR/randpaper/` and keyed by compositor session (e.g. `randpaper-sway.sock`, `randpaper-hypr-<sig>.sock`). The session key is derived from environment variables at runtime (`SWAYSOCK`, `HYPRLAND_INSTANCE_SIGNATURE`, `WAYLAND_DISPLAY`).

## Usage

In the daemon:

```rust
let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<DaemonCommand>(8);
let key = session_key();
tokio::spawn(async move {
    if let Err(e) = randpaper_ipc::listen_for_ipc(cmd_tx, &key).await {
        log::error!("IPC listener exited: {e:#}");
    }
});
```

In the CLI client:

```rust
let socket_path = randpaper_ipc::find_socket(&session_key())?;
let mut stream = UnixStream::connect(&socket_path).await?;
stream.write_all(b"next").await?;
```

## Supported commands

| Command  | Description                        |
|----------|------------------------------------|
| `next`   | Cycle to the next wallpaper        |
| `pause`  | Pause automatic cycling            |
| `resume` | Resume automatic cycling           |
| `status` | Returns current daemon state       |

## Sway

Since `exec`'s run every time Sway restarts/reloads, a stale daemon from the
previous session can linger.

Adding the following `exec` fixed this issue:

```sh
exec pkill -x randpaper; randpaper --daemon
```

I haven't had a chance to test hyprland, but am thinking its `exec-once` will
prevent this from happening. Mango should have similar results to hyprland.
