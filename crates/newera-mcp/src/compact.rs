//! Token-lean JSON views of a home.
//!
//! Conventions (documented once in the server instructions, never repeated):
//! short ids, `[x,y]` points, numbers rounded to 0.1 cm without trailing
//! `.0`, and fields omitted when they hold their default value.

use newera_core::{Compass, Home, Point2, Wall};
use serde_json::{Map, Value, json};

/// Rounds to 0.1 and prints integers without a fractional part.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn num(value: f64) -> Value {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 && rounded.abs() < 9e15 {
        json!(rounded as i64)
    } else {
        json!(rounded)
    }
}

pub(crate) fn point(p: Point2) -> Value {
    json!([num(p.x), num(p.y)])
}

fn points(points: &[Point2]) -> Value {
    Value::Array(points.iter().copied().map(point).collect())
}

fn obj(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect::<Map<_, _>>(),
    )
}

pub(crate) fn room(r: &newera_core::Room) -> Value {
    let mut v = obj([
        ("id", json!(r.id.to_string())),
        ("name", json!(r.name)),
        ("ceiling_flat", json!(r.ceiling_flat)),
        ("pts", points(&r.points)),
        ("m2", json!((r.area() / 100.0).round() / 100.0)),
    ]);
    if !r.usage.is_auto() {
        v["room_use"] = json!(r.usage);
    }
    if !r.floor_visible {
        v["floor"] = json!(false);
    }
    if !r.ceiling_visible {
        v["ceiling"] = json!(false);
    }
    if let Some(m) = &r.floor_material {
        v["floor_mat"] = json!(m.to_string());
    }
    if let Some(m) = &r.ceiling_material {
        v["ceil_mat"] = json!(m.to_string());
    }
    v
}

pub(crate) fn dimension(d: &newera_core::Dimension) -> Value {
    let mut v = obj([
        ("id", json!(d.id.to_string())),
        ("a", point(d.start)),
        ("b", point(d.end)),
        ("len", num(d.length())),
    ]);
    if d.offset != 0.0 {
        v["off"] = num(d.offset);
    }
    v
}

pub(crate) fn label(l: &newera_core::Label) -> Value {
    let mut v = obj([
        ("id", json!(l.id.to_string())),
        ("text", json!(l.text)),
        ("at", point(l.position)),
    ]);
    if (l.size - newera_core::Label::DEFAULT_SIZE).abs() > f64::EPSILON {
        v["size"] = num(l.size);
    }
    if l.angle != 0.0 {
        v["angle"] = num(l.angle);
    }
    if l.bold {
        v["bold"] = json!(true);
    }
    if let Some(d) = l.discipline {
        v["layer"] = json!(d);
    }
    if l.italic {
        v["italic"] = json!(true);
    }
    if l.align != newera_core::TextAlign::Center {
        v["align"] = json!(l.align);
    }
    if let Some(color) = l.color {
        v["color"] = json!(color);
    }
    v
}

pub(crate) fn polyline(p: &newera_core::Polyline) -> Value {
    let mut v = obj([
        ("id", json!(p.id.to_string())),
        ("pts", points(&p.points)),
        ("t", num(p.thickness)),
        ("color", json!(p.color)),
    ]);
    if p.closed {
        v["closed"] = json!(true);
    }
    if let Some(d) = p.discipline {
        v["layer"] = json!(d);
    }
    if p.dash != newera_core::DashStyle::Solid {
        v["dash"] = json!(p.dash);
    }
    if p.join == newera_core::LineJoin::Curved {
        v["curved"] = json!(true);
    }
    if p.start_arrow != newera_core::ArrowStyle::None
        || p.end_arrow != newera_core::ArrowStyle::None
    {
        v["arrows"] = json!([p.start_arrow, p.end_arrow]);
    }
    v
}

/// The array key each kind of element is listed under.
pub(crate) const KINDS: [&str; 6] = ["walls", "rooms", "dims", "labels", "furniture", "polylines"];

