//! Parametric joinery: one build at a time, and the boards it comes to.
//!
//! The server computes every board, clearance and rule, so a build either
//! answers with its parts or says what to change.

use newera_core::{Command, Point2};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{ErrorData, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::NewEraMcp;
use super::reply::{core, invalid};
use crate::compact;
use crate::edit;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct JoineryParams {
    /// What a new build is.
    #[schemars(extend("enum" = ["cabinet", "slats", "countertop", "cove", "shadow_gap", "sofa"]))]
    kind: Option<String>,
    /// Or an existing build's group id: `p` holds only what changes.
    id: Option<String>,
    /// Parameters (flat; everything has a default), see the tool description.
    p: Option<serde_json::Map<String, serde_json::Value>>,
    /// Center on the plan (default: 0,0), or `wall` (+`along` cm) to back it onto a wall.
    at: Option<Point2>,
    /// Wall to back it onto, instead of `at`.
    wall: Option<String>,
    /// With `wall`: center along it, cm from its start (default: the middle).
    along: Option<f64>,
    /// Clockwise degrees.
    angle: Option<f64>,
    /// Bottom above the floor, cm (wall cabinets).
    elev: Option<f64>,
    /// `cove`/`shadow_gap`: room id whose outline to follow.
    room: Option<String>,
    /// Changing a build's size: the face that stays where it is — `back`
    /// (default: joinery stands against a wall), `front`, `left`, `right`,
    /// `bottom`, `top` or a plan side `+x` `-x` `+y` `-y`; `center` grows
    /// around the middle.
    anchor: Option<String>,
    /// Only check and report, create nothing.
    #[serde(default)]
    dry: bool,
}
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CutListParams {
    /// Build or drawn group ids (default: every build and drawn group on this storey).
    ids: Option<Vec<String>>,
    /// Write `.csv` (spreadsheet), `.dxf` (sheets for CNC) or `.svg` (sheets to view).
    #[schemars(skip)]
    path: Option<String>,
}
/// Where to write a cut list.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CutListExportParams {
    /// Build or drawn group ids (default: every build and drawn group on this storey).
    ids: Option<Vec<String>>,
    /// `.csv` (spreadsheet), `.dxf` (sheets for CNC) or `.svg` (sheets to view).
    path: String,
}
/// Thickest part of a drawn group read as a board, mm: a 50 mm stone or a
/// double panel still is, a 30 cm body of a model is not.
const DRAWN_BOARD_MM: f64 = 50.0;

