//! How a front moves when it is opened, and the room it takes while it does.
//!
//! A closed kitchen can look right in every view and still not work: a door
//! whose handle meets the fridge beside it, a drawer that pulls out into the
//! corner cabinet's handle, a dishwasher whose door drops onto the island. The
//! boxes never overlap, so nothing else notices. A front that knows how it
//! moves can say what it runs into.
//!
//! The motion is a property of the piece that moves — a door or drawer front
//! of a joinery build, which the build sets, or an appliance, whose catalog
//! entry implies it — so the check reads the same thing for both.

use crate::furniture::Furniture;
use crate::geometry::Point2;

/// Property key holding a front's motion, written by [`Front::property`].
pub const OPENS_KEY: &str = "opens";

/// How a front moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motion {
    /// A door on a vertical axis at its `left` edge (seen from the front) or
    /// its right one, opened to 90°.
    Hinge { left: bool },
    /// A drawer pulled straight out by `travel` cm (its slide length).
    Drawer { travel: f64 },
    /// A door hinged at its bottom edge that drops to horizontal: an oven,
    /// a dishwasher.
    FlapDown,
}

/// A moving front: how it moves, and how far its handle stands off its
/// face, cm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Front {
    pub motion: Motion,
    /// Handle standing off the face, cm (0 for a groove or no handle).
    pub handle: f64,
}

/// Handle of an appliance door, cm: a fridge bar or an oven rail.
const APPLIANCE_HANDLE: f64 = 4.0;

impl Front {
    /// Reads the text [`Front::property`] writes: `hinge-left`,
    /// `hinge-right`, `drawer 45` or `flap-down`, each optionally followed
    /// by ` handle 3.7`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut words = text.split_whitespace();
        let motion = match words.next()? {
            "hinge-left" => Motion::Hinge { left: true },
            "hinge-right" => Motion::Hinge { left: false },
            "drawer" => Motion::Drawer {
                travel: words.next()?.parse().ok().filter(|t: &f64| *t > 0.0)?,
            },
            "flap-down" => Motion::FlapDown,
            _ => return None,
        };
        let handle = match (words.next(), words.next()) {
            (Some("handle"), Some(cm)) => cm.parse().ok().filter(|h: &f64| *h >= 0.0)?,
            (None, _) => 0.0,
            _ => return None,
        };
        Some(Self { motion, handle })
    }

    /// The text kept under [`OPENS_KEY`].
    pub fn property(&self) -> String {
        let motion = match self.motion {
            Motion::Hinge { left: true } => "hinge-left".to_owned(),
            Motion::Hinge { left: false } => "hinge-right".to_owned(),
            Motion::Drawer { travel } => format!("drawer {}", round(travel)),
            Motion::FlapDown => "flap-down".to_owned(),
        };
        if self.handle > 0.0 {
            format!("{motion} handle {}", round(self.handle))
        } else {
            motion
        }
    }

    /// How `piece` opens: what it says under [`OPENS_KEY`], or what an
    /// appliance of its catalog does — a fridge or a microwave swings its
    /// door, an oven or a dishwasher drops it. `None` for anything else.
    pub fn of(piece: &Furniture) -> Option<Self> {
        if let Some(text) = piece.properties.get(OPENS_KEY) {
            return Self::parse(text);
        }
        if !piece.children.is_empty() {
            return None;
        }
        let motion = match piece.catalog.as_str() {
            "fridge" => Motion::Hinge { left: false },
            "microwave" => Motion::Hinge { left: true },
            "oven" | "dishwasher" => Motion::FlapDown,
            _ => return None,
        };
        Some(Self {
            motion,
            handle: APPLIANCE_HANDLE,
        })
    }

    /// What `piece` takes while this front opens: a plan outline and the
    /// heights it spans, cm above the floor. Only the room in front of its
    /// face counts — behind it is the piece's own body.
    ///
    /// A door sweeps a quarter disc of its own width around the hinge and,
    /// at 90°, stands beside it with its thickness and its handle: the part
    /// that meets a wall or a neighbour on the hinge side. A drawer comes
    /// out by its travel, a dropped door by its height, each with the handle
    /// in front.
    pub fn sweep(&self, piece: &Furniture) -> (Vec<Point2>, (f64, f64)) {
        let (w, depth) = (piece.width, piece.depth);
        let face = depth / 2.0;
        // A door board, or the door of an appliance whose box is the piece.
        let d = depth.min(MAX_DOOR);
        let (bottom, top) = piece.height_range();
        let local: Vec<(f64, f64)> = match self.motion {
            Motion::Hinge { left } => {
                // Worked out for a left hinge; a right one is its mirror.
                let side = if left { 1.0 } else { -1.0 };
                let pivot = (-side * w / 2.0, face);
                let mut out = vec![pivot];
                for step in 0..=ARC_STEPS {
                    let a = std::f64::consts::FRAC_PI_2 * f64::from(step) / f64::from(ARC_STEPS);
                    out.push((pivot.0 + side * w * a.cos(), face + w * a.sin()));
                }
                // Open, the leaf stands past the hinge side by its
                // thickness, and its handle — near the free edge, now the
                // far end — by the handle's depth on top of that.
                let leaf = pivot.0 - side * d;
                let grip = leaf - side * self.handle;
                let band = (w / 2.0).min(HANDLE_BAND);
                out.extend([
                    (grip, face + w),
                    (grip, face + w - band),
                    (leaf, face + w - band),
                    (leaf, face),
                ]);
                out
            }
            Motion::Drawer { travel } => {
                let reach = face + travel + self.handle;
                vec![
                    (-w / 2.0, face),
                    (w / 2.0, face),
                    (w / 2.0, reach),
                    (-w / 2.0, reach),
                ]
            }
            Motion::FlapDown => {
                let reach = face + (top - bottom) + self.handle;
                vec![
                    (-w / 2.0, face),
                    (w / 2.0, face),
                    (w / 2.0, reach),
                    (-w / 2.0, reach),
                ]
            }
        };
        let heights = match self.motion {
            // Lying flat at its hinge line, the door is as tall as it is thick.
            Motion::FlapDown => (bottom - d, bottom + d + self.handle),
            _ => (bottom, top),
        };
        (
            local.into_iter().map(|p| piece.to_plan(p)).collect(),
            heights,
        )
    }
}

