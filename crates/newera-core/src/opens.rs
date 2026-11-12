//! How a front moves when it is opened, and the room it takes while it does.
//!
//! A closed kitchen can look right in every view and still not work: a door
//! whose handle meets the fridge beside it, a drawer that pulls out into the
//! corner cabinet's handle, a dishwasher whose door drops onto the island, a
//! wall cabinet's lift-up door that swings into the hood above it. The
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
    /// The round door of a front-loading washer on its `left` edge or its
    /// right one: a hinge that stands off the face, so the door clears a
    /// niche a few cm deep, and swings past the side to about 135°.
    Porthole { left: bool },
    /// A door hinged at its bottom edge that drops to horizontal: an oven,
    /// a dishwasher, a built-in dryer.
    FlapDown,
    /// A door hinged at its top edge that lifts to horizontal: a wall
    /// cabinet's lift-up flap.
    FlapUp,
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
    /// `hinge-right`, `porthole-left`, `porthole-right`, `drawer 45`,
    /// `flap-down` or `flap-up`, each optionally followed by ` handle 3.7`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut words = text.split_whitespace();
        let motion = match words.next()? {
            "hinge-left" => Motion::Hinge { left: true },
            "hinge-right" => Motion::Hinge { left: false },
            "porthole-left" => Motion::Porthole { left: true },
            "porthole-right" => Motion::Porthole { left: false },
            "drawer" => Motion::Drawer {
                travel: words.next()?.parse().ok().filter(|t: &f64| *t > 0.0)?,
            },
            "flap-down" => Motion::FlapDown,
            "flap-up" => Motion::FlapUp,
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
            Motion::Porthole { left: true } => "porthole-left".to_owned(),
            Motion::Porthole { left: false } => "porthole-right".to_owned(),
            Motion::Drawer { travel } => format!("drawer {}", round(travel)),
            Motion::FlapDown => "flap-down".to_owned(),
            Motion::FlapUp => "flap-up".to_owned(),
        };
        if self.handle > 0.0 {
            format!("{motion} handle {}", round(self.handle))
        } else {
            motion
        }
    }

    /// How `piece` opens: what it says under [`OPENS_KEY`], or what an
    /// appliance of its catalog does — a fridge, a freezer or a microwave
    /// swings its door, a front-loading washer its porthole, an oven, a
    /// dishwasher or a dryer drops it. `None` for anything else.
    ///
    /// A fridge or a freezer hinges on its right, as they come from the
    /// factory; a microwave on its left, and a washer's porthole too, its
    /// handle in the door's rim.
    pub fn of(piece: &Furniture) -> Option<Self> {
        if let Some(text) = piece.properties.get(OPENS_KEY) {
            return Self::parse(text);
        }
        if !piece.children.is_empty() {
            return None;
        }
        let motion = match piece.catalog.as_str() {
            "fridge" | "freezer" => Motion::Hinge { left: false },
            "microwave" => Motion::Hinge { left: true },
            "washer" => Motion::Porthole { left: true },
            "oven" | "dishwasher" | "dryer" => Motion::FlapDown,
            _ => return None,
        };
        let handle = match motion {
            Motion::Porthole { .. } => 0.0,
            _ => APPLIANCE_HANDLE,
        };
        Some(Self { motion, handle })
    }

    /// What `piece` takes while this front opens: plan outlines, each with
    /// the heights it spans, cm above the floor — the front open first, then
    /// the room it passes through on the way when that is not the same
    /// prism. Only the room in front of its face counts — behind it is the
    /// piece's own body.
    ///
    /// A door sweeps a quarter disc of its own width around the hinge and,
    /// at 90°, stands beside it with its thickness and its handle: the part
    /// that meets a wall or a neighbour on the hinge side. A washer's porthole
    /// turns about a hinge standing off the face at the machine's edge, its
    /// rim sweeping a disc sector to past the side: what a niche no deeper
    /// than the hinge's stand-off never meets. A drawer comes
    /// out by its travel, a dropped or lifted door by its height (an
    /// appliance's door, not its body: its plinth and panel stay), each with
    /// the handle in front. Lying flat, a dropped door spans its hinge line at
    /// the bottom with the handle hanging under it, a lifted one its hinge
    /// line at the top with the handle standing over it. On the way a flap
    /// turns through a quarter disc standing on its hinge line, taken as a
    /// stack of bands: at `s` cm from the hinge line the leaf comes out no
    /// further than `√(leaf² − s²)` and its handle, and no band passes the
    /// leaf's own edge, so what stands just under a lifted flap's cabinet
    /// (a hood) or just over a dropped door's bottom is met only when the
    /// door really passes there.
    pub fn sweep(&self, piece: &Furniture) -> Vec<Slab> {
        let (w, depth) = (piece.width, piece.depth);
        let face = depth / 2.0;
        // A door board, or the door of an appliance whose box is the piece.
        let d = depth.min(MAX_DOOR);
        let (bottom, top) = piece.height_range();
        // A door board is the piece; an appliance's door leaves its plinth
        // and its control panel behind.
        let flap = if depth > MAX_DOOR {
            (top - bottom) * APPLIANCE_FLAP
        } else {
            top - bottom
        };
        let out_to = |reach: f64| {
            vec![
                (-w / 2.0, face),
                (w / 2.0, face),
                (w / 2.0, reach),
                (-w / 2.0, reach),
            ]
        };
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
            Motion::Porthole { left } => {
                let side = if left { 1.0 } else { -1.0 };
                let pivot = (-side * w / 2.0, face + PORTHOLE_PIVOT);
                let r = PORTHOLE_REACH.min(w) + self.handle;
                let mut out = vec![pivot];
                for step in 0..=PORTHOLE_STEPS {
                    let a =
                        PORTHOLE_ANGLE.to_radians() * f64::from(step) / f64::from(PORTHOLE_STEPS);
                    out.push((pivot.0 + side * r * a.cos(), pivot.1 + r * a.sin()));
                }
                out
            }
            Motion::Drawer { travel } => out_to(face + travel + self.handle),
            Motion::FlapDown | Motion::FlapUp => out_to(face + flap + self.handle),
        };
        let heights = match self.motion {
            // Lying flat at its hinge line, the door is as tall as it is
            // thick; its outer face, and the handle on it, turn down when it
            // drops and up when it lifts.
            Motion::FlapDown => (bottom - d - self.handle, bottom + d),
            Motion::FlapUp => (top - d, top + d + self.handle),
            _ => (bottom, top),
        };
        let plan = |local: Vec<(f64, f64)>| local.into_iter().map(|p| piece.to_plan(p)).collect();
        let mut slabs = vec![(plan(local), heights)];
        if matches!(self.motion, Motion::FlapDown | Motion::FlapUp) {
            for band in 0..FLAP_BANDS {
                let near = flap * f64::from(band) / f64::from(FLAP_BANDS);
                let far = flap * f64::from(band + 1) / f64::from(FLAP_BANDS);
                let reach = face + (flap * flap - near * near).sqrt() + self.handle;
                let z = if self.motion == Motion::FlapDown {
                    (bottom + near, bottom + far)
                } else {
                    (top - far, top - near)
                };
                slabs.push((plan(out_to(reach)), z));
            }
        }
        slabs
    }
}

