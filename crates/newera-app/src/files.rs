//! Files on every platform: native dialogs and the disk on the desktop; the
//! browser's file picker and downloads on the web. Either way the answer
//! arrives a few frames later: through [`take_picked`] in the browser, through
//! an `Asked` on the desktop.

/// What a picked file is for.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickKind {
    Project,
    /// A scanned plan to trace over.
    Background,
}

/// A file chosen in the browser.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug)]
pub(crate) struct Picked {
    pub(crate) kind: PickKind,
    pub(crate) name: String,
    pub(crate) bytes: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop::{Answer, Asked, FileDialog, ask, dialog_open, open_dialog, save_dialog};

/// Native dialogs that stay open while the window keeps drawing.
///
/// A dialog that blocks until it is answered runs a loop of its own inside
/// the frame that opened it. On macOS that loop hands winit the events it had
/// put aside while the frame was being drawn, and winit, still inside that
/// frame, panics ("tried to handle event while another event is currently
/// being handled"). So the dialog is opened without waiting — a sheet on the
/// window on macOS, a thread of its own elsewhere — and its answer is picked
/// up by a later frame, as in the browser.
#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Wake, Waker};

    /// A dialog to open: where to read a file from, or where to write one.
    #[derive(Debug)]
    pub(crate) enum FileDialog {
        Open(rfd::AsyncFileDialog),
        Save(rfd::AsyncFileDialog),
    }

    /// A dialog that was asked and the answer it gave so far.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) enum Answer {
        Waiting,
        Chosen(PathBuf),
        Cancelled,
    }

    /// An open dialog. Its answer is taken once.
    #[derive(Debug, Clone)]
    pub(crate) struct Asked(Arc<Mutex<Answer>>);

    impl Asked {
        pub(crate) fn waiting() -> Self {
            Self(Arc::new(Mutex::new(Answer::Waiting)))
        }

        /// Gives the answer: what the dialog thread does when it closes.
        pub(crate) fn answer(&self, path: Option<PathBuf>) {
            if let Ok(mut slot) = self.0.lock() {
                *slot = path.map_or(Answer::Cancelled, Answer::Chosen);
            }
        }

        /// The answer once there is one; `Waiting` until then, and after it
        /// was taken.
        pub(crate) fn take(&self) -> Answer {
            let Ok(mut slot) = self.0.lock() else {
                return Answer::Cancelled;
            };
            if *slot == Answer::Waiting {
                Answer::Waiting
            } else {
                std::mem::replace(&mut *slot, Answer::Waiting)
            }
        }
    }

    /// The dialog for picking a file to read, with its filters
    /// (`(name, extensions)`).
    pub(crate) fn open_dialog(filters: &[(&str, &[&str])]) -> FileDialog {
        FileDialog::Open(
            filters
                .iter()
                .fold(rfd::AsyncFileDialog::new(), |dialog, (name, exts)| {
                    dialog.add_filter(*name, exts)
                }),
        )
    }

    /// The dialog for choosing where to save `name`.
    pub(crate) fn save_dialog(filter: &str, ext: &str, name: &str) -> FileDialog {
        FileDialog::Save(
            rfd::AsyncFileDialog::new()
                .add_filter(filter, &[ext])
                .set_file_name(name),
        )
    }

    /// How many dialogs are open: counted up by [`ask`], down when the
    /// dialog answers.
    static OPEN: AtomicUsize = AtomicUsize::new(0);

    /// Whether a dialog is open anywhere in the app. Only one is opened at a
    /// time, so a second request can't replace the first one's answer.
    pub(crate) fn dialog_open() -> bool {
        OPEN.load(Ordering::Acquire) > 0
    }

    /// Opens `dialog` without waiting for it. The answer shows up in the
    /// returned [`Asked`]; `ctx` is woken to read it.
    pub(crate) fn ask(dialog: FileDialog, ctx: Option<eframe::egui::Context>) -> Asked {
        // Made here, on the window's thread: on macOS this starts the sheet.
        let future: Pin<Box<dyn Future<Output = Option<rfd::FileHandle>> + Send>> = match dialog {
            FileDialog::Open(dialog) => Box::pin(dialog.pick_file()),
            FileDialog::Save(dialog) => Box::pin(dialog.save_file()),
        };
        wait_for(
            async move { future.await.map(|file| file.path().to_path_buf()) },
            ctx,
        )
    }

    /// Waits for a dialog's `answer` on a thread of its own; the dialog
    /// counts as open until it answers.
    fn wait_for(
        answer: impl Future<Output = Option<PathBuf>> + Send + 'static,
        ctx: Option<eframe::egui::Context>,
    ) -> Asked {
        let asked = Asked::waiting();
        let slot = asked.clone();
        OPEN.fetch_add(1, Ordering::AcqRel);
        std::thread::spawn(move || {
            slot.answer(block_on(answer));
            OPEN.fetch_sub(1, Ordering::AcqRel);
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
        asked
    }

    /// Wakes the thread that waits for the dialog.
    struct Unpark(std::thread::Thread);

    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    /// Waits for `future` on this thread.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        loop {
            if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
                return out;
            }
            std::thread::park();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_dialog_answer_is_waiting_until_given_and_taken_once() {
            let asked = Asked::waiting();
            assert_eq!(asked.take(), Answer::Waiting);
            // The dialog thread holds a clone.
            let thread = asked.clone();
            std::thread::spawn(move || thread.answer(Some(PathBuf::from("/casa.newera"))))
                .join()
                .unwrap();
            assert_eq!(asked.take(), Answer::Chosen(PathBuf::from("/casa.newera")));
            assert_eq!(asked.take(), Answer::Waiting, "taken once");
            let cancelled = Asked::waiting();
            cancelled.answer(None);
            assert_eq!(cancelled.take(), Answer::Cancelled);
        }

        /// Pending at first; another thread gives the value and wakes it.
        struct Later(Arc<Mutex<Option<u32>>>);

        impl Future for Later {
            type Output = u32;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
                if let Some(value) = *self.0.lock().unwrap() {
                    return Poll::Ready(value);
                }
                let (slot, waker) = (Arc::clone(&self.0), cx.waker().clone());
                std::thread::spawn(move || {
                    *slot.lock().unwrap() = Some(7);
                    waker.wake();
                });
                Poll::Pending
            }
        }

        #[test]
        fn a_future_is_waited_for_until_it_is_woken() {
            assert_eq!(block_on(Later(Arc::default())), 7);
        }

        /// A dialog that has not answered keeps every other one from
        /// opening, so no second request replaces its answer.
        #[test]
        fn a_dialog_is_open_until_it_answers() {
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            let asked = wait_for(
                async move {
                    // Answers once the test lets it; the wait stays on the
                    // dialog's thread.
                    rx.recv().ok().map(|()| PathBuf::from("/foto.png"))
                },
                None,
            );
            assert!(dialog_open(), "open while waiting");
            assert_eq!(asked.take(), Answer::Waiting);
            tx.send(()).unwrap();
            while asked.take() == Answer::Waiting {
                std::thread::yield_now();
            }
        }

        /// Every native dialog goes through [`ask`]: a dialog that waits for
        /// its answer inside a frame closes the editor on macOS.
        #[test]
        fn no_dialog_waits_inside_a_frame() {
            let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
            let mut stack = vec![src];
            let mut offenders = Vec::new();
            while let Some(dir) = stack.pop() {
                for entry in std::fs::read_dir(&dir).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.extension().is_some_and(|e| e == "rs") {
                        let text = std::fs::read_to_string(&path).unwrap();
                        let needle = concat!("rfd::", "FileDialog");
                        if text.contains(needle) || text.contains(concat!("rfd::", "MessageDialog"))
                        {
                            offenders.push(path.display().to_string());
                        }
                    }
                }
            }
            assert!(offenders.is_empty(), "{offenders:?}");
        }
    }
}

