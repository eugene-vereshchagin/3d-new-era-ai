//! Drawing and changing what the plan is made of: walls, rooms, dimensions,
//! labels, polylines and solids.
//!
//! Thin adapters over [`crate::edit`], which holds the logic free of
//! protocol types so it can be tested on its own.

use newera_core::{Command, ops};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{ErrorData, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::NewEraMcp;
use super::reply::{
    self, Dry, Raw, applied, background_scale, core, forward, invalid, ok, on_variant, take_action,
};
use crate::edit::{self, CreateParams, UpdateSpec};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct UpdateParams {
    /// The elements to change, one object each: `id` and the fields to set.
    #[serde(default)]
    pub(crate) items: Vec<UpdateSpec>,
    /// Rename in bulk by a regex, in one step.
    #[serde(default)]
    pub(crate) rename: Option<edit::RenameSpec>,
    /// `variants` index, 0-based (not a revision); made active.
    pub(crate) v: Option<usize>,
    /// Answer what it would change, without writing.
    pub(crate) dry: Option<Dry>,
}
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct IdsParams {
    /// The walls to join, in order along the line they make.
    ids: Vec<String>,
}
/// What `edit_walls` takes; each action goes on to its own parameters.
#[derive(JsonSchema)]
#[allow(dead_code)] // a schema, never built
pub(crate) struct WallsSchema {
    /// What to do with the walls.
    #[schemars(extend("enum" = ["split", "merge"]))]
    action: String,
    /// For `split`: the wall to split.
    id: Option<String>,
    /// For `split`: where along the wall, 0..1 (default 0.5).
    t: Option<f64>,
    /// For `merge`: the walls to join, in order along the line they make.
    ids: Option<Vec<String>>,
}
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DeleteParams {
    /// The elements to delete: walls, rooms, pieces, dimensions, labels, lines.
    ids: Vec<String>,
    /// Also delete the labels left pointing at the deleted pieces.
    labels: Option<bool>,
}
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct MoveParams {
    /// The elements to move together.
    ids: Vec<String>,
    /// Shift in x, cm (with `dy`).
    #[serde(default)]
    dx: f64,
    /// Shift in y, cm (with `dx`).
    #[serde(default)]
    dy: f64,
    /// Instead of dx/dy: where the first id goes, [x,y] as reads give it
    /// (a piece's or a label's center); the others keep their offsets.
    to: Option<newera_core::Point2>,
    /// Drag endpoints of walls joined to moved walls (default true).
    joined: Option<bool>,
    /// Try it without applying; see `update`.
    dry: Option<Dry>,
}
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SplitParams {
    /// The wall to split.
    id: String,
    /// Split position along the wall, 0..1 (default 0.5).
    t: Option<f64>,
}
/// The ids asked for whose element came out exactly as it was.
///
/// `hinge_right: true` on a door already hinged right answered `ok` and a dry
/// run `{}` — which read as "nothing to change here" when the request had
/// simply asked for what was already there. Named, with the current values,
/// it is a sentence instead of a silence.
fn unchanged(
    before: &newera_core::Home,
    after: &newera_core::Home,
    asked: &[String],
) -> Vec<serde_json::Value> {
    let diff = crate::compact::diff(before, after);
    let moved: std::collections::BTreeSet<String> = ["changed", "added", "gone"]
        .iter()
        .filter_map(|k| diff.get(*k).and_then(|v| v.as_array()))
        .flatten()
        .filter_map(|c| c.as_str().or_else(|| c["id"].as_str()).map(str::to_owned))
        .collect();
    // The compact form leaves out what it does not draw (a piece's
    // properties, like `piece:fixed`): the element itself says if it changed.
    let full = |home: &newera_core::Home, id: newera_core::ElementId| match id {
        newera_core::ElementId::Furniture(f) => home
            .find_piece(f)
            .cloned()
            .map(newera_core::Element::Furniture),
        _ => home.element(id),
    };
    asked
        .iter()
        .filter(|id| !moved.contains(*id))
        .filter_map(|raw| {
            let id: newera_core::ElementId = raw.parse().ok()?;
            if full(before, id) != full(after, id) {
                return None;
            }
            let now = crate::compact::element(after, id)?;
            Some(serde_json::json!({"id": raw, "now": now}))
        })
        .collect()
}

/// Elevations asked for that a piece's mount set aside: a downlight given
/// `elev` goes back onto the ceiling. `asked` is `(id, elev)` per request.
///
/// The write did nothing to the piece, and was answered as if the piece
/// already sat at the height asked for — when it sat 27 cm above it. Each
/// one is named with where it ended and why.
pub(super) fn overridden(
    home: &newera_core::Home,
    asked: &[(String, f64)],
) -> Vec<serde_json::Value> {
    use newera_core::mounting::{Mount, mount_of, on_ceiling};
    asked
        .iter()
        .filter_map(|(raw, elev)| {
            let newera_core::ElementId::Furniture(id) = raw.parse().ok()? else {
                return None;
            };
            let piece = home.find_piece(id)?;
            if (piece.elevation - elev).abs() < 0.05 {
                return None;
            }
            let why = if on_ceiling(piece) {
                "snapped to the ceiling"
            } else if mount_of(&piece.catalog) == Some(Mount::Floor) {
                "kept flush with the floor"
            } else {
                "set by what it is mounted on"
            };
            let (given, now) = (
                crate::compact::num(*elev),
                crate::compact::num(piece.elevation),
            );
            Some(serde_json::json!({
                "id": raw,
                "elev": now,
                "note": format!("elev {given} {why} at {now}"),
            }))
        })
        .collect()
}

/// The elevations an update asks for, by id.
fn asked_elevs(items: &[UpdateSpec]) -> Vec<(String, f64)> {
    items
        .iter()
        .filter_map(|i| Some((i.id.clone(), i.elev?)))
        .collect()
}

/// A reply with `unchanged` added: the requests that were already so; and
/// `overridden`: the values asked for that the piece did not keep.
fn with_unchanged(
    reply: &str,
    still: &[serde_json::Value],
    snapped: &[serde_json::Value],
) -> String {
    let set_aside: std::collections::BTreeSet<&str> =
        snapped.iter().filter_map(|s| s["id"].as_str()).collect();
    let still: Vec<&serde_json::Value> = still
        .iter()
        .filter(|s| !s["id"].as_str().is_some_and(|id| set_aside.contains(id)))
        .collect();
    if still.is_empty() && snapped.is_empty() {
        return reply.to_owned();
    }
    let note = "these already had the values asked for; nothing was changed on them";
    let mut extra = serde_json::Map::new();
    if !still.is_empty() {
        extra.insert("unchanged".to_owned(), serde_json::json!(still));
        extra.insert("unchanged_note".to_owned(), serde_json::json!(note));
    }
    if !snapped.is_empty() {
        extra.insert("overridden".to_owned(), serde_json::json!(snapped));
    }
    match reply.find('{') {
        Some(at) => {
            let (head, body) = reply.split_at(at);
            let mut json: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
            if let Some(object) = json.as_object_mut() {
                object.extend(extra);
            }
            format!("{head}{json}")
        }
        None => format!("{reply} {}", serde_json::Value::Object(extra)),
    }
}

#[tool_router(router = elements_router, vis = "pub(crate)")]
impl NewEraMcp {
    #[tool(
        description = "Create walls (polylines; hs = height per point for gables), rooms (pts, or at=[x,y] to detect from walls; ceiling_flat=false follows wall profiles instead of the default flat ceiling), dims (a+b or wall id), labels, roofs (rectangle pts, gable|shed, pitch or ridge_h, eave h, overhang, gables=true closes the ends, skylights [{at,w,d}] cut glazed openings) and solids (pts outline raised by h at elev: slabs/mezzanines of any shape; or profile [[u,z]] swept from a to b: gables, ramps) in one atomic step."
    )]
    pub(crate) fn create(
        &self,
        Parameters(mut p): Parameters<CreateParams>,
    ) -> Result<String, ErrorData> {
        let mut doc = self.document.write();
        on_variant(&mut doc, p.v)?;
        if p.px {
            let bg = background_scale(&doc)?;
            p.map_points(&|q| bg.point(q));
        }
        let created = edit::create(&mut doc, p).map_err(invalid)?;
        let mut reply = ok(&doc, &created.ids);
        if !created.welded.is_empty() {
            reply.push_str(" welded=");
            reply.push_str(&created.welded.join(","));
        }
        if !created.short.is_empty() {
            reply.push_str(" short=");
            reply.push_str(&created.short.join(","));
            reply.push_str(" (shorter than thick: coordinates are cm, metres ×100?)");
        }
        Ok(reply)
    }
    #[tool(
        description = "Change fields of elements by id; each field applies only to the kinds that have it (furniture mat/opacity/pitch, wall h_end, room auto/ceiling_flat, polyline divider). A part of a group takes name, brand, model_name and url on its own; its size and place belong to the group. anchor on a resize holds one face still instead of growing around the center; stretch=[part ids] on a group resize grows only those parts and moves the rest along (uprights keep 5.8 cm while the opening grows). dry=true answers what it would do without writing: changed fields, clearances around each piece it touches (negative: cm it would sit inside what it faces), issues_resolved/issues_new [{ids, kind, extent|cm|over}] with the kinds `layout` gives, and issues_changed (extent_was); dry=\"summary\" leaves out the parts a group rebuilds. Otherwise the reply names what changed. rename {pattern, to, what} renames in bulk by a regex (Rust syntax, `(?i)` for any case, `$1` in to) in one step; with dry it lists them first."
    )]
    pub(crate) fn update(
        &self,
        Parameters(p): Parameters<UpdateParams>,
    ) -> Result<String, ErrorData> {
        if p.items.is_empty() && p.rename.is_none() {
            return Err(invalid(
                "nothing to change: give items, or rename {pattern, to}",
            ));
        }
        let apply = |doc: &mut newera_core::Document,
                     items: Vec<UpdateSpec>,
                     rename: Option<&edit::RenameSpec>|
         -> Result<edit::Updated, ErrorData> {
            if let Some(rule) = rename {
                edit::rename(doc, rule).map_err(invalid)?;
            }
            if items.is_empty() {
                return Ok(edit::Updated::default());
            }
            edit::update(doc, items).map_err(invalid)
        };
        let asked: Vec<String> = p.items.iter().map(|i| i.id.clone()).collect();
        let elevs = asked_elevs(&p.items);
        if Dry::on(p.dry.as_ref()) {
            let doc = self.document.read();
            let before = doc.home().clone();
            let (items, rename) = (p.items, p.rename);
            let mut scratch = newera_core::Document::new(before.clone());
            apply(&mut scratch, items.clone(), rename.as_ref())?;
            let still = unchanged(&before, scratch.home(), &asked);
            let snapped = overridden(scratch.home(), &elevs);
            let answer = reply::preview_with(&doc, Dry::brief(p.dry.as_ref()), move |scratch| {
                apply(scratch, items, rename.as_ref()).map(|_| ())
            })?;
            return Ok(with_unchanged(&answer, &still, &snapped));
        }
        let mut doc = self.document.write();
        on_variant(&mut doc, p.v)?;
        let before = doc.home().clone();
        let updated = apply(&mut doc, p.items, p.rename.as_ref())?;
        let still = unchanged(&before, doc.home(), &asked);
        let snapped = overridden(doc.home(), &elevs);
        let mut reply = with_unchanged(&applied(&doc, &before), &still, &snapped);
        if !updated.welded.is_empty() {
            reply.push_str(" welded=");
            reply.push_str(&updated.welded.join(","));
        }
        if !updated.left.is_empty() {
            reply.push_str(" openings_left=");
            reply.push_str(&updated.left.join("; "));
        }
        Ok(reply)
    }
    #[tool(
        description = "Delete (remove) walls, rooms, pieces or any element by id, atomically. Labels left pointing at a deleted piece — about it, or standing on it — are named in the reply as labels_left [[id, text]], since an index code over what is now another piece is found by nobody; labels=true deletes them in the same step."
    )]
    pub(crate) fn delete(
        &self,
        Parameters(p): Parameters<DeleteParams>,
    ) -> Result<String, ErrorData> {
        let ids = edit::parse_ids(&p.ids).map_err(invalid)?;
        let mut doc = self.document.write();
        let home = doc.home();
        let gone: Vec<&newera_core::Furniture> = ids
            .iter()
            .filter_map(|id| match id {
                newera_core::ElementId::Furniture(f) => home.find_piece(*f),
                _ => None,
            })
            .flat_map(newera_core::Furniture::flatten)
            .collect();
        let left: Vec<(newera_core::LabelId, String)> = home
            .labels
            .iter()
            .filter(|l| !ids.contains(&l.id.into()))
            .filter(|l| {
                gone.iter().any(|f| {
                    if let Some(about) = l.about {
                        return about == f.id;
                    }
                    let (min, max) = newera_core::plan_bounds(f);
                    home.on_level(l.level, f.level)
                        && (min.x..=max.x).contains(&l.position.x)
                        && (min.y..=max.y).contains(&l.position.y)
                })
            })
            .map(|l| (l.id, l.text.clone()))
            .collect();
        let also = p.labels.unwrap_or(false);
        let commands = ids
            .into_iter()
            .chain(
                left.iter()
                    .filter(|_| also)
                    .map(|(id, _)| newera_core::ElementId::from(*id)),
            )
            .map(Command::remove)
            .collect();
        doc.execute(Command::Batch { commands }).map_err(core)?;
        let reply = ok(&doc, &[]);
        Ok(match (left.is_empty(), also) {
            (true, _) => reply,
            (false, true) => format!(
                "{reply} {}",
                serde_json::json!({"labels_deleted": left.iter().map(|(id, _)| id.to_string()).collect::<Vec<_>>()})
            ),
            (false, false) => format!(
                "{reply} {}",
                serde_json::json!({"labels_left": left.iter().map(|(id, text)| serde_json::json!([id.to_string(), text])).collect::<Vec<_>>()})
            ),
        })
    }
    #[tool(
        name = "move",
        description = "Move elements by dx,dy cm, or to=[x,y]: the first id (a piece or a label) goes there and the rest keep their offsets. dry=true answers what it would do without writing anything, dry=\"summary\" answers it short; see `update`."
    )]
    pub(crate) fn move_elements(
        &self,
        Parameters(p): Parameters<MoveParams>,
    ) -> Result<String, ErrorData> {
        let ids = edit::parse_ids(&p.ids).map_err(invalid)?;
        let joined = p.joined.unwrap_or(true);
        let (dx, dy) = match p.to {
            None => (p.dx, p.dy),
            Some(to) => {
                let doc = self.document.read();
                let home = doc.home();
                let from = match ids.first() {
                    Some(newera_core::ElementId::Furniture(id)) => {
                        home.find_piece(*id).map(|f| f.position)
                    }
                    Some(newera_core::ElementId::Label(id)) => {
                        home.labels.iter().find(|l| l.id == *id).map(|l| l.position)
                    }
                    _ => None,
                }
                .ok_or_else(|| {
                    invalid("move takes dx/dy, or to=[x,y] with a piece or a label as the first id")
                })?;
                (to.x - from.x, to.y - from.y)
            }
        };
        if Dry::on(p.dry.as_ref()) {
            let doc = self.document.read();
            return reply::preview_with(&doc, Dry::brief(p.dry.as_ref()), move |scratch| {
                ops::translate(scratch, &ids, dx, dy, joined).map_err(core)
            });
        }
        let mut doc = self.document.write();
        let before = doc.home().clone();
        ops::translate(&mut doc, &ids, dx, dy, joined).map_err(core)?;
        Ok(applied(&doc, &before))
    }
    #[tool(
        description = "Split a wall in two, or join walls into one. split {id, t?} cuts a wall into two joined walls at t (0..1, default 0.5). merge {ids} joins walls running along one line — the wall a partition interrupted, a stretch imported as many segments, the same wall drawn twice: the longest keeps its id and its build (thickness, height, type, finishes) and spans them all, the others are deleted, doors and windows stay where they are, and a gap between them is closed. The reply names the ids that result."
    )]
    pub(crate) fn edit_walls(
        &self,
        Parameters(Raw(mut args, _)): Parameters<Raw<WallsSchema>>,
    ) -> Result<String, ErrorData> {
        match take_action(&mut args, "action", &["split", "merge"], None)?.as_str() {
            "split" => self.split_wall(Parameters(forward(args)?)),
            _ => self.merge_walls(Parameters(forward(args)?)),
        }
    }
    /// Splits a wall into two joined walls at `t`.
    pub(crate) fn split_wall(
        &self,
        Parameters(p): Parameters<SplitParams>,
    ) -> Result<String, ErrorData> {
        let id = p.id.parse().map_err(|e| invalid(format!("{e}")))?;
        let mut doc = self.document.write();
        let second = ops::split_wall(&mut doc, id, p.t.unwrap_or(0.5)).map_err(core)?;
        Ok(ok(&doc, &[second.to_string()]))
    }
    /// Joins walls along one line into the longest of them.
    pub(crate) fn merge_walls(
        &self,
        Parameters(p): Parameters<IdsParams>,
    ) -> Result<String, ErrorData> {
        let ids: Vec<newera_core::WallId> = p
            .ids
            .iter()
            .map(|raw| raw.parse().map_err(|e| invalid(format!("{e}"))))
            .collect::<Result<_, _>>()?;
        let mut doc = self.document.write();
        let kept = ops::merge_walls(&mut doc, &ids).map_err(core)?;
        Ok(ok(&doc, &[kept.to_string()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::cameras::CamerasParams;
    use crate::tools::furniture::PlaceParams;
    use crate::tools::read::GetHomeParams;
    use crate::tools::server;

    #[test]
    fn move_takes_a_place_as_well_as_a_shift() {
        let s = server();
        let reply = s
            .place(Parameters(
                serde_json::from_str(
                    r#"{"items":[{"cat":"armchair","at":[100,100]},{"cat":"side-table","at":[160,100]}]}"#,
                )
                .unwrap(),
            ))
            .unwrap();
        let ids: Vec<String> = reply
            .rsplit("ids=")
            .next()
            .unwrap()
            .split(',')
            .map(|s| s.trim().to_owned())
            .collect();
        s.move_elements(Parameters(
            serde_json::from_str(&format!(
                r#"{{"ids":["{}","{}"],"to":[515,616]}}"#,
                ids[0], ids[1]
            ))
            .unwrap(),
        ))
        .unwrap();
        let home = s.document.read().home().clone();
        let chair = home
            .furniture
            .iter()
            .find(|f| f.id.to_string() == ids[0])
            .unwrap();
        let table = home
            .furniture
            .iter()
            .find(|f| f.id.to_string() == ids[1])
            .unwrap();
        assert!((chair.position.x - 515.0).abs() < 1e-9 && (chair.position.y - 616.0).abs() < 1e-9);
        assert!(
            (table.position.x - 575.0).abs() < 1e-9,
            "the rest keeps its offset"
        );
        let wall_first = s
            .move_elements(Parameters(
                serde_json::from_str(r#"{"ids":["w1"],"to":[0,0]}"#).unwrap(),
            ))
            .unwrap_err();
        assert!(wall_first.message.contains("dx/dy"), "{wall_first:?}");
    }

    #[test]
    fn a_dry_write_answers_the_question_without_touching_the_plan() {
        let s = server();
        s.create(Parameters(
            serde_json::from_str(
                r#"{"walls":[{"pts":[[0,0],[500,0],[500,400],[0,400]],"closed":true}],
                    "rooms":[{"name":"Cozinha","at":[250,200]}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        s.place(Parameters(
            serde_json::from_str(
                r#"{"items":[{"cat":"base-cabinet","at":[250,40],"w":300,"d":60,"h":90},
                             {"cat":"base-cabinet","at":[250,340],"w":300,"d":60,"h":90}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let rev = s.document.read().revision();
        let depth = s.document.read().home().furniture[0].depth;

        // "And if the counter were 100 cm deep?" — asked, not applied.
        let dry: serde_json::Value = serde_json::from_str(
            &s.update(Parameters(UpdateParams {
                items: serde_json::from_str(r#"[{"id":"f6","d":100}]"#).unwrap(),
                rename: None,
                v: None,
                dry: Some(Dry::All(true)),
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(dry["dry"], true, "{dry}");
        assert_eq!(dry["changed"][0]["id"], "f6", "{dry}");
        assert_eq!(dry["changed"][0]["to"]["wdh"][1], 100.0, "{dry}");
        // The corridor it would leave, measured on the copy. Depth grows
        // around the center, so the front only advances 20 cm — and the back
        // ends up inside the wall, which the dry run says before it happens.
        assert!(
            (dry["clearances"]["f6"]["+y"][0].as_f64().unwrap() - 220.0).abs() < 0.5,
            "{dry}"
        );
        assert_eq!(dry["issues_new"][0]["ids"], "f6+w1", "{dry}");
        assert_eq!(dry["issues_new"][0]["kind"], "in_wall", "{dry}");
        assert_eq!(
            s.document.read().revision(),
            rev,
            "a dry run writes nothing"
        );
        assert!(
            (s.document.read().home().furniture[0].depth - depth).abs() < 1e-9,
            "and changes nothing"
        );

        // Applied for real, the reply says what moved instead of only `ok`.
        let reply = s
            .update(Parameters(UpdateParams {
                items: serde_json::from_str(r#"[{"id":"f6","d":100}]"#).unwrap(),
                rename: None,
                v: None,
                dry: None,
            }))
            .unwrap();
        assert!(reply.starts_with("ok rev="), "{reply}");
        let diff: serde_json::Value =
            serde_json::from_str(&reply[reply.find('{').expect("a diff")..]).unwrap();
        assert_eq!(diff["changed"][0]["id"], "f6", "{reply}");
        assert_eq!(diff["changed"][0]["from"]["wdh"][1], 60.0, "{reply}");
    }
    #[test]
    fn a_door_nudged_along_its_wall_keeps_the_side_it_opens_to() {
        let s = server();
        s.create(Parameters(
            serde_json::from_str(r#"{"walls":[{"pts":[[300,0],[300,400]]}]}"#).unwrap(),
        ))
        .unwrap();
        // A bathroom door opening into the bathroom, on the +x side.
        s.place(Parameters(
            serde_json::from_str(
                r#"{"items":[{"cat":"door","wall":"w1","along":200,"w":70,"into":[400,200]}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let door = || {
            let doc = s.document.read();
            let f = doc.home().furniture[0].clone();
            (
                f.id.to_string(),
                f.angle,
                newera_core::facing(&f).to_owned(),
                newera_core::door_swing(&f).unwrap(),
            )
        };
        let (id, angle, faces, _) = door();
        let swings_into_bathroom =
            |swing: &[newera_core::Point2]| swing.iter().all(|p| p.x >= 299.0);
        assert!(
            swings_into_bathroom(&door().3),
            "placed swinging into the bathroom"
        );

        s.move_elements(Parameters(
            serde_json::from_str(&format!(r#"{{"ids":["{id}"],"dx":0,"dy":-4}}"#)).unwrap(),
        ))
        .unwrap();
        let (_, moved_angle, moved_faces, swing) = door();
        assert!(
            (moved_angle - angle).abs() < 1e-6,
            "{angle} → {moved_angle}"
        );
        assert_eq!(moved_faces, faces);
        assert!(
            swings_into_bathroom(&swing),
            "still swings into the bathroom: {swing:?}"
        );

        // A flip of the hinge is a change a dry run names, not `{}`.
        let right = !s.document.read().home().furniture[0]
            .opening
            .as_ref()
            .unwrap()
            .hinge_right;
        let dry: serde_json::Value = serde_json::from_str(
            &s.update(Parameters(UpdateParams {
                items: serde_json::from_str(&format!(r#"[{{"id":"{id}","hinge_right":{right}}}]"#))
                    .unwrap(),
                rename: None,
                v: None,
                dry: Some(Dry::All(true)),
            }))
            .unwrap(),
        )
        .unwrap();
        let changed = &dry["changed"][0];
        assert_eq!(changed["id"], id.as_str(), "{dry}");
        let (from, to) = (
            &changed["from"]["hinge_right"],
            &changed["to"]["hinge_right"],
        );
        assert_ne!(from, to, "the hinge is named as what moved: {dry}");
        assert!(
            from.is_boolean() && to.is_boolean(),
            "both sides as booleans, never null: {dry}"
        );

        // Asking for the hinge it already has is said, not answered with a
        // silent ok or an empty dry run.
        let current = !right;
        let same = format!(r#"[{{"id":"{id}","hinge_right":{current}}}]"#);
        let dry: serde_json::Value = serde_json::from_str(
            &s.update(Parameters(UpdateParams {
                items: serde_json::from_str(&same).unwrap(),
                rename: None,
                v: None,
                dry: Some(Dry::All(true)),
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(dry["unchanged"][0]["id"], id.as_str(), "{dry}");
        let applied = s
            .update(Parameters(UpdateParams {
                items: serde_json::from_str(&same).unwrap(),
                rename: None,
                v: None,
                dry: None,
            }))
            .unwrap();
        assert!(
            applied.contains("unchanged") && applied.contains("already had"),
            "{applied}"
        );
    }

    #[test]
    fn declaring_a_piece_fixed_is_said_as_a_change_and_can_be_taken_back() {
        let s = server();
        s.place(Parameters(
            serde_json::from_str(r#"{"items":[{"cat":"chair","at":[100,100]}]}"#).unwrap(),
        ))
        .unwrap();
        let id = s.document.read().home().furniture[0].id.to_string();
        let set = |fixed: &str| {
            s.update(Parameters(UpdateParams {
                items: serde_json::from_str(&format!(r#"[{{"id":"{id}","fixed":{fixed}}}]"#))
                    .unwrap(),
                rename: None,
                v: None,
                dry: None,
            }))
        };
        let prop = || {
            s.document.read().home().furniture[0]
                .properties
                .get(newera_core::mounting::FIXED_KEY)
                .cloned()
        };
        let reply = set("true").unwrap();
        assert!(
            !reply.contains("unchanged") && reply.contains("\"fixed\""),
            "{reply}"
        );
        assert_eq!(prop().as_deref(), Some("true"));
        let reply = set("true").unwrap();
        assert!(reply.contains("unchanged"), "asked again: {reply}");
        let reply = set(r#""""#).unwrap();
        assert!(!reply.contains("unchanged"), "{reply}");
        assert_eq!(prop(), None, "cleared");
        assert!(set(r#""maybe""#).is_err());
    }

    #[test]
    fn a_group_resize_says_what_stretches() {
        let s = server();
        let part = |id: u64, x0: f64, x1: f64| newera_core::Furniture {
            id: newera_core::FurnitureId(id),
            catalog: "box".into(),
            name: format!("parte {id}"),
            position: newera_core::Point2::new(100.0 + x0.midpoint(x1), 30.0),
            width: x1 - x0,
            depth: 60.0,
            height: 90.0,
            ..newera_core::Furniture::default()
        };
        {
            let mut doc = s.document.write();
            let mut group = part(1, -90.0, 90.0);
            group.name = "península".into();
            group.children = vec![
                part(2, -90.0, -84.2),
                part(3, -84.2, 25.8),
                part(4, 25.8, 90.0),
            ];
            doc.execute(newera_core::Command::insert(group)).unwrap();
        }
        let width = |id: &str| {
            s.document
                .read()
                .home()
                .find_piece(id.parse().unwrap())
                .unwrap()
                .width
        };
        s.update(Parameters(UpdateParams {
            items: serde_json::from_str(r#"[{"id":"f1","w":131,"anchor":"-x","stretch":["f4"]}]"#)
                .unwrap(),
            rename: None,
            v: None,
            dry: None,
        }))
        .unwrap();
        assert!(
            (width("f2") - 5.8).abs() < 1e-6,
            "the upright keeps its thickness"
        );
        assert!(
            (width("f3") - 110.0).abs() < 1e-6,
            "the table keeps its size"
        );
        assert!(
            (width("f4") - 15.2).abs() < 1e-6,
            "the filler takes the change"
        );
        let doc = s.document.read();
        let upright = doc.home().find_piece("f2".parse().unwrap()).unwrap();
        assert!(
            (upright.position.x - 12.9).abs() < 1e-6,
            "anchored at its left face, the upright did not move: {}",
            upright.position.x
        );
        drop(doc);
        let err = s
            .update(Parameters(UpdateParams {
                items: serde_json::from_str(r#"[{"id":"f1","d":80,"stretch":["f9"]}]"#).unwrap(),
                rename: None,
                v: None,
                dry: None,
            }))
            .unwrap_err();
        assert!(err.message.contains("not a part"), "{err:?}");
    }

    #[test]
    fn a_part_of_a_group_can_be_renamed_but_not_resized_alone() {
        let s = server();
        let part = |id: u64, name: &str, x: f64, w: f64| newera_core::Furniture {
            id: newera_core::FurnitureId(id),
            catalog: "box".into(),
            name: name.to_owned(),
            position: newera_core::Point2::new(x, 30.0),
            width: w,
            depth: 30.0,
            height: 3.0,
            elevation: 75.0,
            ..newera_core::Furniture::default()
        };
        {
            let mut doc = s.document.write();
            let mut group = part(1, "mesa basculante", 60.0, 119.0);
            group.height = 78.0;
            group.elevation = 0.0;
            group.children = vec![
                part(2, "tampo aberto 110 × 30", 60.0, 119.0),
                part(3, "montante 5,8", 3.0, 5.8),
            ];
            doc.execute(newera_core::Command::insert(group)).unwrap();
        }
        let update = |json: &str| {
            s.update(Parameters(UpdateParams {
                items: serde_json::from_str(json).unwrap(),
                rename: None,
                v: None,
                dry: None,
            }))
        };
        update(r#"[{"id":"f2","name":"tampo aberto 119 × 30"},{"id":"f3","brand":"Blum"}]"#)
            .unwrap();
        {
            let doc = s.document.read();
            let home = doc.home();
            assert_eq!(
                home.find_piece("f2".parse().unwrap()).unwrap().name,
                "tampo aberto 119 × 30"
            );
            let post = home.find_piece("f3".parse().unwrap()).unwrap();
            assert_eq!(post.info.brand.as_deref(), Some("Blum"), "both edits kept");
            assert_eq!(
                home.furniture[0].name, "mesa basculante",
                "the group keeps its name"
            );
            let stale = newera_core::check_annotations(home);
            assert!(stale.stale.is_empty(), "{stale:?}");
        }
        let err = update(r#"[{"id":"f2","w":80}]"#).unwrap_err();
        assert!(err.message.contains("belongs to the group"), "{err:?}");
        s.document.write().undo().unwrap();
        assert_eq!(
            s.document
                .read()
                .home()
                .find_piece("f2".parse().unwrap())
                .unwrap()
                .name,
            "tampo aberto 110 × 30",
            "one undoable step"
        );
    }

    #[test]
    fn a_group_resized_and_a_part_renamed_in_one_update_moves_its_parts() {
        let s = server();
        let part = |id: u64, name: &str, x: f64, w: f64| newera_core::Furniture {
            id: newera_core::FurnitureId(id),
            catalog: "box".into(),
            name: name.to_owned(),
            position: newera_core::Point2::new(x, 30.0),
            width: w,
            depth: 30.0,
            height: 3.0,
            elevation: 75.0,
            ..newera_core::Furniture::default()
        };
        {
            let mut doc = s.document.write();
            let mut group = part(1, "mesa", 60.0, 120.0);
            group.height = 78.0;
            group.elevation = 0.0;
            group.children = vec![part(2, "tampo", 60.0, 120.0), part(3, "pé", 3.0, 6.0)];
            doc.execute(newera_core::Command::insert(group)).unwrap();
        }
        // Either order: the group twice as tall, its top renamed.
        for items in [
            r#"[{"id":"f1","h":156},{"id":"f2","name":"tampo de vidro"}]"#,
            r#"[{"id":"f2","name":"tampo de vidro"},{"id":"f1","h":156}]"#,
        ] {
            s.update(Parameters(UpdateParams {
                items: serde_json::from_str(items).unwrap(),
                rename: None,
                v: None,
                dry: None,
            }))
            .unwrap();
            {
                let doc = s.document.read();
                let home = doc.home();
                assert!((home.furniture[0].height - 156.0).abs() < 1e-9, "{items}");
                let top = home.find_piece("f2".parse().unwrap()).unwrap();
                assert_eq!(top.name, "tampo de vidro", "{items}");
                assert!(
                    top.elevation > 140.0,
                    "{items}: the top stayed at {}",
                    top.elevation
                );
            }
            s.document.write().undo().unwrap();
        }
    }

    #[test]
    fn a_dry_move_says_what_kind_of_clash_it_trades_for() {
        let s = server();
        // Two stones of a peninsula that touch, end to end.
        s.place(Parameters(
            serde_json::from_str(
                r#"{"items":[{"cat":"box","name":"pedra esquerda","at":[50,30],"w":100,"d":60,"h":90},
                             {"cat":"box","name":"pedra direita","at":[130,30],"w":60,"d":60,"h":90}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let (left, right) = {
            let doc = s.document.read();
            let f = &doc.home().furniture;
            (f[0].id.to_string(), f[1].id.to_string())
        };
        let nudge = |dx: f64| -> serde_json::Value {
            serde_json::from_str(
                &s.move_elements(Parameters(
                    serde_json::from_str(&format!(
                        r#"{{"ids":["{right}"],"dx":{dx},"dy":0,"dry":true}}"#
                    ))
                    .unwrap(),
                ))
                .unwrap(),
            )
            .unwrap()
        };

        // 5.5 cm into the other stone: a real clash, measured, and a
        // clearance that says "inside", not "touching".
        let dry = nudge(-5.5);
        let new = &dry["issues_new"][0];
        assert_eq!(new["ids"], format!("{left}+{right}"), "{dry}");
        assert_eq!(new["kind"], "collision", "{dry}");
        assert_eq!(new["extent"], serde_json::json!([5.5, 60, 90]), "{dry}");
        let side = &dry["clearances"][right.as_str()]["-x"];
        assert!((side[0].as_f64().unwrap() + 5.5).abs() < 0.05, "{dry}");
        assert_eq!(side[1], left.as_str(), "{dry}");

        // Once applied, pushing it further is not a new clash but a worse
        // one, and the dry run says by how much.
        s.move_elements(Parameters(
            serde_json::from_str(&format!(r#"{{"ids":["{right}"],"dx":-5.5,"dy":0}}"#)).unwrap(),
        ))
        .unwrap();
        let dry = nudge(-4.5);
        assert!(dry.get("issues_new").is_none(), "{dry}");
        let grown = &dry["issues_changed"][0];
        assert_eq!(grown["kind"], "collision", "{dry}");
        assert_eq!(grown["extent_was"][0], 5.5, "{dry}");
        assert_eq!(grown["extent"][0], 10, "{dry}");

        // And backing out settles it, named as what it was.
        let dry = nudge(5.5);
        assert_eq!(dry["issues_resolved"][0]["kind"], "collision", "{dry}");
        assert!(
            dry["clearances"][right.as_str()]["-x"][0]
                .as_f64()
                .unwrap()
                .abs()
                < 0.05,
            "{dry}"
        );
    }
    #[test]
    fn a_resize_can_hold_one_face_instead_of_growing_around_the_center() {
        let s = server();
        s.create(Parameters(
            serde_json::from_str(
                r#"{"walls":[{"pts":[[0,0],[500,0],[500,400],[0,400]],"closed":true}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        // A counter with its back on the top wall, and one turned a quarter
        // turn with its back on the left wall.
        s.place(Parameters(
            serde_json::from_str(
                r#"{"items":[{"cat":"base-cabinet","at":[250,37.5],"w":300,"d":60,"h":90},
                             {"cat":"base-cabinet","at":[37.5,250],"w":200,"d":60,"h":90,"angle":270}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let deepen = |id: &str, anchor: &str| {
            s.update(Parameters(UpdateParams {
                items: serde_json::from_str(&format!(
                    r#"[{{"id":"{id}","d":80,"anchor":"{anchor}"}}]"#
                ))
                .unwrap(),
                rename: None,
                v: None,
                dry: None,
            }))
            .unwrap();
        };

        deepen("f5", "back");
        let home = s.document.read();
        let counter = home.home().find_piece("f5".parse().unwrap()).unwrap();
        let (min, max) = newera_core::plan_bounds(counter);
        assert!(
            (min.y - 7.5).abs() < 0.01,
            "the back stays on the wall: {min:?}"
        );
        assert!((max.y - 87.5).abs() < 0.01, "the front advances: {max:?}");
        drop(home);

        // Turned 270°, the piece's back looks at -x: the same word holds the
        // face against the left wall, not a plan side worked out by hand.
        deepen("f6", "back");
        let home = s.document.read();
        let turned = home.home().find_piece("f6".parse().unwrap()).unwrap();
        let (min, max) = newera_core::plan_bounds(turned);
        assert_eq!(newera_core::facing(turned), "+x", "{turned:?}");
        assert!((min.x - 7.5).abs() < 0.01, "{min:?}");
        assert!((max.x - 87.5).abs() < 0.01, "{max:?}");
        drop(home);

        // Without an anchor the center is what stays, which is the old trap.
        s.update(Parameters(UpdateParams {
            items: serde_json::from_str(r#"[{"id":"f5","d":100}]"#).unwrap(),
            rename: None,
            v: None,
            dry: None,
        }))
        .unwrap();
        let home = s.document.read();
        let counter = home.home().find_piece("f5".parse().unwrap()).unwrap();
        let (min, _) = newera_core::plan_bounds(counter);
        assert!((min.y - (-2.5)).abs() < 0.01, "{min:?}");
    }
    #[test]
    fn labels_and_dimensions_in_3d() {
        let s = server();
        let params: CreateParams = serde_json::from_str(
            r#"{"labels":[{"text":"Sala","at":[10,10],"pitch":90,"elev":150},{"text":"Plano","at":[0,0]}],
                "dims":[{"a":[0,0],"b":[300,0],"off":30,"in3d":true,"elev":250,"pitch":90}]}"#,
        )
        .unwrap();
        s.create(Parameters(params)).unwrap();
        {
            let doc = s.document.read();
            let home = doc.home();
            assert_eq!(home.labels[0].pitch, Some(90.0));
            assert!((home.labels[0].elevation - 150.0).abs() < 1e-9);
            assert_eq!(home.labels[1].pitch, None);
            let d = &home.dimensions[0];
            assert!(d.visible_in_3d && (d.elevation[1] - 250.0).abs() < 1e-9);
        }
        let ids: Vec<String> = {
            let doc = s.document.read();
            vec![
                doc.home().labels[0].id.to_string(),
                doc.home().labels[1].id.to_string(),
                doc.home().dimensions[0].id.to_string(),
            ]
        };
        let specs: Vec<UpdateSpec> = serde_json::from_str(&format!(
            r#"[{{"id":"{}","in3d":false}},{{"id":"{}","pitch":0}},{{"id":"{}","in3d":false}}]"#,
            ids[0], ids[1], ids[2]
        ))
        .unwrap();
        s.update(Parameters(UpdateParams {
            items: specs,
            rename: None,
            v: None,
            dry: None,
        }))
        .unwrap();
        let doc = s.document.read();
        let home = doc.home();
        assert_eq!(home.labels[0].pitch, None);
        assert_eq!(home.labels[1].pitch, Some(0.0));
        assert!(!home.dimensions[0].visible_in_3d);
    }
    #[test]
    fn polylines_label_styles_and_cameras() {
        let s = server();
        let params: CreateParams = serde_json::from_str(
            r#"{"labels":[{"text":"Tomada","at":[10,10]}],
                "polylines":[{"pts":[[0,0],[100,0],[100,50]],"t":2,"color":[200,0,0],"dash":"dash","arrows":["none","delta"]}]}"#,
        )
        .unwrap();
        assert_eq!(s.create(Parameters(params)).unwrap(), "ok rev=1 ids=t1,pl2");
        let spec: UpdateSpec =
            serde_json::from_str(r#"{"id":"t1","bold":true,"align":"left","color":[0,0,255]}"#)
                .unwrap();
        s.update(Parameters(UpdateParams {
            items: vec![spec],
            rename: None,
            v: None,
            dry: None,
        }))
        .unwrap();
        let home = s.get_home(Parameters(GetHomeParams::default())).unwrap();
        assert!(
            home.contains(r#""bold":true"#) && home.contains(r#""align":"left""#),
            "{home}"
        );
        assert!(
            home.contains(r#""dash":"dash""#) && home.contains(r#""arrows":["none","delta"]"#),
            "{home}"
        );

        let store = |name: &str| CamerasParams {
            action: Some("store".into()),
            name: Some(name.into()),
            x: Some(100.0),
            y: Some(200.0),
            ..CamerasParams::default()
        };
        s.cameras(Parameters(store("Sala"))).unwrap();
        s.cameras(Parameters(CamerasParams {
            action: Some("view".into()),
            i: Some(0),
            ..CamerasParams::default()
        }))
        .unwrap();
        let list = s.cameras(Parameters(CamerasParams::default())).unwrap();
        assert!(
            list.contains(r#""active":"visitor""#) && list.contains("Sala"),
            "{list}"
        );
        assert!(
            s.cameras(Parameters(CamerasParams {
                action: Some("view".into()),
                i: Some(5),
                ..CamerasParams::default()
            }))
            .is_err()
        );
        s.undo().unwrap();
        assert!(
            !s.document.read().home().cameras.observer_active,
            "undo restores the aerial view"
        );
    }
    #[test]
    fn an_elev_the_ceiling_takes_back_is_said_as_snapped() {
        let s = server();
        s.create(Parameters(
            serde_json::from_str(
                r#"{"walls":[{"pts":[[0,0],[400,0],[400,400],[0,400]],"closed":true}],
                    "rooms":[{"name":"Sala","at":[200,200]}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let reply = s
            .place(Parameters(
                serde_json::from_str(
                    r#"{"items":[{"cat":"downlight","at":[200,200],"elev":200}]}"#,
                )
                .unwrap(),
            ))
            .unwrap();
        let (id, elev) = {
            let doc = s.document.read();
            let f = doc.home().furniture.last().unwrap();
            (f.id.to_string(), f.elevation)
        };
        assert!(elev > 240.0, "on the 250 cm ceiling: {elev}");
        assert!(
            reply.contains(&format!("{id}: elev 200 snapped to the ceiling")),
            "{reply}"
        );
        let reply = s
            .update(Parameters(UpdateParams {
                items: serde_json::from_str(&format!(r#"[{{"id":"{id}","elev":200}}]"#)).unwrap(),
                rename: None,
                v: None,
                dry: None,
            }))
            .unwrap();
        assert!(!reply.contains("already had"), "{reply}");
        assert!(reply.contains("elev 200 snapped to the ceiling"), "{reply}");
    }
    #[test]
    fn a_slope_a_storey_and_an_opacity_are_said_as_changes() {
        let s = server();
        s.create(Parameters(
            serde_json::from_str(r#"{"walls":[{"pts":[[0,0],[400,0]]}]}"#).unwrap(),
        ))
        .unwrap();
        s.place(Parameters(
            serde_json::from_str(r#"{"items":[{"cat":"box","at":[200,200]}]}"#).unwrap(),
        ))
        .unwrap();
        let (wall, piece) = {
            let doc = s.document.read();
            (
                doc.home().walls[0].id.to_string(),
                doc.home().furniture[0].id.to_string(),
            )
        };
        let upper = newera_core::ops::add_level(&mut s.document.write(), None, None).unwrap();
        let update = |item: String| {
            let reply = s
                .update(Parameters(UpdateParams {
                    items: serde_json::from_str(&format!("[{item}]")).unwrap(),
                    rename: None,
                    v: None,
                    dry: None,
                }))
                .unwrap();
            assert!(!reply.contains("unchanged"), "{item}: {reply}");
            let diff: serde_json::Value =
                serde_json::from_str(&reply[reply.find('{').expect("a diff")..]).unwrap();
            diff["changed"][0]["to"].clone()
        };
        let to = update(format!(r#"{{"id":"{wall}","h_end":300}}"#));
        assert_eq!(to["h_end"], 300.0, "{to}");
        let to = update(format!(r#"{{"id":"{wall}","level":"{upper}"}}"#));
        assert_eq!(to["level"], upper.to_string(), "{to}");
        let to = update(format!(r#"{{"id":"{piece}","opacity":0.15}}"#));
        assert_eq!(to["opacity"], 0.15, "{to}");
    }
    #[test]
    fn sloping_walls_and_tilted_pieces() {
        let s = server();
        // A gable: 600 cm base rising to 675 cm in the middle.
        let params: CreateParams =
            serde_json::from_str(r#"{"walls":[{"pts":[[0,0],[300,0],[600,0]],"hs":[10,675,10]}]}"#)
                .unwrap();
        s.create(Parameters(params)).unwrap();
        {
            let doc = s.document.read();
            let walls = &doc.home().walls;
            assert_eq!(
                (walls[0].height, walls[0].height_at_end),
                (10.0, Some(675.0))
            );
            assert_eq!(
                (walls[1].height, walls[1].height_at_end),
                (675.0, Some(10.0))
            );
        }
        let bad: CreateParams =
            serde_json::from_str(r#"{"walls":[{"pts":[[0,0],[100,0]],"hs":[10]}]}"#).unwrap();
        assert!(s.create(Parameters(bad)).is_err());
        let wall = s.document.read().home().walls[0].id.to_string();
        let spec: UpdateSpec =
            serde_json::from_str(&format!(r#"{{"id":"{wall}","h_end":300}}"#)).unwrap();
        s.update(Parameters(UpdateParams {
            items: vec![spec],
            rename: None,
            v: None,
            dry: None,
        }))
        .unwrap();
        assert_eq!(s.document.read().home().walls[0].height_at_end, Some(300.0));

        // A 400 cm rafter tilted 45°: its far end rises.
        let params: PlaceParams = serde_json::from_str(
            r#"{"items":[{"cat":"box","at":[300,300],"w":10,"d":400,"h":10,"elev":100,"pitch":45}]}"#,
        )
        .unwrap();
        s.place(Parameters(params)).unwrap();
        let doc = s.document.read();
        let piece = doc.home().furniture.last().unwrap().clone();
        assert!((piece.pitch - 45.0).abs() < 1e-9);
        let mut only = doc.home().clone();
        only.walls.clear();
        let mesh =
            newera_render::Mesh::from_home(&only, &newera_render::Selection::new(), &|_| None);
        let top = mesh
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max);
        // Centered at 105 cm, half its length at 45° adds ~141 cm.
        assert!(top > 2.3 && top < 2.6, "{top}");
    }
    #[test]
    fn dividers_and_rooms_that_follow_walls() {
        let s = server();
        let params: CreateParams = serde_json::from_str(
            r#"{"walls":[{"pts":[[0,0],[800,0],[800,400],[0,400]],"closed":true}],
                "polylines":[{"pts":[[450,0],[450,400]],"divider":true}],
                "rooms":[{"name":"Sala","at":[200,200]},{"name":"Jantar","at":[600,200]}]}"#,
        )
        .unwrap();
        s.create(Parameters(params)).unwrap();
        let (sala, jantar, line) = {
            let doc = s.document.read();
            let h = doc.home();
            assert!(h.polylines[0].room_divider && h.rooms.iter().all(|r| r.auto));
            (
                h.rooms[0].area(),
                h.rooms[1].area(),
                h.polylines[0].id.to_string(),
            )
        };
        assert!(sala > jantar + 90.0 * 300.0, "{sala} {jantar}");
        let spec: UpdateSpec =
            serde_json::from_str(&format!(r#"{{"id":"{line}","pts":[[350,0],[350,400]]}}"#))
                .unwrap();
        s.update(Parameters(UpdateParams {
            items: vec![spec],
            rename: None,
            v: None,
            dry: None,
        }))
        .unwrap();
        let doc = s.document.read();
        let h = doc.home();
        assert!(
            h.rooms[1].area() > h.rooms[0].area(),
            "rooms followed the divider"
        );
    }
    #[test]
    fn solids_and_skylights() {
        let s = server();
        let params: CreateParams = serde_json::from_str(
            r#"{"roofs":[{"pts":[[0,0],[0,700],[600,700],[600,0]],"h":0,"ridge_h":675,"overhang":0,"gables":true,
                          "skylights":[{"at":[150,350],"w":100,"d":80}]}],
                "solids":[{"pts":[[150,400],[450,400],[300,650]],"h":15,"elev":300,"mat":"wood"},
                          {"profile":[[-100,0],[100,0],[0,150]],"a":[1000,0],"b":[1000,300]}]}"#,
        )
        .unwrap();
        s.create(Parameters(params)).unwrap();
        let doc = s.document.read();
        let home = doc.home();
        let roof = home.furniture.iter().find(|f| f.is_group()).unwrap();
        // The slope with the skylight is split around it, plus the glass and the ridge cap.
        assert_eq!(
            roof.children.len(),
            1 + 5 + 1,
            "{:?}",
            roof.children.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        assert_eq!(
            roof.children.iter().filter(|c| c.opacity.is_some()).count(),
            1
        );
        let slab = home
            .furniture
            .iter()
            .find(|f| matches!(f.shape, Some(newera_core::SolidShape::Outline(_))))
            .unwrap();
        assert!((slab.width - 300.0).abs() < 1e-9 && (slab.elevation - 300.0).abs() < 1e-9);
        let gable = home
            .furniture
            .iter()
            .find(|f| matches!(f.shape, Some(newera_core::SolidShape::Profile(_))))
            .unwrap();
        assert!((gable.depth - 300.0).abs() < 1e-9 && (gable.height - 150.0).abs() < 1e-9);
        assert!(
            (gable.position.x - 1000.0).abs() < 1e-6 && (gable.position.y - 150.0).abs() < 1e-6
        );
        let mut only = home.clone();
        only.walls.clear();
        only.furniture.retain(|f| f.shape.is_some());
        let mesh =
            newera_render::Mesh::from_home(&only, &newera_render::Selection::new(), &|_| None);
        let top = mesh
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max);
        assert!((top - 3.15).abs() < 0.01, "slab top {top}");
        drop(doc);
        let bad: CreateParams =
            serde_json::from_str(r#"{"solids":[{"profile":[[0,0],[1,0],[0,1]]}]}"#).unwrap();
        assert!(s.create(Parameters(bad)).is_err());
    }

    #[test]
    fn walls_written_a_few_centimetres_off_still_join() {
        let s = server();
        // Outer walls 20 cm thick, then a partition written to the inner face
        // of one wall and 3 cm short of the other — how a description reads,
        // not how the walls have to meet.
        s.create(Parameters(
            serde_json::from_str(
                r#"{"walls":[{"pts":[[0,0],[600,0],[600,400],[0,400]],"closed":true,"t":20}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        s.create(Parameters(
            serde_json::from_str(r#"{"walls":[{"pts":[[300,10],[300,397]],"t":10}]}"#).unwrap(),
        ))
        .unwrap();
        let home = s.document.read().home().clone();
        let partition = home.walls.last().unwrap();
        assert_eq!(
            (partition.start, partition.end),
            (
                newera_core::Point2::new(300.0, 0.0),
                newera_core::Point2::new(300.0, 400.0)
            ),
            "the ends land on the walls they meet"
        );
        // And the drawing has no wall over another: the outlines cover the
        // footprint exactly once.
        let covered: f64 = home
            .wall_outlines()
            .iter()
            .map(|o| newera_core::polygon_area(o))
            .sum();
        let ring = 620.0 * 420.0 - 580.0 * 380.0;
        assert!((covered - (ring + 10.0 * 380.0)).abs() < 1e-6, "{covered}");
    }

    #[test]
    fn moving_an_end_into_a_wall_joins_it_there() {
        let s = server();
        s.create(Parameters(
            serde_json::from_str(
                r#"{"walls":[{"pts":[[0,0],[600,0]],"t":20},{"pts":[[300,200],[300,80]],"t":10}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let id = s.document.read().home().walls[1].id.to_string();
        s.update(Parameters(UpdateParams {
            items: serde_json::from_str(&format!(r#"[{{"id":"{id}","b":[300,-4]}}]"#)).unwrap(),
            rename: None,
            v: None,
            dry: None,
        }))
        .unwrap();
        assert_eq!(
            s.document.read().home().walls[1].end,
            newera_core::Point2::new(300.0, 0.0)
        );
    }

    #[test]
    fn walls_on_one_line_join_into_a_single_one() {
        let s = server();
        s.create(Parameters(
            serde_json::from_str(
                r#"{"walls":[{"pts":[[0,0],[300,0]],"t":20},{"pts":[[300,0],[800,0]],"t":20}]}"#,
            )
            .unwrap(),
        ))
        .unwrap();
        let ids: Vec<String> = s
            .document
            .read()
            .home()
            .walls
            .iter()
            .map(|w| w.id.to_string())
            .collect();
        let reply = s
            .merge_walls(Parameters(IdsParams { ids: ids.clone() }))
            .unwrap();
        let home = s.document.read().home().clone();
        assert_eq!(home.walls.len(), 1, "{reply}");
        let wall = &home.walls[0];
        assert_eq!(wall.id.to_string(), ids[1], "the longest one stays");
        assert_eq!(
            (wall.start, wall.end),
            (
                newera_core::Point2::new(0.0, 0.0),
                newera_core::Point2::new(800.0, 0.0)
            )
        );
        // An L cannot become one wall, and says so without changing anything.
        s.create(Parameters(
            serde_json::from_str(r#"{"walls":[{"pts":[[800,0],[800,400]],"t":20}]}"#).unwrap(),
        ))
        .unwrap();
        let both: Vec<String> = s
            .document
            .read()
            .home()
            .walls
            .iter()
            .map(|w| w.id.to_string())
            .collect();
        let err = s
            .merge_walls(Parameters(IdsParams { ids: both }))
            .unwrap_err()
            .to_string();
        assert!(err.contains("same line"), "{err}");
        assert_eq!(s.document.read().home().walls.len(), 2);
    }
}