impl Front {
    /// How far in front of `piece`'s face this front comes while it opens,
    /// cm: a door's width, a drawer's travel, a washer's porthole half open,
    /// a flap's height (an appliance's door, not its body), each with its
    /// handle — the room somebody using it
    /// stands behind.
    pub fn reach(&self, piece: &Furniture) -> f64 {
        self.sweep(piece)
            .into_iter()
            .flat_map(|(outline, _)| outline)
            .map(|p| piece.to_local(p).1 - piece.depth / 2.0)
            .fold(0.0, f64::max)
    }
}

/// A plan outline and the heights it spans, cm above the floor.
pub type Slab = (Vec<Point2>, (f64, f64));

/// Bands a flap's quarter turn is taken as: each 1/6 of the door's height,
/// about 12 cm on a 70 cm door.
const FLAP_BANDS: u32 = 6;

/// Thickest a door is taken to be, cm: a piece deeper than this is a body
/// with a door on its face (a fridge, an oven), not the door itself.
const MAX_DOOR: f64 = 6.0;

/// Share of an appliance's height its dropping door takes, the rest its
/// plinth and its control panel: a dryer 85 cm tall drops a door of about
/// 70 cm.
const APPLIANCE_FLAP: f64 = 0.82;

/// How far in from a door's free edge its handle reaches, cm: bars and
/// knobs sit within it.
const HANDLE_BAND: f64 = 10.0;

