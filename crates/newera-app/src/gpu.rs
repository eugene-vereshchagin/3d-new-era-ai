//! A graphics backend that kills the editor at launch is not tried again.
//!
//! Some drivers fail before the window has drawn anything: out of memory
//! while the device is created, a device lost while the first frame is
//! uploaded. The editor closes, the person opens it again, and it closes again
//! the same way — the same machine reported thirteen of those in eight
//! minutes. Nothing the person can do from inside the program, since there is
//! no program to do it in.
//!
//! So a launch leaves a note saying it started, and the note is cleared once
//! the window has drawn a few frames. A note still there at the next launch
//! means the last one died on the way, and that launch asks wgpu for the next
//! backend this system has (on Windows Direct3D 12, then Vulkan, then OpenGL;
//! on Linux OpenGL), until one draws. A new version of the program starts
//! from the default again, and so does a `WGPU_BACKEND` set by hand, which
//! always wins.
//!
//! A GPU that is lost later, with the window already drawing, is reported as
//! such (`gpu_lost` on the crash report, the reason as a breadcrumb), but does
//! not change the backend: the window had drawn on it, and the cause can be
//! the scene as much as the driver.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use eframe::wgpu::Backends;
use serde::{Deserialize, Serialize};

/// Frames a launch has to draw before it counts as one that works.
const FRAMES_TO_TRUST: u32 = 3;

/// What the last launch left behind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Note {
    /// The version that wrote it; another version starts over.
    pub(crate) version: String,
    /// How far along the backends of this system the launches went.
    pub(crate) step: usize,
    /// Set at launch, cleared once the window draws.
    pub(crate) starting: bool,
}

/// The operating systems with backends to fall back on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum System {
    Windows,
    Linux,
    Other,
}

impl System {
    fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }

    /// The backends to try, in order; the first is wgpu's own choice.
    fn steps(self) -> &'static [Option<Backends>] {
        match self {
            Self::Windows => &[
                None,
                Some(Backends::DX12),
                Some(Backends::VULKAN),
                Some(Backends::GL),
            ],
            Self::Linux => &[None, Some(Backends::GL)],
            Self::Other => &[None],
        }
    }
}

/// The note for this launch, from the one the last launch left.
pub(crate) fn next(last: Option<Note>, version: &str, system: System) -> Note {
    let steps = system.steps().len();
    let step = match last {
        Some(last) if last.version == version && last.step < steps => {
            if last.starting {
                // Died before drawing: the next backend, and after the last
                // one the default again rather than none at all.
                (last.step + 1) % steps
            } else {
                last.step
            }
        }
        _ => 0,
    };
    Note {
        version: version.to_owned(),
        step,
        starting: true,
    }
}

/// The backends a note asks for; `None` leaves wgpu's choice.
pub(crate) fn backends(note: &Note, system: System) -> Option<Backends> {
    system.steps().get(note.step).copied().flatten()
}

/// Whether a panic message is the GPU giving up rather than the program.
pub(crate) fn is_gpu_panic(message: &str) -> bool {
    const SIGNS: [&str; 7] = [
        "Failed to create staging buffer",
        "Parent device is lost",
        "Device lost",
        "DeviceLost",
        "OutOfMemory",
        "HRESULT(0x80070008)",
        "HRESULT(0x887A0005)",
    ];
    SIGNS.iter().any(|sign| message.contains(sign))
}

static ARMED: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicU32 = AtomicU32::new(0);

fn note_path() -> Option<PathBuf> {
    eframe::storage_dir("3D New Era AI").map(|dir| dir.join("gpu-launch.json"))
}

fn write(note: &Note) {
    let Some(path) = note_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(note) {
        let _ = std::fs::write(path, json);
    }
}

/// Reads what the last launch left, writes this one's note and returns the
/// backends to ask for. Call once, before the window opens.
pub(crate) fn launch() -> Option<Backends> {
    if std::env::var_os("WGPU_BACKEND").is_some() {
        return None;
    }
    let system = System::current();
    let last = note_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|json| serde_json::from_str::<Note>(&json).ok());
    let retried = last.as_ref().is_some_and(|last| last.starting);
    let note = next(last, env!("CARGO_PKG_VERSION"), system);
    write(&note);
    ARMED.store(true, Ordering::Relaxed);
    let chosen = backends(&note, system);
    if retried {
        tracing::warn!("the last launch closed before drawing; graphics backends now {chosen:?}");
    }
    chosen
}