#[tool_router(router = joinery_router, vis = "pub(crate)")]
impl NewEraMcp {
    #[tool(
        description = "Parametric joinery and interiors: the server computes every board, clearance and rule and answers {id,name,size,parts,hardware,notes}. Workshop advice (a board nobody stocks, a sagging shelf, a short drawer front) comes back in notes and the piece is built anyway; only what has no geometry fails, saying why. kind + p: cabinet {w,h,d cm; t 15|18|25 mm; back mm; door hinged|sliding|drawers|none; doors; shelves; drawers; dividers; plinth; cooktop; top_cutout [x,y,w,d] cm from the left-back; color; front finish} · slats {w,h cm; slat, thickness, gap mm; orientation vertical|horizontal; backing; finish} · countertop {length,depth,height,thickness cm; material; support none|legs|brackets; cutouts [{kind sink|cooktop|grommet, x, w?, d?}]} · cove {room or pts; type open|closed|inverted; ceiling, width, drop, slot cm; led} · shadow_gap {room or pts; ceiling, gap, depth cm; led} · sofa {length,depth,seat,back cm; arms straight|rounded|none; modules; color}. Place with at|wall(+along), angle, elev. Change a build with id + p holding only the new values; a new size keeps its back (anchor back|front|left|right|bottom|top|+x|-x|+y|-y|center). dry=true validates only."
    )]
    pub(crate) fn joinery(
        &self,
        Parameters(p): Parameters<JoineryParams>,
    ) -> Result<String, ErrorData> {
        let mut doc = self.document.write();
        let patch = serde_json::Value::Object(p.p.clone().unwrap_or_default());
        // What to build: a new kind, or an existing group's parameters with changes.
        let mut resized = None;
        let (build, existing) = match (&p.id, &p.kind) {
            (Some(id), _) => {
                let id: newera_core::FurnitureId =
                    id.parse().map_err(|e| invalid(format!("{e}")))?;
                let group = doc
                    .home()
                    .furniture
                    .iter()
                    .find(|f| f.id == id)
                    .cloned()
                    .ok_or_else(|| invalid(format!("{id} not found")))?;
                let stored = group
                    .properties
                    .get(newera_joinery::PARAMS_KEY)
                    .ok_or_else(
                        // Saying what the piece IS turns a dead end into the
                        // next call: the tool that owns it is named.
                        || {
                            invalid(format!(
                                "{id} was not made by joinery; it was made by {}",
                                compact::made_by(&group)
                            ))
                        },
                    )?;
                let mut build = newera_joinery::merged(stored, &patch).map_err(invalid)?;
                resized = newera_joinery::follow_resize(
                    stored,
                    &mut build,
                    [group.width, group.depth, group.height],
                    &patch,
                );
                (build, Some(group))
            }
            (None, Some(kind)) => {
                let mut value = patch.clone();
                value["kind"] = serde_json::Value::String(kind.clone());
                let build: newera_joinery::Build =
                    newera_joinery::parse_params(&value).map_err(invalid)?;
                (build, None)
            }
            (None, None) => {
                return Err(invalid("give `kind` for a new build or `id` to change one"));
            }
        };
        // Coves and shadow gaps follow a room's outline, kept relative to its corner.
        let mut build = build;
        let mut origin = None;
        if let newera_joinery::Build::Cove(newera_joinery::CoveParams { pts, .. })
        | newera_joinery::Build::ShadowGap(newera_joinery::ShadowGapParams { pts, .. }) =
            &mut build
        {
            let absolute: Option<Vec<Point2>> = match (&p.room, existing.is_some()) {
                (Some(room), _) => {
                    let id: newera_core::RoomId =
                        room.parse().map_err(|e| invalid(format!("{e}")))?;
                    Some(
                        doc.home()
                            .rooms
                            .iter()
                            .find(|r| r.id == id)
                            .ok_or_else(|| invalid(format!("{room} not found")))?
                            .points
                            .clone(),
                    )
                }
                (None, false) if !pts.is_empty() => {
                    Some(pts.iter().map(|q| Point2::new(q[0], q[1])).collect())
                }
                _ => None,
            };
            if let Some(points) = absolute {
                let (lo_x, lo_y) = points
                    .iter()
                    .fold((f64::MAX, f64::MAX), |(x, y), q| (x.min(q.x), y.min(q.y)));
                *pts = points.iter().map(|q| [q.x - lo_x, q.y - lo_y]).collect();
                origin = Some(Point2::new(lo_x, lo_y));
            }
        }
        let mut output = newera_joinery::generate(&build).map_err(invalid)?;
        output.notes.extend(resized);
        let [w, d, h] = output.size;
        let summary = |id: &str| {
            serde_json::json!({
                "id": id,
                "name": output.name,
                "size": [compact::num(w), compact::num(d), compact::num(h)],
                "parts": output.parts.len(),
                "hardware": output.hardware,
                "notes": output.notes,
            })
        };
        if p.dry {
            return Ok(summary("").to_string());
        }
        // Placement: an explicit spot, the build's previous one, or the room corner.
        let mut place = newera_core::Furniture {
            width: w,
            depth: d,
            height: h,
            ..newera_core::Furniture::default()
        };
        if let Some(group) = &existing {
            place.position = group.position;
            place.angle = group.angle;
            place.elevation = group.elevation;
            // A change of size keeps one face where it was — the back, unless
            // told otherwise — instead of shrinking from both sides and
            // leaving a gap behind a cabinet that stood on its wall.
            let anchor = p.anchor.as_deref().unwrap_or("back");
            if p.at.is_none() && p.wall.is_none() && anchor != "center" {
                place.mirrored = group.mirrored;
                edit::hold_face(
                    &mut place,
                    anchor,
                    (group.width, group.depth, group.height, group.elevation),
                )
                .map_err(invalid)?;
            }
        }
        if let Some(o) = origin {
            place.position = Point2::new(o.x + w / 2.0, o.y + d / 2.0);
            place.angle = 0.0;
        }
        if let Some(at) = p.at {
            place.position = at;
        }
        // A build stays on its storey; a new one lands on the storey being
        // edited, and one backed onto a wall on that wall's storey.
        let mut level = existing
            .as_ref()
            .map_or_else(|| doc.home().current_level(), |g| g.level);
        if let Some(wall) = &p.wall {
            let wall_id: newera_core::WallId = wall.parse().map_err(|e| invalid(format!("{e}")))?;
            let wall = doc
                .home()
                .wall(wall_id)
                .cloned()
                .ok_or_else(|| invalid(format!("{wall} not found")))?;
            level = doc.home().resolve_level(wall.level);
            let along = p
                .along
                .unwrap_or_else(|| wall.start.distance(wall.end) / 2.0);
            newera_core::align_to_wall(&mut place, &wall, along);
            edit::back_to_wall(&doc, &mut place, &wall, p.angle);
        }
        if let Some(angle) = p.angle {
            place.angle = angle;
        }
        if let Some(elev) = p.elev {
            place.elevation = elev;
        }
        let group_id = existing
            .as_ref()
            .map_or_else(|| doc.new_furniture_id(), |g| g.id);
        let group = {
            let doc_ref = &mut *doc;
            let mut next = || doc_ref.new_furniture_id();
            newera_joinery::assemble(
                &build,
                &output,
                group_id,
                place.position,
                place.angle,
                place.elevation,
                &mut next,
            )
        };
        // A name the user gave survives changes; the generated one follows them.
        let renamed = existing.as_ref().and_then(|g| {
            let stored = g.properties.get(newera_joinery::PARAMS_KEY)?;
            let before = serde_json::from_str(stored)
                .ok()
                .and_then(|b| newera_joinery::generate(&b).ok())?;
            (g.name != before.name).then(|| g.name.clone())
        });
        let mut group = newera_core::Furniture {
            level,
            name: renamed.unwrap_or_else(|| group.name.clone()),
            ..group
        };
        // Items embedded in it stay in their place on the host.
        let carried = existing
            .as_ref()
            .map(|old| newera_joinery::carry_embedded(old, &mut group))
            .unwrap_or_default();
        let command = if existing.is_some() {
            Command::update(group)
        } else {
            Command::insert(group)
        };
        doc.execute(command).map_err(core)?;
        let mut reply = summary(&group_id.to_string());
        if let Some(notes) = reply["notes"].as_array_mut() {
            notes.extend(carried.into_iter().map(serde_json::Value::String));
        }
        Ok(reply.to_string())
    }
    #[tool(
        name = "cut_list",
        description = "Cut list of joinery builds, and of groups drawn by hand: each part of a drawn group up to 50 mm thick is a board at the size it was drawn (board is its finish and thickness, no edge banding), the groups are named in drawn and the parts that are not boards in skipped [[id,name]]. Rows [part,board,qty,length,width,thickness mm,edge long+short,cutouts [x,y,w,d] mm?] merged by size, hardware, sheets per board. export what=cut_list writes it as .csv, or .dxf/.svg (boards laid out on sheets). sources names the panel standards behind the boards: MDF is a dry-process fibreboard (NBR 15316), MDP a particleboard of 551 to 750 kg/m³ that holds screws better (NBR 14810)."
    )]
    pub(crate) fn read_cut_list(
        &self,
        Parameters(p): Parameters<CutListParams>,
    ) -> Result<String, ErrorData> {
        if p.path.is_some() {
            return Err(invalid(
                "cut_list only reads; export (what=cut_list) writes the file",
            ));
        }
        self.cut_list(Parameters(p))
    }
    /// Writes the cut list to a file.
    pub(crate) fn export_cut_list(
        &self,
        Parameters(p): Parameters<CutListExportParams>,
    ) -> Result<String, ErrorData> {
        self.cut_list(Parameters(CutListParams {
            ids: p.ids,
            path: Some(p.path),
        }))
    }
    /// The cut list, written to a file when a path is given.
    pub(crate) fn cut_list(
        &self,
        Parameters(p): Parameters<CutListParams>,
    ) -> Result<String, ErrorData> {
        let doc = self.document.read();
        let home = doc.home();
        let view = home.level_view(home.current_level());
        let wanted: Option<Vec<String>> = p.ids.clone();
        let mut rows: Vec<newera_joinery::CutRow> = Vec::new();
        let mut hardware: Vec<String> = Vec::new();
        let mut builds = 0;
        for group in view.furniture.iter().filter(|f| {
            f.properties.contains_key(newera_joinery::PARAMS_KEY)
                && wanted
                    .as_ref()
                    .is_none_or(|ids| ids.contains(&f.id.to_string()))
        }) {
            let build: newera_joinery::Build =
                serde_json::from_str(&group.properties[newera_joinery::PARAMS_KEY])
                    .map_err(|e| invalid(format!("{}: {e}", group.id)))?;
            let output = newera_joinery::generate(&build).map_err(invalid)?;
            builds += 1;
            hardware.extend(output.hardware.iter().map(|h| format!("{}: {h}", group.id)));
            for row in newera_joinery::cut_list(&output) {
                match rows.iter_mut().find(|r| {
                    r.board == row.board
                        && r.edge == row.edge
                        && r.holes == row.holes
                        && r.size
                            .iter()
                            .zip(row.size)
                            .all(|(a, b)| (a - b).abs() < 0.05)
                }) {
                    Some(existing) => existing.qty += row.qty,
                    None => rows.push(row),
                }
            }
        }
        // Joinery drawn by hand — a group of solids, boards and all — is cut
        // from the same sheets: every part thin enough to be a board is one,
        // with the size it was drawn at. What is not a board is named, not
        // guessed into a row.
        let mut drawn: Vec<String> = Vec::new();
        let mut skipped: Vec<serde_json::Value> = Vec::new();
        for group in view.furniture.iter().filter(|f| {
            f.is_group()
                && !f.properties.contains_key(newera_joinery::PARAMS_KEY)
                && wanted
                    .as_ref()
                    .is_none_or(|ids| ids.contains(&f.id.to_string()))
        }) {
            let mut boards = 0;
            let mut left = Vec::new();
            for part in group.visible_leaves() {
                let mut size =
                    [part.width, part.depth, part.height].map(|cm| (cm * 100.0).round() / 10.0);
                size.sort_by(|a, b| b.total_cmp(a));
                if part.model.is_some() || size[2] > DRAWN_BOARD_MM || size[2] <= 0.0 {
                    left.push(serde_json::json!([part.id.to_string(), part.name]));
                    continue;
                }
                boards += 1;
                let board = part.texture.as_ref().map_or_else(
                    || format!("desenhado {} mm", size[2]),
                    |mat| format!("{mat} {} mm", size[2]),
                );
                let row = newera_joinery::CutRow {
                    name: part.name.clone(),
                    board,
                    qty: 1,
                    size,
                    edge: [0, 0],
                    holes: Vec::new(),
                };
                match rows.iter_mut().find(|r| {
                    r.board == row.board
                        && r.name == row.name
                        && r.size
                            .iter()
                            .zip(row.size)
                            .all(|(a, b)| (a - b).abs() < 0.05)
                }) {
                    Some(existing) => existing.qty += 1,
                    None => rows.push(row),
                }
            }
            if boards > 0 {
                drawn.push(group.id.to_string());
                skipped.extend(left);
            }
        }
        if builds == 0 && drawn.is_empty() {
            return Err(invalid(
                "no joinery here: no joinery builds (make one with the joinery tool) and no group drawn with board parts",
            ));
        }
        // Where the project stands, before the document is let go: the panel
        // standards oblige here and inform elsewhere, like every other source.
        let place = doc.home().compass.place();
        drop(doc);
        let mut sheets = Vec::new();
        if let Some(path) = &p.path {
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase);
            let layout = match ext.as_deref() {
                Some("dxf") => Some(newera_joinery::cut_list_dxf(&rows)),
                Some("svg") => Some(newera_joinery::cut_list_svg(&rows)),
                Some("csv") => None,
                _ => return Err(invalid("path must end in .csv, .dxf or .svg")),
            };
            let bytes = match layout {
                Some((drawing, used)) => {
                    sheets = used;
                    drawing
                }
                None => newera_joinery::cut_list_csv(&rows),
            };
            std::fs::write(path, bytes).map_err(|e| invalid(format!("{path}: {e}")))?;
        }
        let rows: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                let mut row = serde_json::json!([
                    r.name,
                    r.board,
                    r.qty,
                    r.size[0],
                    r.size[1],
                    r.size[2],
                    format!("{}+{}", r.edge[0], r.edge[1])
                ]);
                // Cutouts `[x, y, w, d]` mm, for stone tops.
                if !r.holes.is_empty()
                    && let Some(cells) = row.as_array_mut()
                {
                    cells.push(serde_json::json!(r.holes));
                }
                row
            })
            .collect();
        let mut reply = serde_json::json!({
            "rows": rows,
            "hardware": hardware,
            "sources": super::sources(&["nbr14810", "nbr15316"], &place),
        });
        if !sheets.is_empty() {
            reply["sheets"] = serde_json::json!(sheets);
        }
        if !drawn.is_empty() {
            reply["drawn"] = serde_json::json!(drawn);
        }
        if !skipped.is_empty() {
            reply["skipped"] = serde_json::json!(skipped);
        }
        Ok(reply.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::CreateParams;
    use crate::tools::server;

    #[test]
    fn a_change_after_update_resized_a_build_keeps_its_new_size() {
        let s = server();
        let joinery = |json: &str| -> serde_json::Value {
            let p: JoineryParams = serde_json::from_str(json).unwrap();
            serde_json::from_str(&s.joinery(Parameters(p)).unwrap()).unwrap()
        };
        let made = joinery(r#"{"kind":"cabinet","p":{"w":60,"d":35,"h":80},"at":[100,100]}"#);
        let id = made["id"].as_str().unwrap().to_owned();
        let update: crate::tools::elements::UpdateParams =
            serde_json::from_str(&format!(r#"{{"items":[{{"id":"{id}","d":60,"h":45}}]}}"#))
                .unwrap();
        s.update(Parameters(update)).unwrap();
        // Only the color changes: the size update gave it stays.
        let changed = joinery(&format!(r#"{{"id":"{id}","p":{{"color":[200,180,150]}}}}"#));
        assert_eq!(
            changed["size"],
            serde_json::json!([60, 60, 45]),
            "{changed}"
        );
        assert!(
            changed["notes"].to_string().contains("Medidas atuais"),
            "{changed}"
        );
        // A size given in the change wins over the one the piece has.
        let shallower = joinery(&format!(r#"{{"id":"{id}","p":{{"d":40}}}}"#));
        assert_eq!(
            shallower["size"],
            serde_json::json!([60, 40, 45]),
            "{shallower}"
        );
    }
    #[test]
    fn a_group_drawn_by_hand_is_cut_from_its_boards() {
        let s = server();
        let board = |id: u64, name: &str, w: f64, d: f64, h: f64| newera_core::Furniture {
            id: newera_core::FurnitureId(id),
            catalog: "box".into(),
            name: name.to_owned(),
            position: newera_core::Point2::new(630.0, 450.0),
            width: w,
            depth: d,
            height: h,
            ..newera_core::Furniture::default()
        };
        {
            let mut doc = s.document.write();
            let mut cupboard = board(1, "vassoureiro", 30.0, 60.0, 280.0);
            let mut front = board(4, "frente rebaixada", 30.0, 1.8, 276.0);
            front.texture = Some(newera_core::Material {
                color: Some([180, 140, 100]),
                ..newera_core::Material::default()
            });
            let mut handle = board(5, "puxador de latão", 2.0, 3.0, 40.0);
            handle.model = Some("puxador.obj".into());
            cupboard.children = vec![
                board(2, "lateral", 1.8, 60.0, 280.0),
                board(3, "lateral", 1.8, 60.0, 280.0),
                front,
                handle,
                board(6, "moldura 3D", 30.0, 12.0, 8.0),
            ];
            doc.execute(newera_core::Command::insert(cupboard)).unwrap();
        }
        let reply: serde_json::Value = serde_json::from_str(
            &s.cut_list(Parameters(serde_json::from_str("{}").unwrap()))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(reply["drawn"], serde_json::json!(["f1"]), "{reply}");
        let rows = reply["rows"].as_array().unwrap();
        let side = rows
            .iter()
            .find(|r| r[0] == "lateral")
            .unwrap_or_else(|| panic!("{reply}"));
        assert_eq!(side[2], 2, "the two sides are one row: {reply}");
        assert_eq!(
            [&side[3], &side[4], &side[5]],
            [
                &serde_json::json!(2800.0),
                &serde_json::json!(600.0),
                &serde_json::json!(18.0)
            ],
            "{reply}"
        );
        assert!(rows.iter().any(|r| r[0] == "frente rebaixada"), "{reply}");
        let skipped = reply["skipped"].to_string();
        assert!(
            skipped.contains("puxador de latão") && skipped.contains("moldura 3D"),
            "a model and a 12 cm block are not boards: {reply}"
        );
    }

    #[test]
    fn explicit_front_selects_the_wall_face_for_joinery_and_catalog_pieces() {
        for angle in [90.0_f64, 270.0, 240.0] {
            let s = server();
            s.create(Parameters(
                serde_json::from_value(serde_json::json!({
                    "walls": [{"pts": [[880,0],[880,470]], "t":12}]
                }))
                .unwrap(),
            ))
            .unwrap();
            s.joinery(Parameters(
                serde_json::from_value(serde_json::json!({
                    "kind":"slats", "p":{"w":220,"h":260},
                    "wall":"w1", "along":150, "angle":angle
                }))
                .unwrap(),
            ))
            .unwrap();
            {
                let mut doc = s.document.write();
                crate::edit::place(
                    &mut doc,
                    vec![crate::edit::PlaceSpec {
                        cat: "wardrobe".into(),
                        wall: Some("w1".into()),
                        along: Some(350.0),
                        angle: Some(angle),
                        ..crate::edit::PlaceSpec::default()
                    }],
                )
                .unwrap();
            }
            let doc = s.document.read();
            for piece in &doc.home().furniture {
                let (lo, hi) = newera_core::plan_bounds(piece);
                let face = if angle > 180.0 { lo.x } else { hi.x };
                let expected = if angle > 180.0 { 886.0 } else { 874.0 };
                assert!(
                    (face - expected).abs() < 0.01,
                    "angle={angle}: {lo:?} {hi:?}"
                );
                assert!((piece.angle - angle).abs() < 0.01);
            }
        }
    }

    #[test]
    fn a_build_made_shallower_keeps_its_back_on_the_wall() {
        let s = server();
        s.create(Parameters(
            serde_json::from_str(
                r#"{"walls":[{"pts":[[0,0],[500,0],[500,400],[0,400]],"closed":true}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let joinery = |json: &str| {
            s.joinery(Parameters(serde_json::from_str(json).unwrap()))
                .unwrap()
        };
        let reply: serde_json::Value = serde_json::from_str(&joinery(
            r#"{"kind":"cabinet","p":{"w":80,"h":90,"d":57},"wall":"w1"}"#,
        ))
        .unwrap();
        let id = reply["id"].as_str().unwrap().to_owned();
        let bounds = || {
            let doc = s.document.read();
            let group = doc
                .home()
                .furniture
                .iter()
                .find(|f| f.id.to_string() == id)
                .unwrap()
                .clone();
            newera_core::plan_bounds(&group)
        };
        let (back, _) = bounds();
        assert!((back.y - 7.5).abs() < 0.5, "on the wall: {back:?}");

        joinery(&format!(r#"{{"id":"{id}","p":{{"d":55}}}}"#));
        let (min, max) = bounds();
        assert!(
            (min.y - back.y).abs() < 0.01,
            "the back did not leave the wall: {min:?}"
        );
        assert!((max.y - min.y - 55.0).abs() < 0.5, "{min:?} {max:?}");

        // Told to, the front holds instead.
        joinery(&format!(
            r#"{{"id":"{id}","p":{{"d":45}},"anchor":"front"}}"#
        ));
        let (min2, max2) = bounds();
        assert!((max2.y - max.y).abs() < 0.01, "{max:?} {max2:?}");
        assert!((min2.y - min.y - 10.0).abs() < 0.5, "{min2:?}");
    }

    #[test]
    fn joinery_builds_change_and_list_their_cuts() {
        let s = server();
        let params: CreateParams = serde_json::from_str(
            r#"{"walls":[{"pts":[[0,0],[500,0],[500,400],[0,400]],"closed":true}],"rooms":[{"name":"Sala","at":[250,200]}]}"#,
        )
        .unwrap();
        s.create(Parameters(params)).unwrap();
        let joinery = |json: &str| {
            let p: JoineryParams = serde_json::from_str(json).unwrap();
            s.joinery(Parameters(p))
        };
        let reply: serde_json::Value = serde_json::from_str(
            &joinery(r#"{"kind":"cabinet","p":{"w":120,"h":210,"d":55},"wall":"w1"}"#).unwrap(),
        )
        .unwrap();
        let id = reply["id"].as_str().unwrap().to_owned();
        assert!(
            reply["hardware"].to_string().contains("dobradiças"),
            "{reply}"
        );
        {
            let doc = s.document.read();
            let group = doc
                .home()
                .furniture
                .iter()
                .find(|f| f.id.to_string() == id)
                .unwrap();
            // Backed onto the top wall: its back half a wall thickness below y = 0.
            assert!(
                (group.position.y - (7.5 + 27.5)).abs() < 0.5,
                "{:?}",
                group.position
            );
            assert!(group.children.len() > 10);
        }
        // Change only the shelves; the rest of the build and its place stay.
        let changed: serde_json::Value = serde_json::from_str(
            &joinery(&format!(r#"{{"id":"{id}","p":{{"shelves":5}}}}"#)).unwrap(),
        )
        .unwrap();
        assert_eq!(changed["id"], id.as_str());
        assert!(
            changed["parts"].as_u64() > reply["parts"].as_u64(),
            "{changed}"
        );
        // A request the workshop would argue with is built, and argued with
        // in the notes; dry runs still create nothing.
        let shallow =
            joinery(r#"{"kind":"cabinet","p":{"d":35,"cooktop":true},"dry":true}"#).unwrap();
        assert!(shallow.contains("cooktop"), "{shallow}");
        // What has no geometry at all still fails, and says why.
        let flat = joinery(r#"{"kind":"cabinet","p":{"w":2}}"#).unwrap_err();
        assert!(flat.message.contains("lado de dentro"), "{}", flat.message);
        let before = s.document.read().home().furniture.len();
        joinery(r#"{"kind":"slats","p":{"w":100},"dry":true}"#).unwrap();
        assert_eq!(s.document.read().home().furniture.len(), before);
        // A cove follows the room given by id.
        let cove: serde_json::Value =
            serde_json::from_str(&joinery(r#"{"kind":"cove","room":"r5"}"#).unwrap()).unwrap();
        assert!(
            cove["hardware"].to_string().contains("fita de LED"),
            "{cove}"
        );
        assert!(joinery(r#"{"id":"w1","p":{}}"#).is_err());

        let dir = std::env::temp_dir().join(format!("newera-cut-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let csv = dir.join("corte.csv");
        let list: serde_json::Value = serde_json::from_str(
            &s.cut_list(Parameters(CutListParams {
                ids: None,
                path: Some(csv.display().to_string()),
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(
            list["rows"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r[0] == "Lateral esquerda" && r[2] == 2),
            "{list}"
        );
        assert!(
            std::fs::read_to_string(&csv)
                .unwrap()
                .starts_with("peca;material")
        );
        // The boards say which standard defines them, resolved once.
        assert_eq!(list["sources"]["nbr15316"][1], "A", "{list}");
        assert!(
            list["sources"]["nbr14810"][0]
                .as_str()
                .unwrap()
                .contains("14810"),
            "{list}"
        );
        let dxf = dir.join("corte.dxf");
        let with_sheets: serde_json::Value = serde_json::from_str(
            &s.cut_list(Parameters(CutListParams {
                ids: Some(vec![id]),
                path: Some(dxf.display().to_string()),
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(with_sheets["sheets"].is_array(), "{with_sheets}");
        assert!(std::fs::read_to_string(&dxf).unwrap().contains("ENTITIES"));
        let svg = dir.join("corte.svg");
        s.cut_list(Parameters(CutListParams {
            ids: None,
            path: Some(svg.display().to_string()),
        }))
        .unwrap();
        assert!(std::fs::read_to_string(&svg).unwrap().starts_with("<svg"));
        std::fs::remove_dir_all(dir).ok();
    }
}
