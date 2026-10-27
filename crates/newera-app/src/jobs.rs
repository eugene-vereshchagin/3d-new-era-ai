//! Work that takes long enough to be noticed, done off the drawing thread.
//!
//! Opening a home with a hundred textures, saving a bundle, exporting a model:
//! seconds of work that used to happen inside a click. The window stopped
//! painting, the system called it unresponsive, and someone with no patience
//! clicked again — on nothing, since those clicks piled up behind the work.
//!
//! So the work moves to a thread of its own and the window keeps painting: it
//! says what is happening, how far along it is when that can be known, how
//! long it has been going, and offers a way out. Only one job runs at a time
//! and while it does, the rest of the interface is out of reach — a home being
//! read is not a home to be edited — but the window is alive and answers.
//!
//! Giving up is two steps, because not all work can stop where it is: asking
//! sets a flag the work looks at between one file and the next, and if it does
//! not stop, leaving anyway hands it over to the background, where it finishes
//! into a result nobody reads.
//!
//! In the browser there are no threads here: the job runs in place, as the
//! file operations still do; image/video renders have their own isolated worker.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use web_time::Instant;

use crate::app::NewEraApp;

/// What a finished job does back on the drawing thread: put the home it read
/// into the document, say what was saved, report what went wrong.
pub(crate) type Finish = Box<dyn FnOnce(&mut NewEraApp) + Send>;

/// What the work reports while it runs, and whether it was asked to stop.
#[derive(Default)]
struct Reported {
    /// What it is doing now, in the words the work itself used.
    what: Mutex<String>,
    done: AtomicU64,
    of: AtomicU64,
    give_up: AtomicBool,
}

impl newera_core::progress::Watcher for Reported {
    fn step(&self, what: &str, done: u64, of: u64) {
        if let Ok(mut current) = self.what.lock()
            && *current != what
        {
            what.clone_into(&mut current);
        }
        self.done.store(done, Ordering::Relaxed);
        self.of.store(of, Ordering::Relaxed);
    }

    fn cancelled(&self) -> bool {
        self.give_up.load(Ordering::Relaxed)
    }
}

/// A piece of work in flight.
pub(crate) struct Job {
    /// What is being done, e.g. "Abrindo projeto".
    title: String,
    /// What it is being done to: a file name, a format.
    detail: String,
    started: Instant,
    reported: Arc<Reported>,
    result: Arc<Mutex<Option<Finish>>>,
    /// When someone asked it to stop, and it has not stopped yet.
    asked_to_stop: Option<Instant>,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("title", &self.title)
            .field("detail", &self.detail)
            .finish_non_exhaustive()
    }
}

impl Job {
    /// How far along, when the work knows: `(done, of)`.
    fn progress(&self) -> (u64, u64) {
        (
            self.reported.done.load(Ordering::Relaxed),
            self.reported.of.load(Ordering::Relaxed),
        )
    }

    fn what(&self) -> String {
        self.reported
            .what
            .lock()
            .map(|w| w.clone())
            .unwrap_or_default()
    }
}

/// Starts `work` in the background and shows a progress window until it ends.
///
/// `work` runs away from the document's write lock; whatever it returns is run
/// on the drawing thread, where touching the app is safe. A job started while
/// another one runs waits for nothing — there is only ever one, and the
/// interface behind the window cannot start a second.
pub(crate) fn start<W>(app: &mut NewEraApp, title: &str, detail: String, work: W)
where
    W: FnOnce() -> Finish + Send + 'static,
{
    let reported = Arc::new(Reported::default());
    let result = Arc::new(Mutex::new(None));
    let job = Job {
        title: title.to_owned(),
        detail,
        started: Instant::now(),
        reported: Arc::clone(&reported),
        result: Arc::clone(&result),
        asked_to_stop: None,
    };
    let run = move || {
        let watcher: Arc<dyn newera_core::progress::Watcher> = reported;
        let finish = newera_core::progress::watched(&watcher, work);
        if let Ok(mut slot) = result.lock() {
            *slot = Some(finish);
        }
    };
    // Browsers have no threads here: the page waits, as it does for a photo.
    #[cfg(target_arch = "wasm32")]
    {
        run();
        app.job = Some(job);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(run);
        app.job = Some(job);
    }
}

