use crate::errors::RenderError;
use image::{DynamicImage, imageops::FilterType};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use std::{num::NonZeroU32, path::Path};
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
};

/// Render a wallpaper image on a Wayland output and block until the
/// connection is closed.
///
/// * `image_path`   – path to the image (JPEG, PNG, BMP, WebP)
/// * `output_name`  – connector name e.g. `"eDP-1"`. `None` lets the
///   compositor choose (good for single-monitor setups).
///
/// # Errors
/// Returns an error if the Wayland connection fails, the image cannot be
/// decoded, or a required protocol is unavailable.
pub fn render_wallpaper(image_path: &Path, output_name: Option<&str>) -> Result<(), RenderError> {
    let conn = Connection::connect_to_env()?;
    // registry_queue_init is generic over the dispatch state — Rust infers
    // WallpaperState here because every .bind() / OutputState::new call below
    // constrains the queue handle type to WallpaperState.
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    // Bind the three globals we need. All delegate macros for these are on
    // WallpaperState, so the qh type is correctly inferred.
    let compositor =
        CompositorState::bind(&globals, &qh).map_err(|e| RenderError::Wayland(format!("{e:?}")))?;
    let layer_shell =
        LayerShell::bind(&globals, &qh).map_err(|e| RenderError::Wayland(format!("{e:?}")))?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| RenderError::Wayland(format!("{e:?}")))?;

    // Decode the image before entering the event loop.
    let image = image::open(image_path)?;

    let pool = SlotPool::new(1920 * 1080 * 4, &shm)?;

    let mut state = WallpaperState {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        compositor,
        layer_shell,
        shm,
        pool,
        layer: None,
        image,
        width: 1920,
        height: 1080,
        first_configure: true,
        target_output: output_name.map(str::to_owned),
    };

    // One roundtrip so that OutputState is populated and new_output() fires
    // for every currently-connected monitor. If output_name matched, state.layer
    // is now Some. If no name was requested, we create a surface with output=None.
    event_queue.roundtrip(&mut state)?;

    if state.layer.is_none() {
        // Either no output_name was given, or the named output wasn't found yet —
        // fall back to compositor-chosen output.
        state.create_layer_surface(&qh, None);
    }

    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}

// ─── Main renderer state ──────────────────────────────────────────────────────

struct WallpaperState {
    registry_state: RegistryState,
    output_state: OutputState,
    // Globals stored so we can create surfaces after the initial roundtrip.
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    pool: SlotPool,
    layer: Option<LayerSurface>,
    image: DynamicImage,
    width: u32,
    height: u32,
    first_configure: bool,
    /// Connector name we want to render on, e.g. "eDP-1".
    target_output: Option<String>,
}

impl WallpaperState {
    /// Create and configure the layer surface, optionally pinned to a specific output.
    fn create_layer_surface(
        &mut self,
        qh: &QueueHandle<Self>,
        output: Option<&wl_output::WlOutput>,
    ) {
        let surface = self.compositor.create_surface(qh);
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Background,
            Some("randpaper"),
            output,
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(0, 0); // fill the output
        layer.commit();
        self.layer = Some(layer);
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) -> Result<(), Option<RenderError>> {
        let Some(ref layer) = self.layer else {
            return Ok(());
        };

        let width = self.width;
        let height = self.height;
        let stride = width.cast_signed() * 4;

        let needed = (stride.cast_unsigned() as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| String::from("Calculation overflowed standard memory limits"))?;

        if self.pool.len() < needed {
            self.pool.resize(needed)?;
        }

        let (buffer, canvas) = self
            .pool
            .create_buffer(
                width.cast_signed(),
                height.cast_signed(),
                stride,
                wl_shm::Format::Xrgb8888,
            )
            .expect("create buffer");

        // Scale to fill (cover mode), then blit RGBA → BGRX.
        let scaled = self
            .image
            .resize_to_fill(width, height, FilterType::Lanczos3)
            .into_rgba8();

        // wl_shm Xrgb8888 is little-endian: bytes are [B, G, R, X].
        for (dst, src) in canvas.chunks_exact_mut(4).zip(scaled.pixels()) {
            let [r, g, b, _a] = src.0;
            dst[0] = b;
            dst[1] = g;
            dst[2] = r;
            dst[3] = 0xFF;
        }

        layer
            .wl_surface()
            .damage_buffer(0, 0, width.cast_signed(), height.cast_signed());
        buffer.attach_to(layer.wl_surface()).expect("buffer attach");
        layer.wl_surface().commit();

        // Request a callback for the next frame so Wayland doesn't starve.
        layer.wl_surface().frame(qh, layer.wl_surface().clone());

        // Finish the function successfully
        Ok(())
    }
}

// ─── Handler impls ────────────────────────────────────────────────────────────

impl CompositorHandler for WallpaperState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // Static wallpaper: only redraw if something changed (e.g. resize).
        // Keeping this as a no-op avoids burning CPU on every vsync.
        let _ = qh;
    }
    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for WallpaperState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        // If a specific output was requested and this is it, pin the surface to it.
        if let Some(ref target) = self.target_output.clone()
            && let Some(info) = self.output_state.info(&output)
            && info.name.as_deref() == Some(target.as_str())
        {
            self.create_layer_surface(qh, Some(&output));
        }
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for WallpaperState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.width = NonZeroU32::new(configure.new_size.0).map_or(1920, NonZeroU32::get);
        self.height = NonZeroU32::new(configure.new_size.1).map_or(1080, NonZeroU32::get);
        if self.first_configure {
            self.first_configure = false;
            if let Err(e) = self.draw(qh) {
                eprintln!("Failed to draw wallpeper: {e}");
            }
        }
    }
}

impl ShmHandler for WallpaperState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

// ─── Delegation macros ────────────────────────────────────────────────────────

delegate_registry!(WallpaperState);

impl ProvidesRegistryState for WallpaperState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

smithay_client_toolkit::delegate_compositor!(WallpaperState);
smithay_client_toolkit::delegate_output!(WallpaperState);
smithay_client_toolkit::delegate_layer!(WallpaperState);
smithay_client_toolkit::delegate_shm!(WallpaperState);
