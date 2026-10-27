//! "Criar foto": realistic renders of the current 3D point of view, made in
//! a background thread or browser worker so the editor stays responsive.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use web_time::Instant;

use eframe::egui::{self, RichText};
use egui_phosphor::regular as icon;

use crate::app::NewEraApp;

struct Job {
    started: Instant,
    size: [u32; 2],
    report: Arc<crate::render_job::Report>,
    result: crate::render_job::ByteOutcome,
    #[cfg(target_arch = "wasm32")]
    _worker: crate::render_web::Worker,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.report
            .cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Settings and progress of the photo window.
pub(crate) struct PhotoWindow {
    pub(crate) quality: newera_render::PhotoQuality,
    /// Local solar hour.
    pub(crate) hour: f64,
    pub(crate) size: [u32; 2],
    job: Option<Job>,
    error: Option<String>,
    result: Option<(image::RgbaImage, egui::TextureHandle, Duration)>,
}

impl std::fmt::Debug for PhotoWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhotoWindow")
            .field("hour", &self.hour)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl Default for PhotoWindow {
    fn default() -> Self {
        Self {
            quality: newera_render::PhotoQuality::Draft,
            hour: 10.0,
            size: [800, 600],
            job: None,
            error: None,
            result: None,
        }
    }
}

impl PhotoWindow {
    fn start(&mut self, app: &NewEraApp, ctx: &egui::Context) {
        if let Err(error) = self.begin(app, ctx) {
            self.error = Some(error);
        }
    }
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    fn begin(&mut self, app: &NewEraApp, ctx: &egui::Context) -> Result<(), String> {
        if self.job.is_some() {
            return Err("Renderização em andamento".into());
        }
        #[cfg(target_arch = "wasm32")]
        let max = crate::render_web::budget().pixels;
        #[cfg(not(target_arch = "wasm32"))]
        let max = 2_073_600;
        if self.size.contains(&0) || u64::from(self.size[0]) * u64::from(self.size[1]) > max {
            return Err("Reduza a resolução para respeitar o limite de memória.".into());
        }
        let (home, assets) = {
            let doc = app.document.read();
            (doc.home().clone(), doc.asset_dir())
        };
        let view = app.scene.current_view();
        let [w, h] = self.size;
        #[allow(clippy::cast_precision_loss)]
        let view = match &app.scene.visitor {
            Some(visitor) => newera_render::View::from_camera(&visitor.camera, w as f32 / h as f32),
            None => view,
        };
        let base = app
            .scene
            .visitor
            .as_ref()
            .map_or(home.cameras.top.time, |v| v.camera.time);
        let base = if base == 0 { 1_789_214_400_000 } else { base };
        let time =
            newera_render::at_local_hour(base, self.hour, home.compass.longitude.unwrap_or(-46.63));
        let quality = self.quality;
        let report = Arc::new(crate::render_job::Report::default());
        let slot = Arc::new(Mutex::new(None));
        #[cfg(target_arch = "wasm32")]
        let worker = crate::render_web::Worker::start(
            serde_json::json!({
                "kind":"photo", "home":home, "assets":assets, "size":[w,h],
                "eye":view.eye.to_array(), "target":view.target.to_array(), "fov":view.fov_y,
                "ortho":view.ortho, "near":view.near, "time":time, "quality":format!("{quality:?}")
            }),
            home.asset_paths(),
            report.clone(),
            slot.clone(),
            ctx.clone(),
        )?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let permit = crate::render_job::Permit::acquire()?;
            let out = slot.clone();
            let watcher: Arc<dyn newera_core::progress::Watcher> = report.clone();
            std::thread::spawn(move || {
                let _permit = permit;
                let image = newera_core::progress::watched(&watcher, || {
                    newera_render::photo_home(&home, &view, time, w, h, assets.as_deref(), quality)
                });
                if let Ok(mut guard) = out.lock() {
                    *guard = Some(Ok(image.into_raw()));
                }
            });
        }
        self.job = Some(Job {
            started: Instant::now(),
            size: [w, h],
            report,
            result: slot,
            #[cfg(target_arch = "wasm32")]
            _worker: worker,
        });
        self.result = None;
        self.error = None;
        Ok(())
    }
}