pub(crate) fn kind_of(id: newera_core::ElementId) -> &'static str {
    use newera_core::ElementId;
    match id {
        ElementId::Wall(_) => "walls",
        ElementId::Room(_) => "rooms",
        ElementId::Dimension(_) => "dims",
        ElementId::Label(_) => "labels",
        ElementId::Furniture(_) => "furniture",
        ElementId::Polyline(_) => "polylines",
        ElementId::Level(_) => "levels",
    }
}

/// One element, in the same shape `home` lists it in. Reaches pieces inside
/// groups, which the top-level listing only counts.
pub(crate) fn element(home: &Home, id: newera_core::ElementId) -> Option<Value> {
    use newera_core::ElementId;
    match id {
        ElementId::Wall(w) => home.wall(w).map(wall),
        ElementId::Room(r) => home.room(r).map(room),
        ElementId::Dimension(d) => home.dimension(d).map(dimension),
        ElementId::Label(l) => home.label(l).map(label),
        ElementId::Polyline(p) => home.polyline(p).map(polyline),
        ElementId::Furniture(f) => {
            let cuts = home.wall_cuts();
            home.find_piece(f).map(|p| piece(home, &cuts, p))
        }
        ElementId::Level(_) => None,
    }
}

/// Full compact state.
pub(crate) fn home(home: &Home, revision: u64) -> Value {
    let mut out = obj([("rev", json!(revision)), ("name", json!(home.name))]);

    let cuts = home.wall_cuts();
    for (key, list) in [
        ("walls", home.walls.iter().map(wall).collect::<Vec<_>>()),
        ("rooms", home.rooms.iter().map(room).collect()),
        ("dims", home.dimensions.iter().map(dimension).collect()),
        ("labels", home.labels.iter().map(label).collect()),
        (
            "furniture",
            home.furniture
                .iter()
                .map(|f| piece(home, &cuts, f))
                .collect(),
        ),
        ("polylines", home.polylines.iter().map(polyline).collect()),
    ] {
        if !list.is_empty() {
            out[key] = Value::Array(list);
        }
    }
    if home.compass != Compass::default() {
        out["north"] = num(home.compass.north_degrees);
    }
    if let Some(bg) = &home.background {
        let (min, max) = bg.bounds();
        out["background"] = obj([
            ("path", json!(bg.path)),
            ("cm_per_px", json!(bg.cm_per_px)),
            ("from", point(min)),
            ("to", point(max)),
            ("opacity", json!(bg.opacity)),
            ("visible", json!(bg.visible)),
        ]);
    }
    out
}

/// Which tool built a composite piece, so an id from a report can be taken
/// back to the tool that owns it instead of guessing and being told no.
pub(crate) fn made_by(f: &newera_core::Furniture) -> &'static str {
    let has = |key: &str| f.properties.contains_key(key);
    if has(newera_joinery::PARAMS_KEY) {
        "joinery"
    } else if has(newera_joinery::RUN_KEY) {
        "cabinet_run"
    } else if f.catalog.starts_with("roof") {
        "fit_roof"
    } else if f.model.is_some() {
        "import"
    } else {
        "group"
    }
}

