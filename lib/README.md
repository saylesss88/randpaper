# randpaper_lib

A minimal Rust library for rendering a wallpaper image onto a Wayland output
using the native `zwlr_layer_shell_v1` protocol. No external tools required, no
`swaybg`, no `swww`.

## Overview

`randpaper_lib` handles all the low-level Wayland plumbing:

- Connects to the Wayland compositor via the environment socket
- Binds `wl_compositor`, `zwlr_layer_shell_v1`, and `wl_shm`
- Decodes the image (JPEG, PNG, BMP, WebP) and scales it to fill the output
  (cover mode)
- Creates a background-layer surface, blits pixels via shared memory, and blocks
  in the event loop

The public API is a single function. You hand it a path and an optional output
name, it does the rest.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
randpaper_lib = "0.2.0"
```

```rust
use randpaper_lib::layer::render_wallpaper;
use std::path::Path;

fn main() -> Result<(), randpaper_lib::errors::RenderError> {
    // Render on a specific monitor
    render_wallpaper(Path::new("/path/to/image.jpg"), Some("eDP-1"))?;

    // Or let the compositor choose (single-monitor / fallback)
    render_wallpaper(Path::new("/path/to/image.png"), None)?;

    Ok(())
}
```

## API

### `render_wallpaper`

```rust
pub fn render_wallpaper(
    image_path: &Path,
    output_name: Option<&str>,
    stop: &Arc<AtomicBool>,
) -> Result<(), RenderError>
```

| Parameter     | Description                                                                                        |
| :------------ | :------------------------------------------------------------------------------------------------- |
| `image_path`  | Path to the image file. Supported formats: JPEG, PNG, BMP, WebP.                                   |
| `output_name` | Connector name to target (e.g. `"eDP-1"`, `"HDMI-A-1"`). Pass `None` to let the compositor choose. |
| `stop`        | Shared flag. Set to `true` to cleanly exit the event loop on the next dispatch cycle.              |

Blocks in the Wayland event loop until `stop` is set to `true` or a dispatch
error occurs.

## Stopping the renderer

`render_wallpaper` accepts an `Arc<AtomicBool>` stop flag. Set it to `true` from
any thread to cleanly unblock the call on the next event loop iteration.

```rust
use randpaper_lib::layer::render_wallpaper;
use std::{
    path::Path,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    thread,
    time::Duration,
};

fn main() -> Result<(), randpaper_lib::errors::RenderError> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_tx = Arc::clone(&stop);

    // Stop after 30 seconds
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(30));
        stop_tx.store(true, Ordering::Relaxed);
    });

    render_wallpaper(Path::new("/path/to/image.jpg"), Some("eDP-1"), stop)?;
    Ok(())
}
```

The flag is checked at the top of every event loop iteration. Stop latency is
bounded by the poll timeout (100ms by default).

### `RenderError`

```rust
pub enum RenderError {
    Image(image::ImageError),      // image decode/processing failed
    Io(std::io::Error),            // shared memory pool I/O
    Dispatch(DispatchError),       // Wayland event dispatch failed
    Connect(ConnectError),         // could not connect to compositor
    Global(GlobalError),           // required Wayland global not available
    Pool(CreatePoolError),         // shared memory pool creation failed
    Overflow,                      // buffer size calculation overflowed
    Wayland(String),               // other protocol-level error (bind, attach)
}
```

## Requirements

- A running Wayland compositor that supports `zwlr_layer_shell_v1` (Sway,
  Hyprland, MangoWM, and most wlroots-based compositors)
- `WAYLAND_DISPLAY` set in the environment

## Dependencies

| Crate                    | Purpose                                              |
| :----------------------- | :--------------------------------------------------- |
| `smithay-client-toolkit` | Layer shell, compositor, output, and SHM bindings    |
| `wayland-client`         | Core Wayland protocol connection and dispatch        |
| `image`                  | Image decoding and scaling (cover mode via Lanczos3) |
| `thiserror`              | Typed error enum                                     |
| `rustix`                 | Poll timeout                                         |

## Real-world usage

`randpaper_lib` is used as the rendering backend for
[persway-tokio](https://github.com/saylesss88/persway), a Sway/Wayland window
management daemon. The integration drives per-output wallpaper setting via IPC:
each output gets its own `render_wallpaper` call in a dedicated
`spawn_blocking` thread, with the stop flag wired to a `WallpaperHandle` that
the daemon holds in a `HashMap<String, WallpaperHandle>` keyed by output name
(e.g. `"eDP-1"`, `"HDMI-A-1"`). Replacing a wallpaper on one monitor stops
only that output's handle, leaving others untouched.

### Non-blocking event loop

The event loop uses a poll-based design rather than `blocking_dispatch`, so the
stop flag is checked reliably even when the compositor sends no events (which is
the common case for a static wallpaper):

```rust
loop {
    if stop.load(Ordering::Relaxed) {
        return Ok(());
    }
    event_queue.flush()?;
    if let Some(guard) = event_queue.prepare_read() {
        // Poll the Wayland fd with a 100ms timeout instead of blocking forever
        rustix::event::poll(&mut [PollFd::new(&guard.connection_fd(), PollFlags::IN)],
            Some(&Timespec { tv_sec: 0, tv_nsec: 100_000_000 }))?;
        let _ = guard.read();
    }
    event_queue.dispatch_pending(&mut state)?;
}
```

This means `WallpaperHandle::stop()` returns within ~100ms regardless of
compositor activity, making it safe to call from async context without stalling
the tokio runtime.

## Limitations

- **No transitions or animations**: pixel-perfect static rendering only.
- **Single surface per call**: call once per output, each in its own thread.

## License

[Apache License 2.0](https://github.com/saylesss88/randpaper/blob/main/LICENSE)
