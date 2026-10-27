//! "Criar vídeo": edits the camera path of the home (keyframes taken from the
//! visitor or an aerial orbit) and renders it to a Motion-JPEG AVI in a
//! background thread.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::time::Duration;

use web_time::Instant;

use eframe::egui::{self, RichText};
use egui_phosphor::regular as icon;
use newera_core::Command;

use crate::app::NewEraApp;
use crate::i18n::tr;

type Outcome = Result<newera_render::video::VideoInfo, String>;

/// A running video render.
struct Job {
    started: Instant,
    outcome: Arc<Mutex<Option<Outcome>>>,
    file: PathBuf,
    report: Arc<crate::render_job::Report>,
    #[cfg(target_arch = "wasm32")]
    web: Option<(
        crate::render_web::Worker,
        crate::render_job::ByteOutcome,
        usize,
        u32,
    )>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.report.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Settings and progress of the video window.
pub(crate) struct VideoWindow {
    pub(crate) size: [u32; 2],
    job: Option<Job>,
    pub(crate) last: Option<(PathBuf, Outcome)>,
}

impl std::fmt::Debug for VideoWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoWindow")
            .field("size", &self.size)
            .field("running", &self.job.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for VideoWindow {
    fn default() -> Self {
        Self {
            size: [640, 360],
            job: None,
            last: None,
        }
    }
}

impl VideoWindow {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn start(&mut self, app: &NewEraApp, file: PathBuf) {
        if let Err(error) = self.begin(app, file.clone()) {
            self.last = Some((file, Err(error)));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn begin(&mut self, app: &NewEraApp, file: PathBuf) -> Result<(), String> {
        let permit = crate::render_job::Permit::acquire()?;
        let (home, assets) = {
            let doc = app.document.read();
            (doc.home().clone(), doc.asset_dir())
        };
        let outcome = Arc::new(Mutex::new(None));
        let (o, f) = (Arc::clone(&outcome), file.clone());
        let size = (self.size[0], self.size[1]);
        newera_render::video::validate_video(
            &home.environment.camera_path,
            home.environment.video.frame_rate,
            home.environment.video.speed,
            size,
        )?;
        let report = Arc::new(crate::render_job::Report::default());
        let watcher: Arc<dyn newera_core::progress::Watcher> = report.clone();
        std::thread::spawn(move || {
            let _permit = permit;
            let video = &home.environment.video;
            let result = newera_core::progress::watched(&watcher, || {
                newera_render::video::render_video(
                    &home,
                    &home.environment.camera_path,
                    video.frame_rate,
                    video.speed,
                    size,
                    assets.as_deref(),
                    &f,
                    |_, _| {},
                )
            });
            if let Ok(mut guard) = o.lock() {
                *guard = Some(result);
            }
        });
        self.job = Some(Job {
            started: Instant::now(),
            outcome,
            file,
            report,
        });
        self.last = None;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn start_web(&mut self, app: &NewEraApp, ctx: &egui::Context) -> Result<(), String> {
        let doc = app.document.read();
        let home = doc.home();
        let video = &home.environment.video;
        newera_render::video::validate_video(
            &home.environment.camera_path,
            video.frame_rate,
            video.speed,
            (self.size[0], self.size[1]),
        )?;
        let frames = newera_render::video::frame_count(
            &home.environment.camera_path,
            video.frame_rate,
            video.speed,
        );
        let report = Arc::new(crate::render_job::Report::default());
        let bytes = Arc::new(Mutex::new(None));
        let worker = crate::render_web::Worker::start(
            serde_json::json!({
                "kind":"video", "home":home, "assets":doc.asset_dir(), "size":self.size,
            }),
            home.asset_paths(),
            report.clone(),
            bytes.clone(),
            ctx.clone(),
        )?;
        self.job = Some(Job {
            started: Instant::now(),
            outcome: Arc::new(Mutex::new(None)),
            file: "video.avi".into(),
            report,
            web: Some((worker, bytes, frames, video.frame_rate)),
        });
        self.last = None;
        Ok(())
    }

    pub(crate) fn running(&self) -> bool {
        self.job.is_some()
    }
}

fn set_environment(app: &NewEraApp, edit: impl FnOnce(&mut newera_core::Environment)) {
    let mut doc = app.document.write();
    let mut environment = doc.home().environment.clone();
    edit(&mut environment);
    if environment != doc.home().environment {
        let _ = doc.execute(Command::SetEnvironment { environment });
    }
}

/// Shows the video window while it is open.
#[allow(clippy::too_many_lines)]
pub(crate) fn show(app: &mut NewEraApp, ctx: &egui::Context) {
    let Some(mut window) = app.video.take() else {
        return;
    };
    let mut open = true;
    #[cfg(target_arch = "wasm32")]
    let mut render_to = None;
    #[cfg(not(target_arch = "wasm32"))]
    let mut ask_where = false;
    let mut cancel = false;
    let environment = app.document.read().home().environment.clone();
    let (mut fps, mut speed) = (environment.video.frame_rate, environment.video.speed);
    let keys = environment.camera_path.len();
    // Fewer than two points cannot generate a clip. Otherwise show encoded
    // duration, including rounding and the renderer's two-frame minimum.
    #[allow(clippy::cast_precision_loss)]
    let seconds = if keys < 2 {
        0.0
    } else {
        newera_render::video::frame_count(&environment.camera_path, fps, speed) as f64
            / f64::from(fps.max(1))
    };
    egui::Window::new(format!("{} {}", icon::FILM_STRIP, tr("Criar vídeo")))
        .open(&mut open)
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label(format!(
                "{}: {keys} · {seconds:.2} s",
                tr("Pontos do caminho")
            ));
            ui.horizontal_wrapped(|ui| {
                let visitor = app.scene.visitor.as_ref().map(|v| v.camera.clone());
                if ui
                    .add_enabled(
                        visitor.is_some(),
                        egui::Button::new(format!(
                            "{} {}",
                            icon::PLUS,
                            tr("Adicionar ponto atual")
                        )),
                    )
                    .on_disabled_hover_text(tr("Ative o visitante na vista 3D"))
                    .clicked()
                    && let Some(camera) = visitor
                {
                    set_environment(app, |e| e.camera_path.push(camera));
                }
                if ui
                    .button(format!("{} {}", icon::ARROWS_CLOCKWISE, tr("Órbita aérea")))
                    .clicked()
                {
                    let orbit =
                        newera_render::video::orbit_path(app.document.read().home(), 800.0, 8);
                    set_environment(app, |e| e.camera_path = orbit);
                }
                if ui
                    .add_enabled(
                        keys > 0,
                        egui::Button::new(format!("{} {}", icon::TRASH, tr("Limpar"))),
                    )
                    .clicked()
                {
                    set_environment(app, |e| e.camera_path.clear());
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Quadros por segundo"));
                ui.add(egui::Slider::new(&mut fps, 5..=60));
            });
            ui.horizontal(|ui| {
                ui.label(tr("Velocidade"));
                ui.add(
                    egui::Slider::new(&mut speed, 0.2..=20.0)
                        .logarithmic(true)
                        .suffix(" m/s"),
                );
            });
            ui.horizontal(|ui| {
                ui.label(tr("Tamanho"));
                for size in [[320, 240], [640, 360], [1280, 720], [1920, 1080]] {
                    #[cfg(target_arch = "wasm32")]
                    let enabled = u64::from(size[0]) * u64::from(size[1])
                        <= crate::render_web::budget().pixels;
                    #[cfg(not(target_arch = "wasm32"))]
                    let enabled = true;
                    ui.add_enabled_ui(enabled, |ui| {
                        ui.selectable_value(
                            &mut window.size,
                            size,
                            format!("{}×{}", size[0], size[1]),
                        );
                    });
                }
            });
            #[cfg(target_arch = "wasm32")]
            {
                ui.weak(format!(
                    "Uma tarefa por vez · até 900 quadros · 32 MB · {} pixels neste aparelho",
                    crate::render_web::budget().pixels
                ));
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        keys >= 2 && !window.running(),
                        egui::Button::new(format!("{} {}", icon::FILM_SLATE, tr("Gerar vídeo…"))),
                    )
                    .clicked()
                {
                    #[cfg(target_arch = "wasm32")]
                    {
                        render_to = Some(PathBuf::from("video.avi"));
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ask_where = true;
                    }
                }
                if let Some(job) = &window.job {
                    cancel = job.report.ui(ui);
                }
            });
            match &window.last {
                Some((file, Ok(info))) => {
                    ui.label(
                        RichText::new(format!(
                            "{} {} · {} {} · {:.1} s",
                            tr("Vídeo salvo em"),
                            file.display(),
                            info.frames,
                            tr("quadros"),
                            info.seconds
                        ))
                        .weak(),
                    );
                }
                Some((_, Err(err))) => {
                    ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {err}"));
                }
                None => {}
            }
        });

    if fps != environment.video.frame_rate || (speed - environment.video.speed).abs() > f64::EPSILON
    {
        set_environment(app, |e| {
            e.video.frame_rate = fps;
            e.video.speed = speed;
        });
    }
    if cancel {
        window.job = None;
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(job) = &window.job
        && let Some((_, bytes, frames, fps)) = &job.web
        && let Some(result) = bytes.lock().ok().and_then(|mut g| g.take())
    {
        let result = result.and_then(|bytes| {
            #[allow(clippy::cast_precision_loss)]
            let info = newera_render::video::VideoInfo {
                frames: *frames,
                seconds: *frames as f64 / f64::from(*fps),
                bytes: bytes.len(),
            };
            crate::files::save_bytes("AVI", "avi", "video.avi", || Ok(bytes))?;
            Ok(info)
        });
        *job.outcome.lock().expect("video outcome") = Some(result);
    }
    let finished = window.job.as_ref().and_then(|job| {
        job.outcome
            .lock()
            .ok()
            .and_then(|mut g| g.take())
            .map(|outcome| (job.file.clone(), job.started, outcome))
    });
    if let Some((file, started, outcome)) = finished {
        if outcome.is_ok() {
            app.set_status(format!(
                "{} {} ({:.0} s)",
                tr("Vídeo salvo em"),
                file.display(),
                started.elapsed().as_secs_f32()
            ));
        }
        window.last = Some((file, outcome));
        window.job = None;
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(file) = render_to
        && let Err(error) = window.start_web(app, ctx)
    {
        window.last = Some((file, Err(error)));
    }
    if open {
        app.video = Some(window);
    }
    // Rendered once the dialog answers, a few frames later, if the window
    // is still open.
    #[cfg(not(target_arch = "wasm32"))]
    if ask_where {
        let dialog = crate::files::save_dialog("AVI", "avi", "video.avi");
        app.ask_file(dialog, |app, path| {
            if let Some(path) = path
                && let Some(mut window) = app.video.take()
            {
                if !window.running() {
                    window.start(app, path);
                }
                app.video = Some(window);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use newera_core::{Document, Point2, SharedDocument, Wall};

    use super::*;

    #[test]
    fn orbit_button_fills_the_path_and_the_video_renders() {
        let mut doc = Document::default();
        let wall = Wall::new(
            doc.new_wall_id(),
            Point2::new(0.0, 0.0),
            Point2::new(300.0, 0.0),
        );
        doc.execute(Command::insert(wall)).unwrap();
        let document = SharedDocument::new(doc);
        let shared = document.clone();
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .with_step_dt(1.0 / 60.0)
            .build_eframe(move |cc| NewEraApp::new(cc, document, None));
        h.run_steps(3);
        h.state_mut().video = Some(VideoWindow {
            size: [64, 48],
            ..VideoWindow::default()
        });
        h.run_steps(3);
        h.get_by_label_contains("Órbita aérea").click();
        h.run_steps(2);
        assert_eq!(shared.read().home().environment.camera_path.len(), 9);
        // Fast tour so the test renders few frames.
        set_environment(h.state(), |e| {
            e.video.speed = 50.0;
            e.video.frame_rate = 2;
        });
        let dir = std::env::temp_dir().join(format!("newera-app-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("tour.avi");
        let mut window = h.state_mut().video.take().unwrap();
        window.start(h.state(), file.clone());
        h.state_mut().video = Some(window);
        for _ in 0..1500 {
            h.run_steps(1);
            if h.state().video.as_ref().unwrap().last.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let (_, outcome) = h
            .state()
            .video
            .as_ref()
            .unwrap()
            .last
            .as_ref()
            .expect("video finished");
        assert!(outcome.as_ref().unwrap().frames >= 2);
        assert!(std::fs::read(&file).unwrap().starts_with(b"RIFF"));
        std::fs::remove_dir_all(dir).ok();
    }
}
