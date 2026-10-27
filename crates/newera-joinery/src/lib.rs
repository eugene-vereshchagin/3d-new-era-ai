//! Parametric joinery and interiors.
//!
//! An agent (or the user) states intent with a few flat parameters — a
//! 200 cm wardrobe with two hinged doors and four shelves — and this crate
//! does the exact work: every board with its thickness, grooves, clearances
//! and hardware.
//!
//! What a workshop would say about the request is said, never enforced. A
//! board thickness nobody stocks, a shelf that will sag, a drawer front too
//! short to grip, a niche shallower than the cooktop standard asks for: all of
//! it is built, and comes back in [`Output::notes`]. Most people drawing here
//! are learning, sketching or just looking at an idea, and a rule that refuses
//! to draw teaches nothing; the notes are there for when the drawing becomes
//! something to cut.
//!
//! A build fails only on what has no geometry at all — a box whose inside is
//! zero or negative, a part that would come out backwards, a fixture that goes
//! somewhere else entirely. That is arithmetic, not an opinion about the
//! project.
//!
//! Geometry is in centimeters, board thicknesses in millimeters as they are
//! sold. A build is a list of [`Part`]s in the piece's local frame (x across
//! the width from the left edge, y from the back to the front, z up from the
//! floor) that [`assemble`] turns into a furniture group, and [`cut_list`]
//! turns into rows for the workshop.

// Board counts and sizes in cm are small, far from integer or float limits.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

mod cabinet;
mod ceiling;
mod countertop;
mod cutlist;
mod embed;
mod roof;
mod run;
mod slats;
mod sofa;
mod wall;

pub use cabinet::{CabinetParams, DoorType, HandleColor, HandleStyle, Niche};
pub use ceiling::{CoveParams, CoveType, ShadowGapParams};
pub use countertop::{CountertopParams, Cutout, CutoutKind, Support};
pub use cutlist::{CutRow, cut_list, cut_list_csv, cut_list_dxf, cut_list_svg};
pub use embed::{EMBED_KEY, EmbedRequest, Fixture, carry_embedded, embed, fixture_of};
pub use roof::fit_joinery_to_roof;
pub use run::{
    EndKind, FillerParams, Interior, Role, RunGap, RunModule, RunOver, RunParams, RunRow, plan_run,
};
pub use slats::{Orientation, SlatsParams};
pub use sofa::{ArmType, SofaParams};
pub use wall::{CabinetRunParams, RUN_KEY, cabinet_run};

use newera_core::{Furniture, FurnitureId, Material, Point2, Properties};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Property keys stored on generated groups and parts.
pub const KIND_KEY: &str = "joinery:kind";
pub const PARAMS_KEY: &str = "joinery:params";
pub const PART_KEY: &str = "joinery:part";
pub const BOARD_KEY: &str = "joinery:board";
pub const EDGE_KEY: &str = "joinery:edge";

/// What to build, with its parameters. Every field has a sensible default,
/// so `{"kind":"cabinet","w":200}` is a complete request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Build {
    Cabinet(CabinetParams),
    Slats(SlatsParams),
    Countertop(CountertopParams),
    Cove(CoveParams),
    ShadowGap(ShadowGapParams),
    Sofa(SofaParams),
    Filler(FillerParams),
}

impl Build {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Cabinet(_) => "cabinet",
            Self::Slats(_) => "slats",
            Self::Countertop(_) => "countertop",
            Self::Cove(_) => "cove",
            Self::ShadowGap(_) => "shadow_gap",
            Self::Sofa(_) => "sofa",
            Self::Filler(_) => "filler",
        }
    }
}