/// How far a washer's porthole hinge stands off the machine's face, cm: the
/// door clears a niche this deep.
const PORTHOLE_PIVOT: f64 = 4.0;

/// From a washer's porthole hinge to the far side of its rim, cm: measured
/// on a 60 cm machine, whose door half open comes 45 cm out of its face.
const PORTHOLE_REACH: f64 = 41.0;

/// How far a washer's porthole opens, degrees: fully open, the 60 cm
/// machine's door stands about 30 cm past its side and 33 cm out.
const PORTHOLE_ANGLE: f64 = 135.0;

/// Points along a washer's porthole sweep: one every 11.25°, so the door
/// standing straight out at 90° is one of them.
const PORTHOLE_STEPS: u32 = 12;

/// Points along a door's sweep.
const ARC_STEPS: u32 = 8;

/// `cm` to a tenth, as the property writes it.
fn round(cm: f64) -> f64 {
    (cm * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    /// A door board 60 × 2 × 70 cm, 10 cm above the floor at the origin,
    /// that opens as `opens` says.
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
            "porthole-left",
            "porthole-right handle 1",
            "drawer 45 handle 2",
            "flap-down",
            "flap-up handle 2.5",
        ] {
            assert_eq!(Front::parse(text).unwrap().property(), text);
        }
        for bad in [
            "",
            "hinge",
            "drawer",
            "drawer -3",
            "flap-down handle",
            "flap-up 3",
            "flap-sideways",
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
        for (catalog, motion) in [
            ("freezer", Motion::Hinge { left: false }),
            ("washer", Motion::Porthole { left: true }),
        ] {
            let appliance = Furniture {
                catalog: catalog.into(),
                ..Furniture::default()
            };
            assert_eq!(Front::of(&appliance).unwrap().motion, motion, "{catalog}");
        }
        for catalog in ["oven", "dishwasher", "dryer"] {
            let appliance = Furniture {
                catalog: catalog.into(),
                ..Furniture::default()
            };
            assert_eq!(
                Front::of(&appliance).unwrap().motion,
                Motion::FlapDown,
                "{catalog}"
            );
        }
        let sofa = Furniture {
            catalog: "sofa".into(),
            ..Furniture::default()
        };
        assert_eq!(Front::of(&sofa), None);
    }

    #[test]
    fn a_door_sweeps_in_front_and_stands_past_its_hinge_side() {
        let piece = door("hinge-left handle 3");
        let (outline, z) = Front::of(&piece).unwrap().sweep(&piece).swap_remove(0);
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
        let (outline, _) = Front::of(&right).unwrap().sweep(&right).swap_remove(0);
        assert_eq!(outline.iter().map(|p| p.x).fold(f64::MIN, f64::max), 35.0);
    }

    /// A washer's porthole, measured on a 60 cm machine: half open it comes
    /// 45 cm out of the face, fully open it stands about 30 cm past the
    /// hinge side and 33 cm out, and nothing of it is beside the machine
    /// within 4 cm of the face, so a niche that deep lets it open.
    #[test]
    fn a_washer_porthole_swings_past_its_hinge_side_clear_of_a_niche() {
        let washer = Furniture {
            catalog: "washer".into(),
            position: Point2::new(0.0, 0.0),
            width: 60.0,
            depth: 60.0,
            height: 85.0,
            ..Furniture::default()
        };
        let (outline, z) = Front::of(&washer).unwrap().sweep(&washer).swap_remove(0);
        assert_eq!(z, (0.0, 85.0));
        let max_y = outline.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        assert!((max_y - (30.0 + 45.0)).abs() < 0.5, "{max_y}");
        let min_x = outline.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        assert!((min_x - (-30.0 - 29.0)).abs() < 0.5, "{min_x}");
        let far = outline.last().unwrap();
        assert!((far.y - (30.0 + 33.0)).abs() < 0.5, "{far:?}");
        // Beside the machine (past its left edge), only beyond the niche.
        assert!(
            outline
                .iter()
                .filter(|p| p.x < -30.0 - 1e-9)
                .all(|p| p.y >= 30.0 + 4.0 - 1e-9),
            "{outline:?}"
        );
        // Never past the other side, never behind the face.
        assert!(outline.iter().all(|p| p.x <= 30.0 + 1e-9 && p.y >= 30.0));
        let mut right = washer.clone();
        right
            .properties
            .insert(OPENS_KEY.into(), "porthole-right".into());
        let (outline, _) = Front::of(&right).unwrap().sweep(&right).swap_remove(0);
        let max_x = outline.iter().map(|p| p.x).fold(f64::MIN, f64::max);
        assert!((max_x - (30.0 + 29.0)).abs() < 0.5, "{max_x}");
    }

    /// A front's reach is how far it comes out of the face: a washer's
    /// porthole half open, a drawer's travel and handle, wherever the piece
    /// stands and however it is turned.
    #[test]
    fn a_front_reaches_out_of_its_face_by_its_sweep() {
        let mut washer = Furniture {
            catalog: "washer".into(),
            position: Point2::new(200.0, 100.0),
            angle: 90.0,
            width: 60.0,
            depth: 60.0,
            height: 85.0,
            ..Furniture::default()
        };
        let reach = Front::of(&washer).unwrap().reach(&washer);
        assert!((reach - 45.0).abs() < 0.5, "{reach}");
        washer.mirrored = true;
        let reach = Front::of(&washer).unwrap().reach(&washer);
        assert!((reach - 45.0).abs() < 0.5, "{reach}");
        let dryer = Furniture {
            catalog: "dryer".into(),
            ..washer.clone()
        };
        let reach = Front::of(&dryer).unwrap().reach(&dryer);
        assert!(
            (reach - (69.7 + 4.0)).abs() < 0.1,
            "about 70 cm and its handle: {reach}"
        );
        let drawer = door("drawer 45 handle 3");
        let reach = Front::of(&drawer).unwrap().reach(&drawer);
        assert!((reach - 48.0).abs() < 1e-9, "{reach}");
    }

    /// A flap lies flat at its hinge line: a dropped one at its bottom edge
    /// with the handle under it, a lifted one at its top edge with the handle
    /// over it; in plan both come out by the door's height.
    #[test]
    fn a_flap_lies_flat_at_its_hinge_line_with_the_handle_outward() {
        // The door is 70 cm tall, 10 to 80 cm above the floor, 2 cm thick.
        let down = door("flap-down handle 3");
        let (outline, z) = Front::of(&down).unwrap().sweep(&down).swap_remove(0);
        assert_eq!(z, (5.0, 12.0));
        assert_eq!(outline.iter().map(|p| p.y).fold(f64::MIN, f64::max), 74.0);
        let up = door("flap-up handle 3");
        let (outline, z) = Front::of(&up).unwrap().sweep(&up).swap_remove(0);
        assert_eq!(z, (78.0, 85.0));
        assert_eq!(outline.iter().map(|p| p.y).fold(f64::MIN, f64::max), 74.0);
        // Nothing behind the face, nothing past its sides.
        assert!(
            outline
                .iter()
                .all(|p| p.y >= 1.0 - 1e-9 && p.x.abs() <= 30.0 + 1e-9)
        );
    }

    #[test]
    fn a_drawer_and_a_dropped_door_come_straight_out() {
        let piece = door("drawer 45 handle 3");
        let (outline, z) = Front::of(&piece).unwrap().sweep(&piece).swap_remove(0);
        assert_eq!(z, (10.0, 80.0));
        assert_eq!(outline.iter().map(|p| p.y).fold(f64::MIN, f64::max), 49.0);
        let oven = door("flap-down");
        let (outline, z) = Front::of(&oven).unwrap().sweep(&oven).swap_remove(0);
        assert_eq!(outline.iter().map(|p| p.y).fold(f64::MIN, f64::max), 71.0);
        assert_eq!(z, (8.0, 12.0));
    }
}