/// Whether a job is running, and so nothing else should be started.
pub(crate) fn busy(app: &NewEraApp) -> bool {
    app.job.is_some()
}

/// Shows the progress window while a job runs, and applies its result.
pub(crate) fn show(app: &mut NewEraApp, ctx: &egui::Context) {
    let Some(mut job) = app.job.take() else {
        return;
    };
    // Finished: back on this thread, the result is free to touch the app.
    let finished = job.result.lock().ok().and_then(|mut slot| slot.take());
    if let Some(finish) = finished {
        finish(app);
        return;
    }

    // The bar has to move while nothing else asks for a frame.
    ctx.request_repaint_after(Duration::from_millis(80));

    let (done, of) = job.progress();
    let what = job.what();
    let elapsed = job.started.elapsed();
    let mut leave = false;
    crate::theme::modal(ctx, egui::Id::unique("job")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading(&job.title);
        if !job.detail.is_empty() {
            ui.label(egui::RichText::new(&job.detail).weak());
        }
        ui.add_space(8.0);
        if of > 0 {
            #[allow(clippy::cast_precision_loss)]
            let share = (done as f32 / of as f32).clamp(0.0, 1.0);
            ui.add(
                egui::ProgressBar::new(share)
                    .animate(true)
                    .text(format!("{:.0}%", share * 100.0)),
            );
        } else {
            // Nobody knows how much there is: a moving bar would be a lie, so
            // it shows what is happening and that it is still happening.
            ui.horizontal(|ui| {
                ui.spinner();
                if done > 0 {
                    ui.label(format!("{done}"));
                }
            });
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let step = if what.is_empty() {
                crate::i18n::tr("Trabalhando…").to_owned()
            } else {
                crate::i18n::dynamic(&what)
            };
            ui.label(step);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} s", elapsed.as_secs()))
                        .weak()
                        .monospace(),
                );
            });
        });
        ui.add_space(8.0);
        match job.asked_to_stop {
            None => {
                if ui.button(crate::i18n::tr("Cancelar")).clicked() {
                    job.reported.give_up.store(true, Ordering::Relaxed);
                    job.asked_to_stop = Some(Instant::now());
                }
            }
            Some(asked) => {
                ui.label(crate::i18n::tr("Parando…"));
                // Work that cannot stop where it is would keep the window
                // waiting forever; after a moment, leaving is the way out.
                if asked.elapsed() > Duration::from_secs(2) {
                    ui.add_space(4.0);
                    if ui
                        .button(crate::i18n::tr("Forçar e encerrar a espera"))
                        .on_hover_text(crate::i18n::tr(
                            "A tarefa termina sozinha em segundo plano; o resultado é descartado.",
                        ))
                        .clicked()
                    {
                        leave = true;
                    }
                }
            }
        }
    });

    if leave {
        app.set_status(crate::i18n::tr(
            "⚠ Espera encerrada — a tarefa termina em segundo plano.",
        ));
        return; // the job is dropped: whatever it returns goes nowhere
    }
    app.job = Some(job);
}

#[cfg(test)]
mod tests {
    use newera_core::progress::Watcher;

    use super::*;

    #[test]
    fn what_the_work_reports_is_what_the_window_reads() {
        let reported = Reported::default();
        reported.step("Guardando imagens e modelos", 3, 40);
        assert_eq!(reported.done.load(Ordering::Relaxed), 3);
        assert_eq!(reported.of.load(Ordering::Relaxed), 40);
        assert_eq!(
            *reported.what.lock().expect("what"),
            "Guardando imagens e modelos"
        );
        assert!(!reported.cancelled());
        reported.give_up.store(true, Ordering::Relaxed);
        assert!(reported.cancelled());
    }
}