/// A piece, omitting whatever matches its catalog defaults.
pub(crate) fn piece(
    home: &Home,
    cuts: &[Vec<newera_core::WallCut>],
    f: &newera_core::Furniture,
) -> Value {
    let mut v = obj([
        ("id", json!(f.id.to_string())),
        ("cat", json!(f.catalog)),
        ("at", point(f.position)),
    ]);
    let item = newera_catalog::find(&f.catalog);
    let default_size = item.map(|i| i.size);
    if f.angle.rem_euclid(360.0).abs() > 0.05 {
        v["angle"] = num(f.angle.rem_euclid(360.0));
    }
    // `wdh` is in the piece's own frame: with a quarter turn, width and
    // depth swap in the plan. The resolved box is always given so nobody
    // has to redo that rotation by hand — and `faces`, the side the piece
    // opens toward, is not readable from `angle` alone.
    let (min, max) = newera_core::plan_bounds(f);
    v["bounds"] = json!([point(min), point(max)]);
    v["faces"] = json!(newera_core::facing(f));
    // The plan layer the piece is in (for a part, its group's when it has none).
    let layer = home.part_owner(f.id).map_or_else(
        || newera_core::layer_of(f),
        |top| newera_core::layer_in_group(top, f),
    );
    if let Some(layer) = layer {
        v["plan_layer"] = json!(layer.key());
    }
    if default_size != Some([f.width, f.depth, f.height]) {
        v["wdh"] = json!([num(f.width), num(f.depth), num(f.height)]);
    }
    // Said whenever it is off the floor: the catalog's default elevation is
    // nowhere to be read (a LED strip is at 140 cm by itself).
    if f.elevation.abs() >= 0.05 {
        v["elev"] = num(f.elevation);
    }
    if item.map(|i| i.name) != Some(f.name.as_str()) {
        v["name"] = json!(f.name);
    }
    if let Some(model) = &f.model {
        v["model"] = json!(model);
    }
    if let Some(color) = f.color {
        v["color"] = json!(color);
    }
    // The finish, as `mat` takes it: what was written can be read back.
    if let Some(mat) = &f.texture {
        v["mat"] = json!(mat.to_string());
    }
    // To the hundredth, as it is given (0.15 for glass): `num` would round
    // it to a tenth and hide the change.
    if let Some(opacity) = f.opacity {
        v["opacity"] = json!((opacity * 100.0).round() / 100.0);
    }
    if f.mirrored {
        v["mirror"] = json!(true);
    }
    if f.is_group() {
        v["parts"] = json!(f.flatten().len() - 1);
        v["made_by"] = json!(made_by(f));
    }
    if let Some(d) = f.discipline {
        v["layer"] = json!(d);
    }
    if let Some(role) = f.properties.get(newera_core::Furniture::ROLE_KEY) {
        v["role"] = json!(role);
    }
    if let Some(light) = &f.light {
        let mut l = json!({"lm": light.flux().round()});
        if let Some(k) = light.kelvin {
            l["k"] = num(k);
        }
        if let Some(beam) = light.beam {
            l["beam"] = num(beam);
        }
        if let Some(watts) = light.watts {
            l["w"] = num(watts);
        }
        if let Some(area) = light.area {
            l["area"] = json!([num(area[0]), num(area[1])]);
            l["panel_upward"] = json!(light.panel_upward.unwrap_or(false));
        }
        v["light"] = l;
    }
    if !f.visible {
        v["visible"] = json!(false);
    }
    // Declared fixed or free-standing (`update fixed`), so the change is seen.
    match f
        .properties
        .get(newera_core::mounting::FIXED_KEY)
        .map(String::as_str)
    {
        Some("true") => v["fixed"] = json!(true),
        Some("false") => v["fixed"] = json!(false),
        _ => {}
    }
    // Which jamb a hinged leaf turns on: a flip of it is a change, and a dry
    // run that answered `{}` for one looked like a flip that did nothing.
    // Always said, true or false: a flip read as `true → null` looked like
    // the value had been lost.
    if let Some(o) = f
        .opening
        .as_ref()
        .filter(|o| !o.sliding && o.leaves < 2 && o.kind == newera_core::OpeningKind::Door)
    {
        v["hinge_right"] = json!(o.hinge_right);
    }
    if f.is_opening()
        && let Some(i) = cuts
            .iter()
            .position(|c| c.iter().any(|cut| cut.furniture == f.id))
    {
        v["wall"] = json!(home.walls[i].id.to_string());
    }
    v
}

