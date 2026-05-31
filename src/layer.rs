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

/// Render a single wallpaper image on one Wayland output and block until the
/// connection is closed.
///
/// * `image_path` – path to the image file (JPEG, PNG, BMP, WebP supported)
/// * `output_name` – the connector name e.g. `"eDP-1"`.  Pass `None` to let
///   the compositor pick (good for single-monitor setups).
///
/// # Errors
/// Returns an error if the Wayland connection fails, the image cannot be
/// decoded, or a required protocol is unavailable.
pub fn render_wallpaper(image_path: &Path, output_name: Option<&str>) -> anyhow::Result<()> {
    // ── Wayland bootstrap ────────────────────────────────────────────────────
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let layer_shell = LayerShell::bind(&globals, &qh).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| anyhow::anyhow!("{e:?}"))?;

    // We need to do one roundtrip so that OutputState can populate itself
    // before we try to match an output by name.
    let output_state = OutputState::new(&globals, &qh);
    event_queue.roundtrip(&mut WarmupState {
        registry_state: RegistryState::new(&globals),
        output_state,
    })?;

    // ── Resolve the wl_output (if a name was requested) ──────────────────────
    // We re-init properly below; this is just to read names.
    let output_state_for_lookup = OutputState::new(&globals, &qh);

    // Do another roundtrip to populate
    let mut warmup = WarmupState {
        registry_state: RegistryState::new(&globals),
        output_state: output_state_for_lookup,
    };
    event_queue.roundtrip(&mut warmup)?;

    let wl_output: Option<wl_output::WlOutput> = output_name.and_then(|name| {
        warmup
            .output_state
            .outputs()
            .find(|o| warmup.output_state.info(o).and_then(|i| i.name).as_deref() == Some(name))
            .cloned()
    });

    // ── Decode image (before entering the event loop) ────────────────────────
    let img: DynamicImage = image::open(image_path)?;

    // ── Create the layer surface ──────────────────────────────────────────────
    let surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Background,
        Some("randpaper"),
        wl_output.as_ref(),
    );

    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.set_size(0, 0);
    layer.commit();

    // Initial pool – resized in draw() once we know the real dimensions.
    let pool = SlotPool::new(1920 * 1080 * 4, &shm)?;

    let mut state = WallpaperState {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        shm,
        first_configure: true,
        pool,
        width: 1920,
        height: 1080,
        layer,
        image: img,
    };

    // ── Event loop ────────────────────────────────────────────────────────────
    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}

// ── Minimal state used only during the warmup roundtrip ──────────────────────

struct WarmupState {
    registry_state: RegistryState,
    output_state: OutputState,
}

impl OutputHandler for WarmupState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ProvidesRegistryState for WarmupState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_registry!(WarmupState);
smithay_client_toolkit::delegate_output!(WarmupState);

// ── Main renderer state ───────────────────────────────────────────────────────

struct WallpaperState {
    registry_state: RegistryState,
    output_state: OutputState,
    shm: Shm,
    first_configure: bool,
    pool: SlotPool,
    width: u32,
    height: u32,
    layer: LayerSurface,
    /// The source image, held so we can rescale on configure.
    image: DynamicImage,
}

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
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // Static wallpaper – no continuous redraws needed.
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
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
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
            self.draw(qh);
        }
    }
}

impl ShmHandler for WallpaperState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl WallpaperState {
    fn draw(&mut self, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = width as i32 * 4;

        // Resize the pool if necessary (e.g. after a monitor reconfigure).
        let needed = (stride as usize) * height as usize;
        if self.pool.len() < needed {
            self.pool.resize(needed).expect("pool resize failed");
        }

        let (buffer, canvas) = self
            .pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Xrgb8888,
            )
            .expect("create buffer");

        // ── Scale the image to fill the surface, then blit ───────────────────
        // `Cover` scaling: fill the whole area, cropping if aspect ratios differ.
        let scaled = self
            .image
            .resize_to_fill(width, height, FilterType::Lanczos3)
            .into_rgba8();

        // Wayland wl_shm with Xrgb8888 is stored as [B, G, R, X] (little-endian 32-bit).
        for (dst, src) in canvas.chunks_exact_mut(4).zip(scaled.pixels()) {
            let [r, g, b, _a] = src.0;
            dst[0] = b;
            dst[1] = g;
            dst[2] = r;
            dst[3] = 0xFF;
        }

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        buffer
            .attach_to(self.layer.wl_surface())
            .expect("buffer attach");
        self.layer.wl_surface().commit();
    }
}

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
