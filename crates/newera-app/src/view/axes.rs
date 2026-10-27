//! Axes floating in a corner of the 3D view, the way Blender draws them: one
//! ball per side of the house, turning with the camera. A click on a ball
//! looks at the house from that side; a drag on the disc orbits.

use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use glam::Vec3;
use newera_render::Side;

/// Radius of the disc, in points.
const RADIUS: f32 = 42.0;
/// Radius of each ball, in points.
const BALL: f32 = 9.0;
/// Gap between the disc and the corner of the view, in points.
const MARGIN: f32 = 10.0;

const RED: Color32 = Color32::from_rgb(230, 70, 85);
const GREEN: Color32 = Color32::from_rgb(120, 190, 30);
const BLUE: Color32 = Color32::from_rgb(50, 135, 235);

/// The balls, back first. There is no ball under the house: the aerial
/// camera stays above the ground.
const SIDES: [Side; 5] = [Side::Right, Side::Left, Side::Front, Side::Back, Side::Top];

/// Where the eye sits, seen from the target, when it looks from `side`. World
/// axes: x is the plan's x, y is up and z is the plan's y.
pub(crate) fn toward(side: Side) -> Vec3 {
    match side {
        Side::Right => Vec3::X,
        Side::Left => Vec3::NEG_X,
        Side::Front => Vec3::Z,
        Side::Back => Vec3::NEG_Z,
        Side::Top => Vec3::Y,
    }
}

/// The axis a ball lies on, named after the plan's (z up), and whether it
/// is the positive end, which carries the letter.
fn axis(side: Side) -> (&'static str, Color32, bool) {
    match side {
        Side::Right => ("X", RED, true),
        Side::Left => ("X", RED, false),
        Side::Front => ("Y", GREEN, true),
        Side::Back => ("Y", GREEN, false),
        Side::Top => ("Z", BLUE, true),
    }
}

fn name(side: Side) -> &'static str {
    crate::i18n::tr(match side {
        Side::Right => "Direita",
        Side::Left => "Esquerda",
        Side::Front => "Frente",
        Side::Back => "Trás",
        Side::Top => "Topo",
    })
}

/// How the camera is turned: its right, its up and the way back towards the
/// eye, in world axes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Basis {
    pub(crate) right: Vec3,
    pub(crate) up: Vec3,
    pub(crate) back: Vec3,
}

/// What the hands did to the axes this frame.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Outcome {
    /// The ball clicked: look from that side.
    pub(crate) look_from: Option<Side>,
    /// A drag on the disc, in points, to orbit by.
    pub(crate) drag: Vec2,
    /// The ball under the pointer, to light it up.
    pub(crate) hovered: Option<Side>,
    /// Whether the pointer is over the disc at all.
    pub(crate) over: bool,
}

/// The disc's square, in the top right corner of `view`.
fn disc(view: Rect) -> Rect {
    Rect::from_center_size(
        view.right_top() + egui::vec2(-MARGIN - RADIUS, MARGIN + RADIUS),
        Vec2::splat(RADIUS * 2.0),
    )
}

/// Each ball's center on screen and how near it is to the viewer (1 facing
/// it, -1 behind), back first so the front ones paint over.
fn balls(basis: Basis, center: Pos2) -> Vec<(Side, Pos2, f32)> {
    let reach = RADIUS - BALL - 2.0;
    let mut balls: Vec<_> = SIDES
        .iter()
        .map(|&side| {
            let a = toward(side);
            let at = center + egui::vec2(a.dot(basis.right), -a.dot(basis.up)) * reach;
            (side, at, a.dot(basis.back))
        })
        .collect();
    balls.sort_by(|a, b| a.2.total_cmp(&b.2));
    balls
}

/// The ball under `pointer`: the one nearest the viewer when two overlap.
fn under(basis: Basis, center: Pos2, pointer: Pos2) -> Option<Side> {
    balls(basis, center)
        .into_iter()
        .rev()
        .find(|(_, at, _)| at.distance(pointer) <= BALL + 1.0)
        .map(|(side, ..)| side)
}

/// Reads the hands on the disc. Call it after the view took its own
/// interaction, so the disc sits on top of it.
pub(crate) fn interact(ui: &egui::Ui, view: Rect, basis: Basis) -> Outcome {
    let rect = disc(view);
    let response = ui.interact(
        rect,
        ui.scope_id().with("3d-view-axes"),
        egui::Sense::click_and_drag(),
    );
    let pointer = response.hover_pos().or(response.interact_pointer_pos());
    let over = pointer.is_some_and(|p| p.distance(rect.center()) <= RADIUS) || response.dragged();
    let hovered = pointer
        .filter(|_| !response.dragged())
        .and_then(|p| under(basis, rect.center(), p));
    let outcome = Outcome {
        look_from: hovered.filter(|_| response.clicked()),
        drag: if response.dragged() {
            response.drag_delta()
        } else {
            Vec2::ZERO
        },
        hovered,
        over,
    };
    if let Some(side) = hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        response.on_hover_text(name(side));
    } else if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    outcome
}

/// Paints the disc over the view.
pub(crate) fn paint(ui: &egui::Ui, view: Rect, basis: Basis, outcome: Outcome) {
    let painter = ui.painter_at(view);
    let center = disc(view).center();
    if outcome.over {
        painter.circle_filled(center, RADIUS, Color32::from_black_alpha(28));
    }
    for (side, at, near) in balls(basis, center) {
        let (letter, color, positive) = axis(side);
        // The far half of the axes fades a little, so the depth reads.
        let color = color.gamma_multiply(0.75 + 0.25 * near.clamp(-1.0, 1.0));
        if positive {
            painter.line_segment([center, at], egui::Stroke::new(2.0, color));
            painter.circle_filled(at, BALL, color);
            painter.text(
                at,
                egui::Align2::CENTER_CENTER,
                letter,
                egui::FontId::proportional(11.0),
                Color32::from_black_alpha(200),
            );
        } else {
            painter.circle(
                at,
                BALL - 1.0,
                color.gamma_multiply(0.45),
                egui::Stroke::new(1.5, color),
            );
        }
        if outcome.hovered == Some(side) {
            painter.circle_stroke(at, BALL + 1.5, egui::Stroke::new(2.0, Color32::WHITE));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Looking from the front: x to the right, up is up, the eye on +z.
    const FRONT: Basis = Basis {
        right: Vec3::X,
        up: Vec3::Y,
        back: Vec3::Z,
    };

    #[test]
    fn balls_follow_the_camera() {
        let center = Pos2::new(100.0, 100.0);
        let at = |side| {
            balls(FRONT, center)
                .into_iter()
                .find(|b| b.0 == side)
                .unwrap()
                .1
        };
        assert!(at(Side::Right).x > center.x);
        assert!(at(Side::Left).x < center.x);
        assert!(at(Side::Top).y < center.y);
        // Front and back both sit on the center, the front one drawn last.
        assert!(at(Side::Front).distance(center) < 1e-3);
        assert_eq!(balls(FRONT, center).last().unwrap().0, Side::Front);
    }

    #[test]
    fn the_nearest_ball_wins_the_click() {
        let center = Pos2::new(100.0, 100.0);
        assert_eq!(under(FRONT, center, center), Some(Side::Front));
        let right = center + egui::vec2(RADIUS - BALL - 2.0, 0.0);
        assert_eq!(under(FRONT, center, right), Some(Side::Right));
        assert_eq!(under(FRONT, center, center + egui::vec2(15.0, 15.0)), None);
    }
}