pub(crate) fn wall(w: &Wall) -> Value {
    let mut v = obj([
        ("id", json!(w.id.to_string())),
        ("a", point(w.start)),
        ("b", point(w.end)),
    ]);
    if (w.thickness - Wall::DEFAULT_THICKNESS).abs() > f64::EPSILON {
        v["t"] = num(w.thickness);
    }
    if (w.height - Wall::DEFAULT_HEIGHT).abs() > f64::EPSILON {
        v["h"] = num(w.height);
    }
    // A sloping wall's height at `b`: without it `update h_end` read back as
    // a write that changed nothing.
    if let Some(h_end) = w.height_at_end {
        v["h_end"] = num(h_end);
    }
    if let Some(arc) = w.arc_extent.filter(|_| w.is_arc()) {
        v["arc"] = num(arc);
    }
    if let Some(kind) = &w.wall_type {
        v["type"] = json!(kind);
    }
    match (&w.left_side, &w.right_side) {
        (Some(l), Some(r)) if l == r => v["sides"] = json!(l.to_string()),
        (left, right) => {
            if let Some(l) = left {
                v["left"] = json!(l.to_string());
            }
            if let Some(r) = right {
                v["right"] = json!(r.to_string());
            }
        }
    }
    v
}

/// Wall types and material patterns, for the `materials` tool.
pub(crate) fn materials() -> Value {
    let walls: Vec<Value> = newera_core::WALL_TYPES
        .iter()
        .map(|t| json!([t.id, t.name, num(t.thickness)]))
        .collect();
    let patterns: Vec<Value> = newera_core::Pattern::ALL
        .iter()
        .map(|p| {
            let [r, g, b] = p.default_color();
            let [w, h] = p.default_tile();
            json!([
                p.key(),
                p.label(),
                format!("#{r:02x}{g:02x}{b:02x}"),
                format!("{w}x{h}")
            ])
        })
        .collect();
    obj([("wall_types", json!(walls)), ("patterns", json!(patterns))])
}

/// Cheapest overview: counts, bounds and room areas, no geometry.
pub(crate) fn summary(home: &Home, revision: u64) -> Value {
    let mut out = obj([
        ("rev", json!(revision)),
        ("name", json!(home.name)),
        (
            "counts",
            obj([
                ("walls", json!(home.walls.len())),
                ("rooms", json!(home.rooms.len())),
                ("dims", json!(home.dimensions.len())),
                ("labels", json!(home.labels.len())),
                ("furniture", json!(home.furniture.len())),
            ]),
        ),
    ]);
    if let Some((min, max)) = home.bounds() {
        out["bounds"] = json!([point(min), point(max)]);
    }
    if !home.rooms.is_empty() {
        out["rooms"] = Value::Array(
            home.rooms
                .iter()
                .map(|r| json!([r.id.to_string(), r.name, (r.area() / 100.0).round() / 100.0]))
                .collect(),
        );
    }
    out
}

/// Catalog rows `[id, name, w, d, h]`, optionally filtered.
pub(crate) fn catalog(query: Option<&str>, category: Option<&str>, limit: usize) -> Value {
    let items: Vec<_> = newera_catalog::search(query.unwrap_or(""))
        .into_iter()
        .filter(|i| category.is_none_or(|c| i.category.id() == c))
        .collect();
    let rows: Vec<Value> = items
        .iter()
        .take(limit)
        .map(|i| {
            let mut row = vec![
                json!(i.id),
                json!(i.name),
                num(i.size[0]),
                num(i.size[1]),
                num(i.size[2]),
            ];
            // Which side is the front, so nobody places a sofa facing the
            // wall: `wall seat/backrest` — back on a wall, seat to the room.
            let front = i.front();
            if front.stance.has_front() {
                row.push(json!(format!(
                    "{} {}/{}",
                    front.stance.name(),
                    front.front,
                    front.back
                )));
            }
            Value::Array(row)
        })
        .collect();
    let mut out = json!({ "items": rows });
    if items.len() > limit {
        out["more"] = json!(items.len() - limit);
    }
    // An unknown category answers with the ones there are, not a bare empty
    // list that reads as "nothing like that exists".
    let unknown =
        category.is_some_and(|c| !newera_catalog::Category::ALL.iter().any(|k| k.id() == c));
    if (query.is_none() && category.is_none()) || unknown {
        out["categories"] = json!(
            newera_catalog::Category::ALL
                .iter()
                .map(|c| c.id())
                .collect::<Vec<_>>()
        );
    }
    out
}