/// Shows the photo window while it is open.
pub(crate) fn show(app: &mut NewEraApp, ctx: &egui::Context) {
    let Some(mut window) = app.photo.take() else {
        return;
    };
    let mut open = true;
    let mut start = false;
    let mut save = false;
    let mut cancel = false;
    egui::Window::new(format!(
        "{} {}",
        icon::CAMERA,
        crate::i18n::tr("Criar foto")
    ))
    .open(&mut open)
    .default_width(560.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(crate::i18n::tr("Qualidade"));
            for (quality, name) in [
                (
                    newera_render::PhotoQuality::Draft,
                    crate::i18n::tr("Rascunho"),
                ),
                (newera_render::PhotoQuality::Good, crate::i18n::tr("Boa")),
                (newera_render::PhotoQuality::Best, crate::i18n::tr("Máxima")),
            ] {
                ui.selectable_value(&mut window.quality, quality, name);
            }
        });
        ui.horizontal(|ui| {
            ui.label(crate::i18n::tr("Hora do dia"));
            ui.add(
                egui::Slider::new(&mut window.hour, 0.0..=24.0)
                    .step_by(0.25)
                    .suffix(" h"),
            );
        });
        ui.horizontal(|ui| {
            ui.label(crate::i18n::tr("Tamanho"));
            for size in [[640, 480], [800, 600], [1280, 960], [1920, 1080]] {
                #[cfg(target_arch = "wasm32")]
                let enabled =
                    u64::from(size[0]) * u64::from(size[1]) <= crate::render_web::budget().pixels;
                #[cfg(not(target_arch = "wasm32"))]
                let enabled = true;
                ui.add_enabled_ui(enabled, |ui| {
                    ui.selectable_value(&mut window.size, size, format!("{}×{}", size[0], size[1]));
                });
            }
        });
        ui.weak(crate::i18n::tr(
            "Usa o ponto de vista atual da vista 3D (aérea ou visitante).",
        ));
        ui.separator();
        let running = window.job.is_some();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !running,
                    egui::Button::new(format!(
                        "{} {}",
                        icon::APERTURE,
                        crate::i18n::tr("Renderizar")
                    )),
                )
                .clicked()
            {
                start = true;
            }
            if ui
                .add_enabled(
                    window.result.is_some(),
                    egui::Button::new(format!(
                        "{} {}",
                        icon::FLOPPY_DISK,
                        crate::i18n::tr("Salvar PNG…")
                    )),
                )
                .clicked()
            {
                save = true;
            }
            if let Some(job) = &window.job {
                cancel = job.report.ui(ui);
            }
            if let Some((_, _, took)) = &window.result {
                let took = format!("{:.1}", took.as_secs_f32());
                ui.label(RichText::new(crate::i18n::fill("Pronta em {} s", &[&took])).weak());
            }
        });
        if let Some(error) = &window.error {
            ui.colored_label(ui.visuals().warn_fg_color, error);
        }
        #[cfg(target_arch = "wasm32")]
        {
            ui.weak(format!(
                "Uma tarefa por vez · até {} pixels neste aparelho",
                crate::render_web::budget().pixels
            ));
        }
        if let Some((_, texture, _)) = &window.result {
            let width = ui.available_width();
            let size = texture.size_vec2();
            ui.image((
                texture.id(),
                egui::vec2(width, width * size.y / size.x.max(1.0)),
            ));
        }
    });

    if cancel {
        window.job = None;
    }
    let finished = window.job.as_ref().and_then(|job| {
        job.result
            .lock()
            .ok()
            .and_then(|mut g| g.take())
            .map(|result| (job.started, job.size, result))
    });
    if let Some((started, [w, h], result)) = finished {
        match result.and_then(|bytes| {
            image::RgbaImage::from_raw(w, h, bytes).ok_or_else(|| "Imagem inválida".into())
        }) {
            Ok(image) => {
                let size = [image.width() as usize, image.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                let texture = ctx.load_texture("newera-photo", color, egui::TextureOptions::LINEAR);
                window.result = Some((image, texture, started.elapsed()));
            }
            Err(error) => window.error = Some(error),
        }
        window.job = None;
    }
    if start {
        window.start(app, ctx);
    }
    if save && let Some((image, _, _)) = &window.result {
        let png = || {
            let mut bytes = Vec::new();
            image
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(bytes)
        };
        #[cfg(target_arch = "wasm32")]
        match crate::files::save_bytes("PNG", "png", "foto.png", png) {
            Ok(Some(path)) => app.set_status(crate::i18n::fill("Foto salva em {}", &[&path])),
            Ok(None) => {}
            Err(err) => {
                app.set_status(crate::i18n::fill("⚠ Não foi possível salvar: {}", &[&err]));
            }
        }
        // Written once the dialog answers, a few frames later.
        #[cfg(not(target_arch = "wasm32"))]
        match png() {
            Ok(bytes) => {
                let dialog = crate::files::save_dialog("PNG", "png", "foto.png");
                app.ask_file(dialog, move |app, path| {
                    let Some(path) = path else {
                        return;
                    };
                    match std::fs::write(&path, bytes) {
                        Ok(()) => app
                            .set_status(crate::i18n::fill("Foto salva em {}", &[&path.display()])),
                        Err(err) => app.set_status(crate::i18n::fill(
                            "⚠ Não foi possível salvar: {}",
                            &[&err],
                        )),
                    }
                });
            }
            Err(err) => {
                app.set_status(crate::i18n::fill("⚠ Não foi possível salvar: {}", &[&err]));
            }
        }
    }
    if open {
        app.photo = Some(window);
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use newera_core::{Command, Document, Point2, SharedDocument, Wall};

    use super::*;

    #[test]
    fn closing_or_cancelling_a_photo_releases_the_render_budget() {
        let document = SharedDocument::new(Document::default());
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .with_step_dt(1.0 / 60.0)
            .build_eframe(move |cc| NewEraApp::new(cc, document, None));
        h.run_steps(3);
        h.state_mut().photo = Some(PhotoWindow {
            size: [1280, 960],
            quality: newera_render::PhotoQuality::Best,
            ..PhotoWindow::default()
        });
        h.run_steps(3);
        h.get_by_label_contains("Renderizar").click();
        h.run_steps(2);
        h.get_by_label("Cancelar").click();
        h.run_steps(2);
        assert!(h.state().photo.as_ref().unwrap().job.is_none());
        for _ in 0..200 {
            if crate::render_job::Permit::acquire().is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("cancelled photo kept using the render budget");
    }

    #[test]
    fn renders_a_photo_in_the_background_and_shows_it() {
        let mut doc = Document::default();
        let wall = Wall::new(
            doc.new_wall_id(),
            Point2::new(0.0, 0.0),
            Point2::new(300.0, 0.0),
        );
        doc.execute(Command::insert(wall)).unwrap();
        let document = SharedDocument::new(doc);
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .with_step_dt(1.0 / 60.0)
            .build_eframe(move |cc| NewEraApp::new(cc, document, None));
        h.run_steps(3);
        h.state_mut().photo = Some(PhotoWindow {
            size: [64, 48],
            ..PhotoWindow::default()
        });
        h.run_steps(3);
        h.get_by_label_contains("Renderizar").click();
        h.run_steps(2);
        assert!(
            h.state().photo.as_ref().unwrap().job.is_some(),
            "job started"
        );
        for _ in 0..600 {
            h.run_steps(1);
            if h.state().photo.as_ref().unwrap().result.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let window = h.state().photo.as_ref().unwrap();
        let (image, _, _) = window.result.as_ref().expect("photo finished");
        assert_eq!(image.dimensions(), (64, 48));
    }
}
