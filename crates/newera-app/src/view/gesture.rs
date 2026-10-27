//! What the hands did this frame.
//!
//! A mouse and a trackpad send the same kind of event and mean opposite
//! things by it: a wheel clicks in *lines* and asks to zoom, two fingers
//! glide in *points* and ask to move the view. The machine tells them apart
//! by the unit it reports, so we read that and hand the rest of the window a
//! gesture instead of a scroll — pinch, glide, twist, notch.

#![allow(clippy::float_cmp)] // egui reports "no pinch" as exactly 1.0.

use eframe::egui::{self, MouseWheelUnit, Vec2};

/// One frame of hands on the input device, already smoothed by egui.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Gesture {
    /// Two fingers gliding on the trackpad, in points. Follows the fingers:
    /// the same direction the content should go.
    pub(crate) glide: Vec2,
    /// A pinch, or a wheel with the zoom key held: a factor around 1.0,
    /// above it when the fingers spread. Read it with [`Gesture::pinch`].
    zoom: f32,
    /// Wheel notches, in points, positive when pushed away.
    pub(crate) wheel: f32,
    /// Two fingers twisting, in radians, positive counter-clockwise.
    pub(crate) twist: f32,
}

impl Gesture {
    /// Reads this frame's gestures. Nothing here is per-widget: ask only when
    /// the pointer is over the view that should answer.
    pub(crate) fn read(ui: &egui::Ui) -> Self {
        let from_trackpad = last_scroll_was_a_glide(ui);
        ui.input(|i| {
            let scroll = i.smooth_scroll_delta;
            Self {
                glide: if from_trackpad { scroll } else { Vec2::ZERO },
                zoom: i.zoom_delta(),
                wheel: if from_trackpad { 0.0 } else { scroll.y },
                twist: i.rotation_delta(),
            }
        })
    }

    /// How much the fingers spread, when they did: a factor above 1.0 to
    /// come closer, below it to pull away.
    pub(crate) fn pinch(self) -> Option<f32> {
        (self.zoom != 1.0).then_some(self.zoom)
    }

    /// Whether anything at all happened, so a view can skip the work.
    pub(crate) fn is_idle(self) -> bool {
        self.glide == Vec2::ZERO && self.pinch().is_none() && self.wheel == 0.0 && self.twist == 0.0
    }
}

/// Which device the scroll on screen is coming from. Momentum keeps arriving
/// after the fingers lift, and smoothing keeps moving after that, both with
/// no new event to read — so the answer is remembered until a new wheel event
/// says otherwise.
fn last_scroll_was_a_glide(ui: &egui::Ui) -> bool {
    let id = egui::Id::unique("scroll-device-is-a-trackpad");
    let fresh = ui.input(|i| {
        i.raw.events.iter().rev().find_map(|event| match event {
            egui::Event::MouseWheel { unit, .. } => Some(*unit == MouseWheelUnit::Point),
            _ => None,
        })
    });
    match fresh {
        Some(glide) => {
            ui.data_mut(|d| d.insert_temp(id, glide));
            glide
        }
        None => ui.data(|d| d.get_temp(id).unwrap_or(false)),
    }
}