/// One element in an issue report: id, name, box and height range.
///
/// A bare pair of ids says nothing — a cooktop set into its worktop and a
/// sink eating into the dishwasher next to it read the same. With the name
/// and the geometry alongside, the call that found the problem is also the
/// call that explains it.
pub(crate) fn issue_ref(home: &Home, id: newera_core::ElementId) -> Value {
    use newera_core::ElementId;
    let mut v = obj([("id", json!(id.to_string()))]);
    if let ElementId::Furniture(f) = id
        && let Some(piece) = home.find_piece(f)
    {
        v["name"] = json!(piece.name);
        let (lo, hi) = piece.height_range();
        v["z"] = json!([num(lo), num(hi)]);
        if let Some(level) = home.piece_level(f) {
            v["level"] = json!(level.to_string());
        }
    }
    if let ElementId::Wall(w) = id
        && let Some(wall) = home.wall(w)
    {
        v["name"] = json!(
            wall.wall_type
                .clone()
                .unwrap_or_else(|| "parede".to_owned())
        );
    }
    if let Some((min, max)) = newera_core::element_bounds(home, id) {
        v["bounds"] = json!([point(min), point(max)]);
    }
    v
}

/// A row with the key it is accepted by, first.
fn keyed(row: Value, key: &str) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("key".to_owned(), json!(key));
    if let Value::Object(fields) = row {
        out.extend(fields);
    }
    Value::Object(out)
}