/// Downloads `bytes` made by `make` as `name`. Returns the name.
///
/// # Errors
/// When `make` fails or the browser refuses the download.
#[cfg(target_arch = "wasm32")]
pub(crate) fn save_bytes(
    filter: &str,
    ext: &str,
    name: &str,
    make: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<Option<String>, String> {
    let _ = (filter, ext);
    web::download(name, &make()?)?;
    Ok(Some(name.to_owned()))
}

#[cfg(target_arch = "wasm32")]
pub(crate) use web::{pick, take_picked};

#[cfg(target_arch = "wasm32")]
mod web {
    use std::cell::RefCell;

    use wasm_bindgen::JsCast;

    use super::{PickKind, Picked};

    thread_local! {
        static PICKED: RefCell<Vec<Picked>> = const { RefCell::new(Vec::new()) };
    }

    /// Opens the browser's file picker; the file shows up in [`take_picked`].
    pub(crate) fn pick(kind: PickKind, ctx: &eframe::egui::Context) {
        let dialog = match kind {
            PickKind::Project => rfd::AsyncFileDialog::new().add_filter(
                "3D New Era AI / Sweet Home 3D",
                &[newera_core::PROJECT_EXTENSION, "sh3d"],
            ),
            PickKind::Background => rfd::AsyncFileDialog::new().add_filter(
                "PNG, JPEG, WebP, BMP",
                &["png", "jpg", "jpeg", "webp", "bmp"],
            ),
        };
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(file) = dialog.pick_file().await {
                let picked = Picked {
                    kind,
                    name: file.file_name(),
                    bytes: file.read().await,
                };
                PICKED.with(|p| p.borrow_mut().push(picked));
                ctx.request_repaint();
            }
        });
    }

    /// Files picked since the last call.
    pub(crate) fn take_picked() -> Vec<Picked> {
        PICKED.with(|p| std::mem::take(&mut *p.borrow_mut()))
    }

    /// Hands `bytes` to the browser as a download named `name`.
    pub(crate) fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
        let fail = |e: wasm_bindgen::JsValue| format!("{e:?}");
        let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
        let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).map_err(fail)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(fail)?;
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or("no document")?;
        let anchor: web_sys::HtmlAnchorElement = document
            .create_element("a")
            .map_err(fail)?
            .dyn_into()
            .map_err(|_| "not an anchor")?;
        anchor.set_href(&url);
        anchor.set_download(name);
        anchor.click();
        web_sys::Url::revoke_object_url(&url).map_err(fail)
    }
}
