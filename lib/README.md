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
randpaper_lib = { git = "https://github.com/saylesss88/randpaper" }
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
) -> Result<(), RenderError>
```

| Parameter     | Description                                                                                        |
| :------------ | :------------------------------------------------------------------------------------------------- |
| `image_path`  | Path to the image file. Supported formats: JPEG, PNG, BMP, WebP.                                   |
| `output_name` | Connector name to target (e.g. `"eDP-1"`, `"HDMI-A-1"`). Pass `None` to let the compositor choose. |

Blocks indefinitely in the Wayland event loop — intended to be run in a
dedicated thread or process per output.

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

## Limitations

- **No transitions or animations**: pixel-perfect static rendering only.
- **Single surface per call**: call once per output, each in its own thread.

## License

[Apache License 2.0](https://github.com/saylesss88/randpaper/blob/main/LICENSE)