/// Layout issues grouped by kind; empty object when everything is fine.
///
/// Overlaps come classified and measured, worst first, so a report of
/// twenty pairs reads as "one real clash, four built-in pieces, fifteen
/// artefacts of two storeys drawn on top of each other" instead of twenty
/// equally alarming pairs.
pub(crate) fn issues(home: &Home, scope: newera_core::Storeys) -> Value {
    use newera_core::{Issue, Overlap};
    let mut out = serde_json::Map::new();
    let mut push = |key: &str, value: Value| {
        out.entry(key.to_owned())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .expect("array")
            .push(value);
    };
    let mut found = newera_core::check_layout_in(home, scope);
    // Real clashes first: everything after them is explained, not broken.
    found.sort_by_key(|i| match i {
        Issue::Overlap {
            kind: Overlap::Collision,
            ..
        } => 0,
        Issue::Overlap {
            kind: Overlap::Nesting | Overlap::Served,
            ..
        } => 2,
        Issue::Overlap {
            kind: Overlap::CrossLevel,
            ..
        } => 3,
        _ => 1,
    });
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for issue in found {
        // Looked at and accepted: out of the sections that mean "fix this",
        // still in the report with the reason, so it stays reviewable.
        if let Some(why) = issue.accepted(home) {
            let mut row = obj([
                ("key", json!(issue.key())),
                ("kind", json!(issue.kind_name())),
                ("why", json!(why)),
            ]);
            if let Issue::Overlap { extent, .. } = &issue {
                row["extent"] = json!(extent.map(num));
            }
            if let Issue::BlocksWindow { extent, .. } = &issue {
                row["extent"] = json!(extent.map(num));
            }
            push("accepted", row);
            continue;
        }
        let key = issue.key();
        match issue {
            Issue::Overlap { a, b, kind, extent } => {
                *counts.entry(kind.name()).or_default() += 1;
                push(
                    "overlap",
                    obj([
                        ("key", json!(key)),
                        ("a", issue_ref(home, a.into())),
                        ("b", issue_ref(home, b.into())),
                        ("kind", json!(kind.name())),
                        ("extent", json!(extent.map(num))),
                    ]),
                );
            }
            Issue::Blocked { piece, against, cm } => push(
                "blocked",
                obj([
                    ("key", json!(key)),
                    ("piece", issue_ref(home, piece.into())),
                    ("against", issue_ref(home, against)),
                    ("cm", num(cm)),
                ]),
            ),
            Issue::Backwards {
                piece,
                wall,
                cm,
                angle,
            } => push(
                "backwards",
                obj([
                    ("key", json!(key)),
                    ("piece", issue_ref(home, piece.into())),
                    ("wall", issue_ref(home, wall.into())),
                    ("cm", num(cm)),
                    (
                        "fix",
                        json!({"angle": num(angle), "or": "place again with wall=<wall id>"}),
                    ),
                ]),
            ),
            Issue::InWall(f, w) => push(
                "in_wall",
                obj([
                    ("key", json!(key)),
                    ("piece", issue_ref(home, f.into())),
                    ("wall", issue_ref(home, w.into())),
                ]),
            ),
            Issue::BlocksDoor { door, by } => push(
                "blocks_door",
                obj([
                    ("key", json!(key)),
                    ("door", issue_ref(home, door.into())),
                    ("by", issue_ref(home, by.into())),
                ]),
            ),
            Issue::BlocksWindow { window, by, extent } => push(
                "blocks_window",
                obj([
                    ("key", json!(key)),
                    ("window", issue_ref(home, window.into())),
                    ("by", issue_ref(home, by.into())),
                    ("extent", json!(extent.map(num))),
                ]),
            ),
            Issue::OutsideRooms(f) => push("outside_rooms", keyed(issue_ref(home, f.into()), &key)),
            Issue::Loose { piece, why } => push(
                "loose",
                obj([
                    ("key", json!(key)),
                    ("piece", issue_ref(home, piece.into())),
                    ("why", json!(why)),
                ]),
            ),
            Issue::LooseOpening(f) => push("loose_opening", keyed(issue_ref(home, f.into()), &key)),
            Issue::OutgrewNiche { piece, host, over } => push(
                "outgrew_niche",
                obj([
                    ("key", json!(key)),
                    ("piece", issue_ref(home, piece.into())),
                    ("host", issue_ref(home, host.into())),
                    ("over", json!(over.map(num))),
                ]),
            ),
            Issue::AboveCeiling {
                piece,
                room,
                ceiling,
                top,
            } => push(
                "above_ceiling",
                obj([
                    ("key", json!(key)),
                    ("piece", issue_ref(home, piece.into())),
                    ("room", issue_ref(home, room.into())),
                    ("ceiling", num(ceiling)),
                    ("top", num(top)),
                    ("over", num(top - ceiling)),
                ]),
            ),
            Issue::UnratedLight(f) => push("unrated_light", keyed(issue_ref(home, f.into()), &key)),
            Issue::NoDoor { room, passages } => push(
                "no_door",
                obj([
                    ("key", json!(key)),
                    (
                        "name",
                        json!(home.room(room).map_or("", |r| r.name.as_str())),
                    ),
                    ("room", issue_ref(home, room.into())),
                    (
                        "passages",
                        json!(passages.iter().map(ToString::to_string).collect::<Vec<_>>()),
                    ),
                ]),
            ),
            Issue::UnclearFront {
                piece,
                candidates,
                placed,
            } => push(
                "unclear_front",
                obj([
                    ("key", json!(key)),
                    ("piece", issue_ref(home, piece.into())),
                    ("candidates", json!(candidates)),
                    ("placed", json!(placed)),
                ]),
            ),
            Issue::Turned {
                piece,
                built,
                placed,
            } => push(
                "turned",
                obj([
                    ("key", json!(key)),
                    ("piece", issue_ref(home, piece.into())),
                    ("built", json!(built)),
                    ("placed", json!(placed)),
                ]),
            ),
        }
    }
    if !counts.is_empty() {
        out.insert("overlap_kinds".to_owned(), json!(counts));
    }
    for warning in warnings(home) {
        out.entry("warnings".to_owned())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .expect("array")
            .push(json!(warning));
    }
    Value::Object(out)
}

