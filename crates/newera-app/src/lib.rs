//! Desktop editor for 3D New Era AI.
//!
//! Layout follows classic home design tools: catalog and home contents on the
//! left, the 2D floor plan on top and the native 3D view below it.

mod ai;
#[cfg(target_arch = "wasm32")]
mod ai_web;
mod app;
mod cabinets;
mod dialogs;
mod ergonomics;
mod files;
#[cfg(not(target_arch = "wasm32"))]
mod gpu;
mod i18n;
mod jobs;
mod panels;
mod photo;
#[cfg(target_arch = "wasm32")]
mod recovery_web;
mod render_job;
#[cfg(target_arch = "wasm32")]
pub mod render_web;
#[cfg(test)]
mod screens;
mod tabs;
mod theme;
mod video;
mod view;

use std::path::PathBuf;

use newera_core::SharedDocument;

/// Options for [`run`].
#[derive(Debug, Clone, Default)]
pub struct AppOptions {
    /// MCP endpoint shown in the status bar, if the server is running.
    pub mcp_url: Option<String>,
    /// Project to open at startup.
    pub open: Option<PathBuf>,
}

/// Opens the editor window and blocks until it is closed.
#[cfg(not(target_arch = "wasm32"))]
pub fn run(document: SharedDocument, options: AppOptions) -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("3D New Era AI")
        .with_inner_size([1440.0, 920.0])
        .with_min_inner_size([960.0, 620.0]);
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(icon);
    }
    gpu::watch_panics();
    let native = eframe::NativeOptions {
        viewport,
        wgpu_options: wgpu_options(gpu::launch()),
        ..Default::default()
    };
    eframe::run_native(
        "3D New Era AI",
        native,
        Box::new(move |cc| {
            if let Some(state) = &cc.wgpu_render_state {
                gpu::watch_device(state);
            }
            let mut app = app::NewEraApp::new(cc, document, options.mcp_url);
            app.ask_support();
            if let Some(path) = options.open {
                app.open_path(&path);
            }
            Ok(Box::new(app))
        }),
    )
}

/// The default wgpu setup, minus the validation of indirect draws.
///
/// wgpu builds a compute pipeline for that check when the device is created,
/// and the Metal compiler of older Intel Macs (macOS 12) fails on it, which
/// loses the device and closes the editor at launch. Nothing here draws
/// indirectly, so the check guards nothing; `WGPU_VALIDATION_INDIRECT_CALL=1`
/// still turns it back on. The device also asks for the adapter's largest
/// buffer.
///
/// `backends`, when given, replaces wgpu's own list: see [`gpu`].
#[cfg(not(target_arch = "wasm32"))]
fn wgpu_options(backends: Option<eframe::wgpu::Backends>) -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::{egui_wgpu::WgpuSetup, wgpu::InstanceFlags};
    let mut options = eframe::egui_wgpu::WgpuConfiguration::default();
    if let WgpuSetup::CreateNew(setup) = &mut options.wgpu_setup {
        let flags = &mut setup.instance_descriptor.flags;
        *flags = flags
            .difference(InstanceFlags::VALIDATION_INDIRECT_CALL)
            .with_env();
        // The largest buffer this GPU takes rather than wgpu's portable
        // 256 MiB, so a big scene goes up in as few parts as it can.
        let base = std::sync::Arc::clone(&setup.device_descriptor);
        setup.device_descriptor = std::sync::Arc::new(move |adapter| {
            let mut descriptor = base(adapter);
            let limits = &mut descriptor.required_limits;
            limits.max_buffer_size = limits.max_buffer_size.max(adapter.limits().max_buffer_size);
            descriptor
        });
        if let Some(backends) = backends {
            setup.instance_descriptor.backends = backends;
        }
    }
    options
}

/// The app mark (`assets/icon.svg`), for the window, dock and taskbar.
#[cfg(not(target_arch = "wasm32"))]
fn app_icon() -> Option<std::sync::Arc<eframe::egui::IconData>> {
    let png = include_bytes!("../assets/icon-256.png");
    let image = image::load_from_memory(png).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(std::sync::Arc::new(eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }))
}

/// Starts the editor in a browser canvas, optionally with a project
/// (`.newera` JSON or bundle bytes) already open.
///
/// # Errors
/// When the canvas can't get a graphics context.
#[cfg(target_arch = "wasm32")]
pub async fn start_web(
    canvas: web_sys::HtmlCanvasElement,
    project: Option<(String, Vec<u8>)>,
    asked: bool,
) -> Result<(), wasm_bindgen::JsValue> {
    let document = SharedDocument::default();
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(move |cc| {
                if let Some(body) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.body())
                {
                    let backend = cc.wgpu_render_state.as_ref().map_or_else(
                        || "WebGL".to_owned(),
                        |state| format!("{:?}", state.adapter.get_info().backend),
                    );
                    let _ = body.set_attribute("data-editor-backend", &backend);
                }
                let mut app = app::NewEraApp::new(cc, document, None);
                app.ask_support();
                // The demo home is what a first visit opens on; someone who
                // was already drawing here gets their own work back instead —
                // unless the address asked for a project (a link from their
                // AI, "open in the editor"), which is what they came for.
                if let Some((name, bytes)) = project.filter(|_| asked || !app.restored) {
                    app.open_bytes(&name, &bytes);
                }
                Ok(Box::new(app))
            }),
        )
        .await
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use eframe::{egui_wgpu::WgpuSetup, wgpu::InstanceFlags};

    #[test]
    fn the_editor_does_not_validate_indirect_draws() {
        let WgpuSetup::CreateNew(setup) = super::wgpu_options(None).wgpu_setup else {
            panic!("the editor creates its own wgpu instance");
        };
        let flags = setup.instance_descriptor.flags;
        if std::env::var_os("WGPU_VALIDATION_INDIRECT_CALL").is_none() {
            assert!(!flags.contains(InstanceFlags::VALIDATION_INDIRECT_CALL));
        }
    }

    #[test]
    fn a_fallback_backend_replaces_wgpus_list() {
        let WgpuSetup::CreateNew(setup) =
            super::wgpu_options(Some(eframe::wgpu::Backends::GL)).wgpu_setup
        else {
            panic!("the editor creates its own wgpu instance");
        };
        assert_eq!(
            setup.instance_descriptor.backends,
            eframe::wgpu::Backends::GL
        );
    }
}