/// One board, block or strip, as a box in the local frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// Workshop name, e.g. `Lateral esquerda`.
    pub name: String,
    /// Rotation of this part within the assembly.
    pub angle: f64,
    /// Rated emitter, when this part is a luminaire.
    pub light: Option<newera_core::Light>,
    /// Minimum corner, cm.
    pub at: [f64; 3],
    /// Size along x, y, z, cm.
    pub size: [f64; 3],
    /// Board material for the cut list (`MDF 18`), `None` for hardware-like
    /// or non-sheet parts (cushions, LED strips).
    pub board: Option<String>,
    /// Look in 3D.
    pub color: [u8; 3],
    pub finish: Option<Material>,
    /// Edges to band, as a count of long and short edges `[long, short]`.
    pub edge: [u8; 2],
    pub opacity: Option<f64>,
    /// Plan outline (local x, y) instead of a box: mitred plaster strips
    /// following a room. `at`/`size` still give its bounds and height.
    pub outline: Option<Vec<[f64; 2]>>,
    /// Length × width for the cut list when it isn't the box (mitred strips).
    pub cut: Option<[f64; 2]>,
    /// Holes through the board for the workshop `[x, y, w, d]`, cm from its
    /// left-back corner (sink and cooktop cutouts in a stone top).
    pub holes: Vec<[f64; 4]>,
    /// Face shape `[x, z]` from its left-bottom corner instead of a rectangle
    /// (a board cut along a roof slope), swept through its depth.
    pub profile: Option<Vec<[f64; 2]>>,
}

impl Part {
    pub(crate) fn board(
        name: &str,
        at: [f64; 3],
        size: [f64; 3],
        board: &str,
        color: [u8; 3],
    ) -> Self {
        Self {
            name: name.to_owned(),
            angle: 0.0,
            light: None,
            at,
            size,
            board: Some(board.to_owned()),
            color,
            finish: None,
            edge: [0, 0],
            opacity: None,
            outline: None,
            cut: None,
            holes: Vec::new(),
            profile: None,
        }
    }

    pub(crate) fn solid(name: &str, at: [f64; 3], size: [f64; 3], color: [u8; 3]) -> Self {
        Self {
            board: None,
            ..Self::board(name, at, size, "", color)
        }
    }

    pub(crate) fn banded(mut self, long: u8, short: u8) -> Self {
        self.edge = [long, short];
        self
    }
}

/// A finished build: its parts, overall size and what the workshop should know.
#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub parts: Vec<Part>,
    /// Overall width, depth, height, cm.
    pub size: [f64; 3],
    /// Hardware to buy, e.g. `8 dobradiças de caneco 35 mm`.
    pub hardware: Vec<String>,
    /// Things that work but deserve a look.
    pub notes: Vec<String>,
    /// Boards for the cut list that aren't drawn as parts (a whole stone top
    /// the 3D splits around its cutouts).
    pub extra_cuts: Vec<Part>,
    /// Default name of the group.
    pub name: String,
}

/// Generates the parts for `build`.
///
/// # Errors
/// A sentence describing what cannot be built and the value that would work.
pub fn generate(build: &Build) -> Result<Output, String> {
    match build {
        Build::Cabinet(p) => cabinet::generate(p),
        Build::Slats(p) => slats::generate(p),
        Build::Countertop(p) => countertop::generate(p),
        Build::Cove(p) => ceiling::cove(p),
        Build::ShadowGap(p) => ceiling::shadow_gap(p),
        Build::Sofa(p) => sofa::generate(p),
        Build::Filler(p) => run::filler(p),
    }
}

/// Merges `patch` (new values for some parameters) into the parameters a
/// group was built with.
///
/// # Errors
/// When the stored or merged parameters don't describe a build.
pub fn merged(stored: &str, patch: &serde_json::Value) -> Result<Build, String> {
    let mut value: serde_json::Value = serde_json::from_str(stored)
        .map_err(|e| format!("stored parameters are unreadable: {e}"))?;
    if let (Some(target), Some(changes)) = (value.as_object_mut(), patch.as_object()) {
        for (key, v) in changes {
            if key != "kind" {
                target.insert(key.clone(), v.clone());
            }
        }
    }
    parse_params(&value)
}