/// Things about the project itself that make every other reading suspect.
///
/// Today that is one thing, and it cost a whole session to diagnose: two
/// storeys that are not reference layers sharing a floor-to-ceiling range,
/// which makes every check report pieces clashing with their own copies.
pub(crate) fn warnings(home: &Home) -> Vec<String> {
    home.stacked_levels()
        .into_iter()
        .map(|(a, b)| {
            let name = |id| home.level(id).map_or_else(String::new, |l| l.name.clone());
            format!(
                "{a} \"{}\" and {b} \"{}\" share an elevation; if one is a tracing or an \
                 older version mark it with levels(reference=true) so checks skip it",
                name(a),
                name(b),
            )
        })
        .collect()
}

/// What one write did, element by element.
///
/// Writes used to answer `ok rev=N` and nothing else, so the only way to
/// learn the effect of a change was to read the whole home back. Comparing
/// the compact view before and after says it in a line: which elements
/// appeared, which went, and exactly which fields moved.
pub(crate) fn diff(before: &Home, after: &Home) -> Value {
    // The rows leave out the storey (reads are already split by it), so it
    // is added here: a move to another level is a change, not a no-op.
    let on = |mut row: Value, level: Option<newera_core::LevelId>| {
        if let Some(level) = level {
            row["level"] = json!(level.to_string());
        }
        row
    };
    let index = |home: &Home| -> std::collections::BTreeMap<String, Value> {
        let cuts = home.wall_cuts();
        let mut map = std::collections::BTreeMap::new();
        for wall in &home.walls {
            map.insert(wall.id.to_string(), on(self::wall(wall), wall.level));
        }
        for r in &home.rooms {
            map.insert(r.id.to_string(), on(room(r), r.level));
        }
        for d in &home.dimensions {
            map.insert(d.id.to_string(), on(dimension(d), d.level));
        }
        for l in &home.labels {
            map.insert(l.id.to_string(), on(label(l), l.level));
        }
        for pl in &home.polylines {
            map.insert(pl.id.to_string(), on(polyline(pl), pl.level));
        }
        for top in &home.furniture {
            for f in top.flatten() {
                map.insert(f.id.to_string(), on(piece(home, &cuts, f), f.level));
            }
        }
        map
    };
    let (old, new) = (index(before), index(after));
    let mut changed = Vec::new();
    let mut added = Vec::new();
    for (id, now) in &new {
        let Some(was) = old.get(id) else {
            added.push(json!(id));
            continue;
        };
        if was == now {
            continue;
        }
        // Only the fields that actually moved, both sides, so the reply can
        // be read without holding the previous state in mind.
        let mut from = serde_json::Map::new();
        let mut to = serde_json::Map::new();
        let keys: std::collections::BTreeSet<&String> = was
            .as_object()
            .into_iter()
            .chain(now.as_object())
            .flat_map(serde_json::Map::keys)
            .collect();
        for key in keys {
            let (a, b) = (was.get(key), now.get(key));
            if a != b {
                from.insert(key.clone(), a.cloned().unwrap_or(Value::Null));
                to.insert(key.clone(), b.cloned().unwrap_or(Value::Null));
            }
        }
        changed.push(json!({"id": id, "from": from, "to": to}));
    }
    let gone: Vec<Value> = old
        .keys()
        .filter(|id| !new.contains_key(*id))
        .map(|id| json!(id))
        .collect();
    let mut out = serde_json::Map::new();
    for (key, list) in [("changed", changed), ("added", added), ("gone", gone)] {
        if !list.is_empty() {
            out.insert(key.to_owned(), Value::Array(list));
        }
    }
    // Settings of the plan itself change as silently as a piece moves, and a
    // lost one is worse: 126 reference numbers vanished from a drawing and
    // nothing in any reply said so. Whatever a write does to them is named.
    if before.annotations != after.annotations {
        out.insert(
            "annotations".to_owned(),
            project_change(before.annotations, after.annotations),
        );
    }
    if before.properties != after.properties {
        let keys: Vec<&String> = before
            .properties
            .keys()
            .chain(after.properties.keys())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|k| before.properties.get(*k) != after.properties.get(*k))
            .collect();
        out.insert("properties".to_owned(), json!(keys));
    }
    Value::Object(out)
}