/// Counts a frame drawn; after a few, this launch works and its note is
/// cleared. Does nothing in a process that did not call [`launch`].
pub(crate) fn drew_frame() {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    if FRAMES.fetch_add(1, Ordering::Relaxed) == FRAMES_TO_TRUST - 1 {
        let Some(json) = note_path().and_then(|path| std::fs::read_to_string(path).ok()) else {
            return;
        };
        if let Ok(mut note) = serde_json::from_str::<Note>(&json) {
            note.starting = false;
            write(&note);
        }
    }
}

/// Marks a crash report as a lost GPU when it is one. Runs before the crash
/// reporter's own panic hook.
pub(crate) fn watch_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or_default();
        if is_gpu_panic(message) {
            newera_telemetry::set_tag("gpu_lost", "true");
        }
        previous(info);
    }));
}

/// Leaves a breadcrumb when the GPU device is lost, with wgpu's reason, and
/// tags reports with the backend in use.
pub(crate) fn watch_device(state: &eframe::egui_wgpu::RenderState) {
    let backend = format!("{:?}", state.adapter.get_info().backend);
    newera_telemetry::set_tag("gpu.backend", &backend);
    state.device.set_device_lost_callback(|reason, message| {
        if reason == eframe::wgpu::DeviceLostReason::Destroyed {
            tracing::debug!("GPU device closed: {message}");
        } else {
            tracing::warn!("GPU device lost ({reason:?}): {message}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(version: &str, step: usize, starting: bool) -> Note {
        Note {
            version: version.into(),
            step,
            starting,
        }
    }

    #[test]
    fn a_launch_that_died_before_drawing_moves_to_the_next_backend() {
        let first = next(None, "3.1.0", System::Windows);
        assert_eq!(first, note("3.1.0", 0, true));
        assert_eq!(backends(&first, System::Windows), None, "wgpu's choice");

        // Died at launch: Direct3D 12, then Vulkan, then OpenGL.
        let second = next(Some(first), "3.1.0", System::Windows);
        assert_eq!(backends(&second, System::Windows), Some(Backends::DX12));
        let third = next(Some(second), "3.1.0", System::Windows);
        assert_eq!(backends(&third, System::Windows), Some(Backends::VULKAN));
        let fourth = next(Some(third), "3.1.0", System::Windows);
        assert_eq!(backends(&fourth, System::Windows), Some(Backends::GL));
        // And after the last one, the default again.
        let fifth = next(Some(fourth), "3.1.0", System::Windows);
        assert_eq!(fifth.step, 0);
    }

    #[test]
    fn a_launch_that_drew_keeps_its_backend_until_a_new_version() {
        let drew = note("3.1.0", 1, false);
        let again = next(Some(drew.clone()), "3.1.0", System::Windows);
        assert_eq!(again, note("3.1.0", 1, true));
        assert_eq!(backends(&again, System::Windows), Some(Backends::DX12));
        assert_eq!(next(Some(drew), "3.2.0", System::Windows).step, 0);
    }

    #[test]
    fn linux_falls_back_to_opengl_and_other_systems_stay() {
        let died = note("3.1.0", 0, true);
        let linux = next(Some(died.clone()), "3.1.0", System::Linux);
        assert_eq!(backends(&linux, System::Linux), Some(Backends::GL));
        let mac = next(Some(died), "3.1.0", System::Other);
        assert_eq!(backends(&mac, System::Other), None);
        // A note from a system with more steps does not index past these.
        let stale = next(Some(note("3.1.0", 3, true)), "3.1.0", System::Linux);
        assert_eq!(stale.step, 0);
    }

    #[test]
    fn gpu_panics_are_told_from_the_rest() {
        assert!(is_gpu_panic(
            "Failed to create staging buffer for index data. Index count: 117147."
        ));
        assert!(is_gpu_panic(
            "called `Result::unwrap()` on an `Err` value: Error { code: HRESULT(0x80070008), message: \"Not enough memory resources are available to process this command.\" }"
        ));
        assert!(!is_gpu_panic("min > max, or either was NaN"));
    }
}