/// Makes `build` — the stored parameters with a change merged in — keep the
/// size the group has now (`width, depth, height`) where it was resized by
/// other means than joinery (`update w=…`) and the change doesn't give that
/// size again. Without it, the next change rebuilt the group at the size the
/// parameters remembered. Returns a note when a size was taken.
pub fn follow_resize(
    stored: &str,
    build: &mut Build,
    actual: [f64; 3],
    patch: &serde_json::Value,
) -> Option<String> {
    let before = merged(stored, &serde_json::json!({}))
        .and_then(|b| generate(&b))
        .ok()?
        .size;
    let given = |key: &str| patch.get(key).is_some();
    // Each axis's parameter, and the axis of the group's box it sets.
    let mut taken = Vec::new();
    let mut follow = |key: &'static str, value: &mut f64, axis: usize| {
        let change = actual[axis] - before[axis];
        if change.abs() > 0.05 && !given(key) {
            *value += change;
            taken.push(format!("{key} = {}", num(*value)));
        }
    };
    match build {
        Build::Cabinet(p) => {
            follow("w", &mut p.w, 0);
            follow("d", &mut p.d, 1);
            follow("h", &mut p.h, 2);
        }
        Build::Slats(p) => {
            follow("w", &mut p.w, 0);
            follow("h", &mut p.h, 2);
        }
        Build::Countertop(p) => {
            follow("length", &mut p.length, 0);
            follow("depth", &mut p.depth, 1);
        }
        Build::Sofa(p) => {
            follow("length", &mut p.length, 0);
            follow("depth", &mut p.depth, 1);
        }
        _ => {}
    }
    (!taken.is_empty()).then(|| {
        format!(
            "Medidas atuais da peça mantidas ({}): ela foi redimensionada fora do joinery.",
            taken.join(", ")
        )
    })
}

/// Reads parameters, naming the field that does not fit.
///
/// serde says `invalid type: boolean true, expected u32` and not where: in a
/// request of ten keys, two of them booleans, that is a guessing game. Every
/// field of a build has a default, so each key can be tried on its own, and
/// the first one that fails alone is the one to name.
///
/// # Errors
/// When the parameters don't describe what `T` is, with the field named.
pub fn parse_params<T: serde::de::DeserializeOwned>(
    value: &serde_json::Value,
) -> Result<T, String> {
    let error = match T::deserialize(value) {
        Ok(parsed) => return Ok(parsed),
        Err(e) => e.to_string(),
    };
    let culprit = value.as_object().and_then(|fields| {
        let tag = fields.get("kind").cloned();
        fields
            .iter()
            .filter(|(key, _)| *key != "kind")
            .find_map(|(key, v)| {
                let mut alone = serde_json::Map::new();
                if let Some(tag) = &tag {
                    alone.insert("kind".to_owned(), tag.clone());
                }
                alone.insert(key.clone(), v.clone());
                match T::deserialize(&serde_json::Value::Object(alone)) {
                    Err(e) if !e.to_string().starts_with("missing field") => {
                        Some(format!("`{key}`: {e}"))
                    }
                    _ => None,
                }
            })
    });
    Err(format!("invalid parameters: {}", culprit.unwrap_or(error)))
}