/// Thickest a door is taken to be, cm: a piece deeper than this is a body
/// with a door on its face (a fridge, an oven), not the door itself.
const MAX_DOOR: f64 = 6.0;

/// How far in from a door's free edge its handle reaches, cm: bars and
/// knobs sit within it.
const HANDLE_BAND: f64 = 10.0;

/// Points along a door's quarter-disc sweep.
const ARC_STEPS: u32 = 8;

fn round(cm: f64) -> f64 {
    (cm * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    fn door(opens: &str) -> Furniture {
        let mut piece = Furniture {
            catalog: "box".into(),
            position: Point2::new(0.0, 0.0),
            width: 60.0,
            depth: 2.0,
            height: 70.0,
            elevation: 10.0,
            ..Furniture::default()
        };
        piece.properties.insert(OPENS_KEY.into(), opens.into());
        piece
    }

    #[test]
    fn the_property_reads_back_what_it_wrote() {
        for text in [
            "hinge-left",
            "hinge-right handle 3.7",
            "drawer 45 handle 2",
            "flap-down",
        ] {
            assert_eq!(Front::parse(text).unwrap().property(), text);
        }
        for bad in [
            "",
            "hinge",
            "drawer",
            "drawer -3",
            "flap-down handle",
            "hinge-left knob 2",
        ] {
            assert_eq!(Front::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn appliances_open_the_way_their_catalog_says() {
        let fridge = Furniture {
            catalog: "fridge".into(),
            ..Furniture::default()
        };
        assert_eq!(
            Front::of(&fridge).unwrap().motion,
            Motion::Hinge { left: false }
        );
        let sofa = Furniture {
            catalog: "sofa".into(),
            ..Furniture::default()
        };
        assert_eq!(Front::of(&sofa), None);
    }

    #[test]
    fn a_door_sweeps_in_front_and_stands_past_its_hinge_side() {
        let piece = door("hinge-left handle 3");
        let (outline, z) = Front::of(&piece).unwrap().sweep(&piece);
        assert_eq!(z, (10.0, 80.0));
        let xs: Vec<f64> = outline.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = outline.iter().map(|p| p.y).collect();
        // Past the left edge by its thickness and handle, never past the right.
        assert_eq!(xs.iter().copied().fold(f64::MAX, f64::min), -35.0);
        assert!(xs.iter().all(|x| *x <= 30.0 + 1e-9));
        // Nothing behind its face; a leaf's width in front of it.
        assert!(ys.iter().all(|y| *y >= 1.0 - 1e-9));
        assert!((ys.iter().copied().fold(f64::MIN, f64::max) - 61.0).abs() < 1e-9);
        // A right hinge is the mirror image.
        let right = door("hinge-right handle 3");
        let (outline, _) = Front::of(&right).unwrap().sweep(&right);
        assert_eq!(outline.iter().map(|p| p.x).fold(f64::MIN, f64::max), 35.0);
    }

    #[test]
    fn a_drawer_and_a_dropped_door_come_straight_out() {
        let piece = door("drawer 45 handle 3");
        let (outline, z) = Front::of(&piece).unwrap().sweep(&piece);
        assert_eq!(z, (10.0, 80.0));
        assert_eq!(outline.iter().map(|p| p.y).fold(f64::MIN, f64::max), 49.0);
        let oven = door("flap-down");
        let (outline, z) = Front::of(&oven).unwrap().sweep(&oven);
        assert_eq!(outline.iter().map(|p| p.y).fold(f64::MIN, f64::max), 71.0);
        assert_eq!(z, (8.0, 12.0));
    }
}