/// A plan setting before and after, as the MCP names its switches.
fn project_change(was: newera_core::PlanAnnotations, now: newera_core::PlanAnnotations) -> Value {
    let view = |a: newera_core::PlanAnnotations| {
        json!({
            "dims": a.auto_dimensions,
            "refs": a.references,
            "details": a.reference_details,
            "legend": a.legend,
        })
    };
    json!({"from": view(was), "to": view(now)})
}

/// Variant rows `[i, name, active, walls, rooms, m2, furniture, issues]`.
pub(crate) fn variants(doc: &newera_core::Document) -> Value {
    Value::Array(
        doc.variants()
            .enumerate()
            .map(|(i, v)| {
                let home = v.home();
                let area: f64 = home.rooms.iter().map(newera_core::Room::area).sum();
                json!([
                    i,
                    v.name,
                    i == doc.active_variant(),
                    home.walls.len(),
                    home.rooms.len(),
                    // `+ 0.0` turns the empty sum (-0.0) into 0.0.
                    (area / 100.0).round() / 100.0 + 0.0,
                    home.furniture.len(),
                    newera_core::check_layout(home)
                        .iter()
                        .filter(|i| i.is_pending(home))
                        .count()
                ])
            })
            .collect(),
    )
}

/// Storey rows `[id, name, elevation, height, selected]`, ground first.
pub(crate) fn levels(home: &Home) -> Value {
    let current = home.current_level();
    Value::Array(
        home.sorted_levels()
            .into_iter()
            .map(|l| {
                json!([
                    l.id.to_string(),
                    l.name,
                    num(l.elevation),
                    num(l.height),
                    Some(l.id) == current,
                    l.elevation_index,
                    l.viewable,
                    l.is_reference()
                ])
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use newera_core::WallId;

    use super::*;

    #[test]
    fn an_unknown_category_says_which_there_are() {
        let out = catalog(None, Some("hvac"), 40);
        assert_eq!(out["items"], json!([]));
        assert!(
            out["categories"]
                .as_array()
                .is_some_and(|c| c.contains(&json!("living")))
        );
        let known = catalog(None, Some("living"), 40);
        assert!(known.get("categories").is_none(), "a known one stays lean");
    }

    #[test]
    fn compact_light_preserves_the_rating_and_emitting_side() {
        let mut light = newera_core::Light::led(800.0, 3000.0, (0.5, 0.5, 1.0));
        light.watts = Some(8.0);
        light.area = Some([100.0, 1.0]);
        light.panel_upward = Some(true);
        let f = newera_core::Furniture {
            light: Some(light),
            ..newera_core::Furniture::default()
        };
        let v = piece(&Home::default(), &[], &f);
        assert_eq!(
            v["light"],
            json!({"lm":800.0,"k":3000,"w":8,"area":[100,1],"panel_upward":true})
        );
    }

    #[test]
    fn compact_view_omits_defaults_and_rounds() {
        let mut h = Home::default();
        h.walls.push(Wall::new(
            WallId(1),
            Point2::new(0.0, 0.0),
            Point2::new(800.04, 0.0),
        ));
        assert_eq!(
            home(&h, 3).to_string(),
            r#"{"name":"Nova casa","rev":3,"walls":[{"a":[0,0],"b":[800,0],"id":"w1"}]}"#
        );
    }
}