/// The build as a furniture group placed with its back-left-bottom corner
/// logic centered at `position`, turned `angle` degrees, raised `elevation`.
/// Pieces get ids from `next_id`.
pub fn assemble(
    build: &Build,
    output: &Output,
    group_id: FurnitureId,
    position: Point2,
    angle: f64,
    elevation: f64,
    next_id: &mut dyn FnMut() -> FurnitureId,
) -> Furniture {
    let [w, d, h] = output.size;
    let mut properties = Properties::new();
    properties.insert(KIND_KEY.into(), build.kind().into());
    properties.insert(
        PARAMS_KEY.into(),
        serde_json::to_string(build).unwrap_or_default(),
    );
    let mut group = Furniture {
        id: group_id,
        catalog: "group".into(),
        name: output.name.clone(),
        position,
        angle,
        elevation,
        width: w.max(0.1),
        depth: d.max(0.1),
        height: h.max(0.1),
        properties,
        ..Furniture::default()
    };
    group.children = output
        .parts
        .iter()
        .map(|part| {
            // Local frame: x from the left edge, y from the back; the group is
            // centered, with its front toward +y like every piece.
            let cx = part.at[0] + part.size[0] / 2.0 - w / 2.0;
            let cy = part.at[1] + part.size[1] / 2.0 - d / 2.0;
            let mut properties = Properties::new();
            properties.insert(PART_KEY.into(), part.name.clone());
            if let Some(board) = &part.board {
                properties.insert(BOARD_KEY.into(), board.clone());
            }
            if part.edge != [0, 0] {
                properties.insert(
                    EDGE_KEY.into(),
                    format!("{}+{}", part.edge[0], part.edge[1]),
                );
            }
            Furniture {
                id: next_id(),
                catalog: "box".into(),
                name: part.name.clone(),
                position: group.to_plan((cx, cy)),
                angle: angle + part.angle,
                light: part.light.clone(),
                elevation: elevation + part.at[2],
                width: part.size[0].max(0.05),
                depth: part.size[1].max(0.05),
                height: part.size[2].max(0.05),
                // A plain color finish is just the piece's color.
                color: Some(
                    part.finish
                        .as_ref()
                        .filter(|f| f.pattern.is_none() && f.image.is_none())
                        .and_then(|f| f.color)
                        .unwrap_or(part.color),
                ),
                texture: part
                    .finish
                    .clone()
                    .filter(|f| f.pattern.is_some() || f.image.is_some()),
                opacity: part.opacity,
                shape: part
                    .profile
                    .as_ref()
                    .map(|points| {
                        newera_core::SolidShape::Profile(
                            points
                                .iter()
                                .map(|p| [p[0] - part.size[0] / 2.0, p[1]])
                                .collect(),
                        )
                    })
                    .or_else(|| {
                        part.outline.as_ref().map(|points| {
                            let center = [
                                part.at[0] + part.size[0] / 2.0,
                                part.at[1] + part.size[1] / 2.0,
                            ];
                            newera_core::SolidShape::Outline(
                                points
                                    .iter()
                                    .map(|p| [p[0] - center[0], p[1] - center[1]])
                                    .collect(),
                            )
                        })
                    }),
                properties,
                ..Furniture::default()
            }
        })
        .collect();
    group
}

/// Board thickness in cm from millimeters.
pub(crate) fn cm(mm: f64) -> f64 {
    mm / 10.0
}

/// A number written the Brazilian way in messages: `55`, `1,8`.
pub(crate) fn num(v: f64) -> String {
    let rounded = (v * 10.0).round() / 10.0;
    let text = if (rounded - rounded.round()).abs() < 1e-9 {
        format!("{}", rounded.round())
    } else {
        format!("{rounded:.1}")
    };
    text.replace('.', ",")
}

pub(crate) const MDF_WHITE: [u8; 3] = [238, 236, 230];

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    #[test]
    fn a_wrong_type_names_its_field() {
        let err = super::parse_params::<super::RunParams>(&serde_json::json!({
            "row": "base", "h": 87, "d": 65, "top": true, "drawers": true
        }))
        .unwrap_err();
        assert!(err.contains("`drawers`"), "{err}");
        assert!(!err.contains("`top`"), "{err}");
        let err = super::parse_params::<super::Build>(
            &serde_json::json!({"kind": "cabinet", "w": "largo"}),
        )
        .unwrap_err();
        assert!(err.contains("`w`"), "{err}");
        assert!(
            super::parse_params::<super::Build>(&serde_json::json!({"kind": "cabinet", "w": 80}))
                .is_ok()
        );
    }

    use super::*;

    #[test]
    fn assembled_groups_place_parts_and_keep_their_parameters() {
        let build: Build = serde_json::from_str(
            r##"{"kind":"cabinet","w":100,"h":90,"d":50,"front":"#5f6e4a","shelves":1}"##,
        )
        .unwrap();
        let output = generate(&build).unwrap();
        let mut next_id = 10;
        let group = assemble(
            &build,
            &output,
            FurnitureId(1),
            Point2::new(500.0, 200.0),
            90.0,
            0.0,
            &mut || {
                next_id += 1;
                FurnitureId(next_id)
            },
        );
        assert_eq!(group.children.len(), output.parts.len());
        assert!(
            group
                .properties
                .get(PARAMS_KEY)
                .is_some_and(|p| p.contains("\"w\":100"))
        );
        // Turned 90°: the left side (local x −48,1) lands at plan y −48,1 from the center.
        let side = group
            .children
            .iter()
            .find(|c| c.name == "Lateral esquerda")
            .unwrap();
        assert!(
            (side.position.x - 500.0).abs() < 2.0
                && (side.position.y - (200.0 - 49.1)).abs() < 0.01,
            "{:?}",
            side.position
        );
        let door = group
            .children
            .iter()
            .find(|c| c.name.starts_with("Porta"))
            .unwrap();
        assert_eq!(door.color, Some([0x5f, 0x6e, 0x4a]));
        assert!(door.texture.is_none());
        assert_eq!(
            door.properties.get(BOARD_KEY).map(String::as_str),
            Some("MDF 18")
        );
    }

    #[test]
    fn builds_round_trip_and_merge_patches() {
        let build: Build = serde_json::from_str(r#"{"kind":"cabinet","w":120}"#).unwrap();
        let stored = serde_json::to_string(&build).unwrap();
        let patched = merged(&stored, &serde_json::json!({"shelves": 3, "kind": "sofa"})).unwrap();
        let Build::Cabinet(p) = patched else {
            panic!("kind never changes")
        };
        assert_eq!((p.w, p.shelves), (120.0, 3));
        assert!(merged(&stored, &serde_json::json!({"w": "wide"})).is_err());
        assert_eq!(num(1.8), "1,8");
        assert_eq!(num(55.0), "55");
    }

    /// The rule that used to stop the drawing now travels with it. Somebody
    /// laying out a kitchen to learn, to try an idea or to see what it would
    /// look like gets the cabinet and the workshop's opinion, not a refusal.
    #[test]
    fn a_rule_is_a_note_and_never_a_refusal() {
        // A cooktop over a shallow cabinet, a board nobody stocks, a shelf
        // that will sag, a sofa nobody sits on: every one of them is built.
        for (json, word) in [
            (
                r#"{"kind":"cabinet","w":100,"h":85,"d":35,"cooktop":true}"#,
                "cooktop",
            ),
            (
                r#"{"kind":"cabinet","w":100,"h":85,"d":55,"t":16}"#,
                "15, 18 ou 25",
            ),
            (
                r#"{"kind":"cabinet","w":160,"h":85,"d":55,"t":15,"dividers":0,"shelves":2}"#,
                "barrigar",
            ),
            (r#"{"kind":"sofa","seat":70}"#, "35 a 55"),
            (
                r#"{"kind":"cove","pts":[[0,0],[300,0],[300,300],[0,300]],"drop":3}"#,
                "LED",
            ),
        ] {
            let build: Build = serde_json::from_str(json).expect(json);
            let out = generate(&build).unwrap_or_else(|e| panic!("{json} was refused: {e}"));
            assert!(
                out.notes.iter().any(|n| n.contains(word)),
                "{json}: {:?}",
                out.notes
            );
            assert!(!out.parts.is_empty(), "{json} came back empty");
        }

        // What has no geometry at all still fails, and says so plainly.
        let flat: Build =
            serde_json::from_str(r#"{"kind":"cabinet","w":2,"h":85,"d":55}"#).unwrap();
        assert!(
            generate(&flat)
                .unwrap_err()
                .contains("não tem lado de dentro")
        );
    }
}
