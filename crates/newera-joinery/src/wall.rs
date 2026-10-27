//! `cabinet_run`: fills one wall with cabinets sized for it. The wall is
//! measured in `newera-core`, the modules are planned in `newera-joinery`,
//! and here the old cabinets make way for the new ones. Where the run meets
//! another wall's run in an L, the cabinet under the other run's fronts
//! becomes a blind corner; the other wall is planned again in the same undo
//! step so the corner works from whichever side was drawn first.

use crate::{Build, CutoutKind, EndKind, PARAMS_KEY, RunGap, RunOver, RunParams, RunRow};
use newera_core::{
    Command, Document, Furniture, FurnitureId, Home, Point2, RunBlock, RunObstacle, WallId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// A number rounded to a tenth for replies.
fn num(v: f64) -> Value {
    let rounded = (v * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 && rounded.abs() < 1e15 {
        json!(rounded as i64)
    } else {
        json!(rounded)
    }
}

/// Property marking the modules of a run: `w3:base`.
pub const RUN_KEY: &str = "joinery:run";
/// Property holding the run's request, to plan it again after a neighbor changes.
const REQUEST_KEY: &str = "joinery:run_request";
/// Filler between a blind panel's edge and the doors beside it, cm.
const CORNER_FILLER: f64 = 3.0;
/// How far in front of a run another run's fronts still block its doors, cm.
const DOOR_SWING: f64 = 60.0;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct CabinetRunParams {
    /// Wall whose face gets the cabinets.
    pub wall: Option<String>,
    /// Or a piece against the wall (`f12`, the fridge): its nearest wall, on its side.
    pub near: Option<String>,
    /// Room the fronts face, by id or name (default: toward the house middle).
    pub room: Option<String>,
    /// Flat choices, all optional: row base|wall|tall, h, d, elev, t (mm), front,
    /// color and `handle_color` [r,g,b] or "#hex", handle bar|profile|knob|cava|none (bar),
    /// drawers (drawer units), max (widest module, 90), target (60),
    /// top (base countertop, true), `top_material`, sink / cooktop (center cm along
    /// the wall: its cabinet and cutout; null removes it), `sink_w` (80), `cooktop_w` (60),
    /// interior shelves|hanging|wardrobe (tall rows; wardrobe by default facing a bedroom).
    pub p: Option<serde_json::Map<String, Value>>,
    /// Stretch to fill, cm from the wall start (default: all of it).
    pub from: Option<f64>,
    /// Where the run ends, cm from the wall start (default: the wall's end).
    pub to: Option<f64>,
    /// Cabinets on that wall that stay as they are.
    pub keep: Option<Vec<String>>,
    /// Plan and report only.
    #[serde(default)]
    pub dry: bool,
}

/// A resolved run: one wall face and the choices for it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Request {
    wall: WallId,
    side: f64,
    params: RunParams,
    /// Keys the caller set explicitly (the rest may follow replaced cabinets).
    given: Vec<String>,
    from: Option<f64>,
    to: Option<f64>,
    #[serde(default)]
    keep: Vec<String>,
}

/// A module ready to become a furniture group.
struct Pending {
    build: Build,
    role: crate::Role,
    from: f64,
    width: f64,
    position: Point2,
    angle: f64,
    elevation: f64,
}

struct Planned {
    request: Request,
    removed: Vec<FurnitureId>,
    modules: Vec<Pending>,
    notes: Vec<String>,
    /// Runs of other walls meeting this one in a corner, by tag.
    neighbors: Vec<String>,
    /// Items embedded in replaced hosts, to put in the new ones.
    carry: Vec<Furniture>,
    /// The wall's storey, where the modules stand.
    level: Option<newera_core::LevelId>,
}

fn joinery_kind(f: &Furniture) -> Option<String> {
    let stored = f.properties.get(PARAMS_KEY)?;
    let value: Value = serde_json::from_str(stored).ok()?;
    value["kind"].as_str().map(str::to_owned)
}

fn has_cooktop(f: &Furniture) -> bool {
    f.properties.get(PARAMS_KEY).is_some_and(|stored| {
        serde_json::from_str::<Value>(stored).is_ok_and(|v| {
            v["cooktop"] == true
                || v["cutouts"]
                    .as_array()
                    .is_some_and(|c| c.iter().any(|c| c["kind"] == "cooktop"))
        })
    })
}

fn row_key(row: RunRow) -> &'static str {
    match row {
        RunRow::Base => "base",
        RunRow::Wall => "wall",
        RunRow::Tall => "tall",
    }
}

/// Lowercase without accents, for matching names.
fn plain(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// Cabinets drawn by hand or imported, recognized by name: they belong to
/// the run being redone. Appliances and fixtures never do.
fn named_cabinet(f: &Furniture, row: RunRow) -> bool {
    let full = plain(&f.name);
    // Designers number their modules: `7 — Armário portas ao lado do cooktop`.
    let name = full
        .split_once(" — ")
        .filter(|(head, _)| head.chars().all(|c| c.is_ascii_alphanumeric()))
        .map_or(full.as_str(), |(_, rest)| rest);
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    let cabinet = has(&[
        "armario",
        "gaveteiro",
        "gavetao",
        "gavetoes",
        "balcao",
        "gabinete",
        "modulo",
        "nicho",
        "paneleiro",
        "tamponamento",
        "arremate",
        "separador",
        "rodape",
    ]);
    let appliance = [
        "geladeira",
        "refrigerador",
        "fogao",
        "cooktop",
        "forno",
        "micro",
        "lava",
        "maquina",
        "cuba",
        "coifa",
        "tanque",
        "torneira",
        "pia",
    ]
    .iter()
    .any(|w| name.starts_with(w));
    if appliance && !cabinet {
        return false;
    }
    match row {
        RunRow::Base => cabinet || has(&["bancada", "tampo", "peninsula"]),
        RunRow::Wall => cabinet || name.starts_with("aereo"),
        RunRow::Tall => cabinet || has(&["torre", "despensa", "guarda-roupa", "roupeiro"]),
    }
}

/// Whether `p` is inside a polygon (ray casting).
fn inside(points: &[Point2], p: Point2) -> bool {
    let mut odd = false;
    let n = points.len();
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            odd = !odd;
        }
    }
    odd
}

/// Distance from a point to a wall's centerline segment, cm.
fn distance_to(wall: &newera_core::Wall, p: Point2) -> f64 {
    let (dx, dy) = (wall.end.x - wall.start.x, wall.end.y - wall.start.y);
    let len2 = (dx * dx + dy * dy).max(1e-9);
    let t = (((p.x - wall.start.x) * dx + (p.y - wall.start.y) * dy) / len2).clamp(0.0, 1.0);
    Point2::new(wall.start.x + dx * t, wall.start.y + dy * t).distance(p)
}

fn brazilian(v: f64) -> String {
    let rounded = (v * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}").replace('.', ",")
    }
}

/// Which wall, which face, which choices, and a note saying which face when
/// a room chose it.
#[allow(clippy::too_many_lines)]
fn resolve(home: &Home, p: &CabinetRunParams) -> Result<(Request, Option<String>), String> {
    let near = match &p.near {
        Some(id) => Some(
            home.furniture
                .iter()
                .find(|f| f.id.to_string() == *id)
                .ok_or_else(|| format!("{id} not found"))?
                .position,
        ),
        None => None,
    };
    let wall_id: WallId = match (&p.wall, near) {
        (Some(wall), _) => wall.parse().map_err(|e| format!("{e}"))?,
        (None, Some(at)) => {
            let view = home.level_view(home.current_level());
            view.walls
                .iter()
                .filter(|w| !w.is_arc())
                .min_by(|a, b| distance_to(a, at).total_cmp(&distance_to(b, at)))
                .ok_or("there are no straight walls")?
                .id
        }
        (None, None) => return Err("give `wall`, or `near` with a piece against it".into()),
    };
    let wall = home
        .wall(wall_id)
        .ok_or_else(|| format!("{wall_id} not found"))?;
    if wall.is_arc() {
        return Err(format!(
            "{wall_id} is curved: cabinets need a straight wall"
        ));
    }
    let given = p.p.clone().unwrap_or_default();
    // Planning the same row again starts from the choices made last time
    // (sink, cooktop, fronts…); what is given now changes only those keys.
    let row = given.get("row").and_then(Value::as_str).unwrap_or("base");
    let tag = format!("{wall_id}:{row}");
    let mut merged_params = home
        .furniture
        .iter()
        .find(|f| f.properties.get(RUN_KEY) == Some(&tag))
        .and_then(|f| f.properties.get(REQUEST_KEY))
        .and_then(|s| serde_json::from_str::<Request>(s).ok())
        .and_then(|r| serde_json::to_value(r.params).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    for (key, value) in &given {
        merged_params.insert(key.clone(), value.clone());
    }
    let params: RunParams = crate::parse_params(&Value::Object(merged_params))?;
    let length = wall.start.distance(wall.end);
    let u = (
        (wall.end.x - wall.start.x) / length,
        (wall.end.y - wall.start.y) / length,
    );
    let mid = Point2::new(
        wall.start.x.midpoint(wall.end.x),
        wall.start.y.midpoint(wall.end.y),
    );
    // The side whose half-plane holds `facing`.
    let toward = |facing: Point2| {
        if -u.1 * (facing.x - mid.x) + u.0 * (facing.y - mid.y) >= 0.0 {
            1.0
        } else {
            -1.0
        }
    };
    let mut said = None;
    let side = match (&p.room, near) {
        (Some(room), _) => {
            // By id or by name, as the user calls it: `r2` or `Cozinha`.
            let level = home.resolve_level(wall.level);
            let found = home
                .rooms
                .iter()
                .find(|r| r.id.to_string() == *room)
                .or_else(|| {
                    // A name repeated on several storeys means the wall's one.
                    let name = plain(room.trim());
                    let named: Vec<_> = home
                        .rooms
                        .iter()
                        .filter(|r| plain(r.name.trim()) == name)
                        .collect();
                    named
                        .iter()
                        .find(|r| home.resolve_level(r.level) == level)
                        .or(named.first())
                        .copied()
                })
                .ok_or_else(|| format!("no room `{room}`: give its id or its name"))?;
            // The face the room is on, looked for just past the wall along
            // the stretch being filled: an L-shaped room's centroid can lie
            // on the other side of one of its own walls.
            let (a, b) = (
                p.from.unwrap_or(0.0).clamp(0.0, length),
                p.to.unwrap_or(length).clamp(0.0, length),
            );
            let reach = wall.thickness / 2.0 + 1.0;
            let mut hits = [0_u32; 2];
            for k in 0..9 {
                let along = a + (b - a) * (f64::from(k) + 0.5) / 9.0;
                let at = Point2::new(wall.start.x + u.0 * along, wall.start.y + u.1 * along);
                for (hit, sign) in hits.iter_mut().zip([1.0, -1.0]) {
                    let probe = Point2::new(at.x - u.1 * sign * reach, at.y + u.0 * sign * reach);
                    if inside(&found.points, probe) {
                        *hit += 1;
                    }
                }
            }
            let label = if found.name.trim().is_empty() {
                found.id.to_string()
            } else {
                format!("{} ({})", found.name.trim(), found.id)
            };
            let side = match hits {
                [0, 0] => {
                    return Err(format!(
                        "{label} não encosta em {wall_id}: dê o cômodo de um dos lados dela, ou nenhum."
                    ));
                }
                [l, r] if l > r => 1.0,
                [l, r] if r > l => -1.0,
                _ => toward(newera_core::polygon_centroid(&found.points).unwrap_or(mid)),
            };
            let center = a.midpoint(b);
            let front = Point2::new(
                wall.start.x + u.0 * center - u.1 * side * 60.0,
                wall.start.y + u.1 * center + u.0 * side * 60.0,
            );
            said = Some(format!(
                "Frentes para {label}: no lado de {wall_id} voltado para [{}, {}].",
                num(front.x),
                num(front.y)
            ));
            side
        }
        (None, Some(at)) => toward(at),
        (None, None) => toward(home.bounds().map_or(mid, |(a, b)| {
            Point2::new(a.x.midpoint(b.x), a.y.midpoint(b.y))
        })),
    };
    // Tall cabinets facing a bedroom are a wardrobe.
    let mut params = params;
    if params.row == RunRow::Tall && params.interior.is_none() {
        let normal = (-u.1 * side, u.0 * side);
        let front = Point2::new(mid.x + normal.0 * 60.0, mid.y + normal.1 * 60.0);
        let bedroom = home
            .rooms
            .iter()
            .find(|r| inside(&r.points, front))
            .is_some_and(|r| {
                let name = r.name.to_lowercase();
                ["quarto", "dormit", "suíte", "suite", "closet", "bedroom"]
                    .iter()
                    .any(|w| name.contains(w))
                    || home
                        .furniture
                        .iter()
                        .any(|f| f.catalog.starts_with("bed-") && inside(&r.points, f.position))
            });
        if bedroom {
            params.interior = Some(crate::Interior::Wardrobe);
        }
    }
    let request = Request {
        wall: wall_id,
        side,
        params,
        given: given.keys().cloned().collect(),
        from: p.from,
        to: p.to,
        keep: p.keep.clone().unwrap_or_default(),
    };
    Ok((request, said))
}

/// Plans one run against `home` without changing it.
#[allow(clippy::too_many_lines)]
fn plan(home: &Home, request: &Request) -> Result<Planned, String> {
    let wall_id = request.wall;
    let side = request.side;
    let wall = home
        .wall(wall_id)
        .cloned()
        .ok_or_else(|| format!("{wall_id} not found"))?;
    let mut params = request.params.clone();
    let length = wall.start.distance(wall.end);
    let u = (
        (wall.end.x - wall.start.x) / length,
        (wall.end.y - wall.start.y) / length,
    );
    let wall_angle = u.1.atan2(u.0).to_degrees();
    let (from, to) = (
        request.from.unwrap_or(0.0).clamp(0.0, length),
        request.to.unwrap_or(length).clamp(0.0, length),
    );
    if to - from < 1.0 {
        return Err(format!(
            "from/to leave nothing of {wall_id} ({} cm long)",
            brazilian(length)
        ));
    }
    let row = row_key(params.row);
    let tag = format!("{wall_id}:{row}");
    let other_run = |f: &Furniture| {
        f.properties
            .get(RUN_KEY)
            .filter(|t| t.ends_with(&format!(":{row}")) && **t != tag)
            .cloned()
    };
    let catalog_cabinet = match params.row {
        RunRow::Base => "base-cabinet",
        RunRow::Wall => "wall-cabinet",
        RunRow::Tall => "wardrobe",
    };
    let (h, d, elev) = params.sizes();
    let z = match params.row {
        RunRow::Base => (0.0, h + 3.0),
        RunRow::Wall => (elev, elev + h),
        RunRow::Tall => (0.0, elev + h),
    };
    let band = if params.row == RunRow::Base {
        d + 3.0
    } else {
        d
    };
    // Pieces this run replaces: its own modules and loose cabinets in the band.
    let replaceable = |f: &Furniture| {
        if request.keep.contains(&f.id.to_string()) {
            return false;
        }
        // Another wall's run meeting this one stays, and so does anything
        // not parallel to this wall.
        if f.properties.get(RUN_KEY).is_some_and(|t| *t != tag) {
            return false;
        }
        if (f.angle - wall_angle).to_radians().sin().abs() > 0.1 {
            return false;
        }
        // Only pieces of this row: a tower is not a wall cabinet.
        let (lo, hi) = f.height_range();
        let this_row = match params.row {
            RunRow::Base => hi <= 100.0,
            RunRow::Wall => lo >= 100.0,
            RunRow::Tall => lo < 50.0 && hi > 150.0,
        };
        this_row
            && match joinery_kind(f).as_deref() {
                Some("cabinet" | "filler") => true,
                Some("countertop") => params.row == RunRow::Base,
                _ => f.catalog == catalog_cabinet || named_cabinet(f, params.row),
            }
    };
    let piece = |id: FurnitureId| home.furniture.iter().find(|f| f.id == id);
    let first = newera_core::wall_run(home, wall_id, side, band, z, &|_| false)
        .ok_or_else(|| format!("{wall_id} can't hold a run"))?;
    let inside = |o: &RunObstacle| {
        let c = o.from.midpoint(o.to);
        c >= from - 0.5 && c <= to + 0.5
    };
    let mut removed: Vec<FurnitureId> = Vec::new();
    for o in first.obstacles.iter().filter(|o| inside(o)) {
        if let RunBlock::Piece { id, .. } = &o.block
            && !removed.contains(id)
            && piece(*id).is_some_and(replaceable)
        {
            removed.push(*id);
        }
    }
    // Keys this call set: `p` is per call, the rest comes from the last one.
    let given = |key: &str| request.given.iter().any(|g| g == key);
    // Keep the finish of the cabinets being replaced unless told otherwise.
    if let Some(Build::Cabinet(old)) = removed
        .iter()
        .filter_map(|id| piece(*id))
        .filter_map(|f| serde_json::from_str::<Build>(f.properties.get(PARAMS_KEY)?).ok())
        .find(|b| matches!(b, Build::Cabinet(_)))
    {
        if !given("t") {
            params.t = old.t;
        }
        if !given("front") {
            params.front = old.front;
        }
        if !given("color") {
            params.color = old.color;
        }
        if !given("handle") {
            params.handle = old.handle;
        }
        if !given("handle_color") {
            params.handle_color = old.handle_color;
        }
    }
    // A sink or cooktop already set into the old countertop keeps its place:
    // the new run puts its cabinet and cutout right there.
    let mut embedded: Vec<FurnitureId> = Vec::new();
    let mut found_notes = Vec::new();
    if params.row == RunRow::Base && !removed.is_empty() {
        let along = |p: Point2| (p.x - wall.start.x) * u.0 + (p.y - wall.start.y) * u.1;
        for o in first.obstacles.iter().filter(|o| inside(o)) {
            let RunBlock::Piece { id, .. } = &o.block else {
                continue;
            };
            let Some(f) = piece(*id).filter(|f| !removed.contains(&f.id)) else {
                continue;
            };
            let (lo, hi) = f.height_range();
            let name = plain(&f.name);
            if lo < 50.0 || hi - lo > 45.0 || joinery_kind(f).is_some() {
                continue;
            }
            let center = along(f.position);
            if name.contains("cooktop") && params.cooktop.is_none() && !given("cooktop") {
                params.cooktop = Some(center);
                params.cooktop_w = ((f.width + 6.0) / 5.0).ceil().max(12.0) * 5.0;
                params.cooktop_real = Some([f.width, f.depth]);
                embedded.push(f.id);
                found_notes.push("cooktop".to_owned());
            } else if (name.contains("pia") || name.contains("cuba") || name.starts_with("tanque"))
                && params.sink.is_none()
                && !given("sink")
                && !name.contains("gavet")
                && !name.contains("armario")
            {
                params.sink = Some(center);
                params.sink_w = ((f.width + 10.0) / 5.0).ceil().max(12.0) * 5.0;
                params.sink_real = Some([f.width, f.depth]);
                embedded.push(f.id);
                found_notes.push("sink".to_owned());
            }
        }
    }
    // Items embedded in the cabinets and countertops being replaced move to the
    // new ones: a sink or cooktop keeps its spot and gets its cutout there.
    let mut carry: Vec<Furniture> = Vec::new();
    {
        let along = |p: Point2| (p.x - wall.start.x) * u.0 + (p.y - wall.start.y) * u.1;
        for id in &removed {
            for child in piece(*id)
                .into_iter()
                .flat_map(|f| f.children.iter())
                .filter(|c| c.properties.contains_key(crate::EMBED_KEY))
            {
                let center = along(child.position);
                let fixture = crate::fixture_of(child);
                let mut child = child.clone();
                let key = match fixture {
                    crate::Fixture::Sink if params.row == RunRow::Base => "sink",
                    crate::Fixture::Cooktop if params.row == RunRow::Base => "cooktop",
                    _ => "",
                };
                if !key.is_empty() {
                    let at = if key == "sink" {
                        &mut params.sink
                    } else {
                        &mut params.cooktop
                    };
                    // A real item already set in wins over the position asked
                    // before; one asked for now moves it, and `null` leaves it
                    // with the old countertop.
                    if !given(key) {
                        *at = Some(center);
                    } else if let Some(to) = *at {
                        child.position = Point2::new(
                            child.position.x + u.0 * (to - center),
                            child.position.y + u.1 * (to - center),
                        );
                    } else {
                        continue;
                    }
                    if key == "sink" {
                        params.sink_w = ((child.width + 10.0) / 5.0).ceil().max(12.0) * 5.0;
                        params.sink_real = Some([child.width, child.depth]);
                    } else {
                        params.cooktop_w = ((child.width + 6.0) / 5.0).ceil().max(12.0) * 5.0;
                        params.cooktop_real = Some([child.width, child.depth]);
                    }
                }
                carry.push(child);
            }
        }
    }
    // The sink or cooktop asked for in this call keeps its place where
    // another wall's run turns the corner over it: that run gives way and is
    // planned again against this one, stopping short of it. A stored request
    // never lists them as given, so a neighbor planned again doesn't push back.
    let mut yielded: Vec<FurnitureId> = Vec::new();
    let mut neighbors: Vec<String> = Vec::new();
    // As deep as the fronts that would cover it (see `covering` below): the
    // other run's modules behind its corner one would still take the stretch.
    let reach = (params.row == RunRow::Base)
        .then(|| newera_core::wall_run(home, wall_id, side, d + DOOR_SWING, z, &|_| false))
        .flatten();
    if let Some(reach) = &reach {
        for (key, center, width) in [
            ("sink", params.sink, params.sink_w),
            ("cooktop", params.cooktop, params.cooktop_w),
        ] {
            let Some(center) = center.filter(|_| given(key)) else {
                continue;
            };
            let (a, b) = (center - width / 2.0, center + width / 2.0);
            for o in reach.obstacles.iter().filter(|o| o.from < b && o.to > a) {
                if let RunBlock::Piece { id, .. } = &o.block
                    && let Some(t) = piece(*id).and_then(other_run)
                    && !yielded.contains(id)
                {
                    yielded.push(*id);
                    if !neighbors.contains(&t) {
                        neighbors.push(t);
                    }
                }
            }
        }
    }
    let skip_top = params.row == RunRow::Base && !params.top;
    let skip = |f: &Furniture| {
        removed.contains(&f.id)
            || embedded.contains(&f.id)
            || yielded.contains(&f.id)
            || (skip_top && joinery_kind(f).as_deref() == Some("countertop"))
    };
    let mut run = newera_core::wall_run(home, wall_id, side, band, z, &skip)
        .ok_or_else(|| format!("{wall_id} can't hold a run"))?;
    // A sink or cooktop of the run on the wall across the corner is worked
    // at from the front: its cabinet and the room to open it stay clear of
    // this run, which stops short of them rather than standing in front.
    if params.row == RunRow::Base {
        let normal = (-u.1 * side, u.0 * side);
        let face = wall.thickness / 2.0;
        for top in home.furniture.iter().filter(|f| {
            other_run(f).is_some()
                && joinery_kind(f).as_deref() == Some("countertop")
                && (f.angle - wall_angle).to_radians().sin().abs() > 0.9
                && !skip(f)
        }) {
            let Some(cutouts) = top
                .properties
                .get(PARAMS_KEY)
                .and_then(|stored| serde_json::from_str::<Value>(stored).ok())
                .and_then(|v| v["cutouts"].as_array().cloned())
            else {
                continue;
            };
            for c in cutouts
                .iter()
                .filter(|c| c["kind"] == "sink" || c["kind"] == "cooktop")
            {
                let Some(x) = c["x"].as_f64() else {
                    continue;
                };
                let half = (c["w"].as_f64().unwrap_or(56.0) + 20.0).max(60.0) / 2.0;
                let (cx, reach) = (
                    x - top.width / 2.0,
                    top.depth / 2.0 + DOOR_SWING + CORNER_FILLER,
                );
                let corners = [
                    (-half, -reach),
                    (half, -reach),
                    (-half, reach),
                    (half, reach),
                ]
                .map(|(dx, dy)| top.to_plan((cx + dx, dy)));
                let span = |axis: (f64, f64)| {
                    corners
                        .iter()
                        .map(|p| (p.x - wall.start.x) * axis.0 + (p.y - wall.start.y) * axis.1)
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                            (lo.min(v), hi.max(v))
                        })
                };
                let (out, along) = (span(normal), span(u));
                let (a, b) = (along.0.max(0.0), along.1.min(length));
                if out.1 <= face || out.0 >= face + band || b - a < 1.0 {
                    continue;
                }
                run.obstacles.push(RunObstacle {
                    from: a,
                    to: b,
                    block: RunBlock::Piece {
                        id: top.id,
                        catalog: top.catalog.clone(),
                    },
                });
            }
        }
        run.obstacles.sort_by(|a, b| a.from.total_cmp(&b.from));
    }
    let named = |id: FurnitureId, words: &[&str]| {
        piece(id).is_some_and(|f| {
            let n = plain(&f.name);
            words.iter().any(|w| n.starts_with(w))
        })
    };
    // A stove or a cooktop cabinet; a countertop only holds its cooktop in one
    // place, handled where it matters (the hood gap).
    let heat = |id: FurnitureId, catalog: &str| {
        catalog == "stove"
            || catalog == "cooktop"
            || named(id, &["fogao"])
            || piece(id)
                .is_some_and(|f| has_cooktop(f) && joinery_kind(f).as_deref() != Some("countertop"))
    };
    let fridge = |id: FurnitureId, catalog: &str| {
        catalog == "fridge" || named(id, &["geladeira", "refrigerador"])
    };
    let hot_top = |id: FurnitureId| {
        piece(id)
            .is_some_and(|f| has_cooktop(f) && joinery_kind(f).as_deref() == Some("countertop"))
    };
    let mut notes = Vec::new();
    let mut over = Vec::new();
    if params.row == RunRow::Wall {
        // Leave the hood's width free over a stove or cooktop.
        let below = newera_core::wall_run(home, wall_id, side, 60.0, (0.0, 95.0), &skip);
        let along = |p: Point2| (p.x - wall.start.x) * u.0 + (p.y - wall.start.y) * u.1;
        for o in below.iter().flat_map(|b| b.obstacles.iter()) {
            let RunBlock::Piece { id, catalog } = &o.block else {
                continue;
            };
            if !(heat(*id, catalog) || hot_top(*id))
                || run.obstacles.iter().any(|r| r.block == o.block)
            {
                continue;
            }
            // A countertop holds its cooktop somewhere along it: only there.
            let spans: Vec<(f64, f64)> =
                match piece(*id).filter(|f| joinery_kind(f).as_deref() == Some("countertop")) {
                    Some(top) => serde_json::from_str::<Value>(&top.properties[PARAMS_KEY])
                        .ok()
                        .and_then(|v| v["cutouts"].as_array().cloned())
                        .unwrap_or_default()
                        .iter()
                        .filter(|c| c["kind"] == "cooktop")
                        .filter_map(|c| {
                            let x = c["x"].as_f64()? - top.width / 2.0;
                            let half = c["w"].as_f64().unwrap_or(56.0).max(60.0) / 2.0;
                            let center = along(top.to_plan((x, 0.0)));
                            Some((center - half, center + half))
                        })
                        .collect(),
                    None => vec![(o.from, o.to)],
                };
            for (a, b) in spans {
                // A cooktop cabinet under its countertop's cutout is one hood.
                if run.obstacles.iter().any(|r| {
                    r.from < b - 5.0
                        && r.to > a + 5.0
                        && matches!(&r.block, RunBlock::Piece { id, catalog } if heat(*id, catalog) || hot_top(*id))
                }) {
                    continue;
                }
                run.obstacles.push(RunObstacle {
                    from: a.max(0.0),
                    to: b.min(length),
                    block: o.block.clone(),
                });
                notes.push(format!(
                    "Vão de {} cm deixado para a coifa sobre o fogão.",
                    brazilian(b - a)
                ));
            }
        }
        run.obstacles.sort_by(|a, b| a.from.total_cmp(&b.from));
        for o in &run.obstacles {
            if let RunBlock::Piece { id, catalog } = &o.block
                && fridge(*id, catalog)
                && let Some(f) = piece(*id)
            {
                over.push(RunOver {
                    from: o.from,
                    to: o.to,
                    bottom: f.height_range().1,
                });
            }
        }
    }
    let end_kind = |o: Option<&RunObstacle>| match o.map(|o| &o.block) {
        None => EndKind::Free,
        Some(RunBlock::Wall(_)) => EndKind::Wall,
        Some(RunBlock::Opening { .. }) => EndKind::Frame,
        Some(RunBlock::Piece { id, catalog }) => {
            if fridge(*id, catalog) {
                EndKind::Fridge
            } else if heat(*id, catalog) {
                // Over the stove the hood gap is exact; beside it, air.
                if params.row == RunRow::Wall {
                    EndKind::Kept
                } else {
                    EndKind::Heat
                }
            } else if let Some(f) = piece(*id).filter(|f| f.properties.contains_key(PARAMS_KEY)) {
                // Another wall's run (any row) owns the corner: fronts need a filler.
                if f.properties
                    .get(RUN_KEY)
                    .is_some_and(|t| !t.starts_with(&format!("{wall_id}:")))
                {
                    EndKind::Wall
                } else {
                    EndKind::Kept
                }
            } else {
                EndKind::Piece
            }
        }
    };
    for o in &run.obstacles {
        if let RunBlock::Piece { id, .. } = &o.block
            && let Some(t) = piece(*id).and_then(other_run)
            && !neighbors.contains(&t)
        {
            neighbors.push(t);
        }
    }
    // Other runs' cabinets just in front of this one: their fronts cover ours.
    let front = newera_core::wall_run(home, wall_id, side, d + DOOR_SWING, z, &skip);
    let covering: Vec<(f64, f64, FurnitureId)> = front
        .iter()
        .flat_map(|r| r.obstacles.iter())
        .filter_map(|o| {
            let RunBlock::Piece { id, .. } = &o.block else {
                return None;
            };
            let f = piece(*id)?;
            let t = other_run(f)?;
            if !matches!(joinery_kind(f).as_deref(), Some("cabinet" | "filler")) {
                return None;
            }
            if !neighbors.contains(&t) {
                neighbors.push(t);
            }
            Some((o.from, o.to, *id))
        })
        .collect();
    let gaps: Vec<RunGap> = run
        .gaps()
        .into_iter()
        .filter_map(|(a, b, before, after)| {
            let (a2, b2) = (a.max(from), b.min(to));
            if b2 - a2 <= 0.5 {
                return None;
            }
            let start = if a2 > a {
                EndKind::Kept
            } else {
                end_kind(before)
            };
            let end = if b2 < b {
                EndKind::Kept
            } else {
                end_kind(after)
            };
            let blind_start = if start == EndKind::Wall {
                covering
                    .iter()
                    .filter(|(f, t, _)| *f <= a2 + 1.0 && *t > a2 + 1.0)
                    .map(|(_, t, _)| t - a2 + CORNER_FILLER)
                    .fold(0.0, f64::max)
            } else {
                0.0
            };
            let blind_end = if end == EndKind::Wall {
                covering
                    .iter()
                    .filter(|(f, t, _)| *t >= b2 - 1.0 && *f < b2 - 1.0)
                    .map(|(f, _, _)| b2 - f + CORNER_FILLER)
                    .fold(0.0, f64::max)
            } else {
                0.0
            };
            Some(RunGap {
                from: a2,
                to: b2,
                start,
                end,
                blind_start,
                blind_end,
            })
        })
        .collect();
    // A sink or cooktop in a stretch the corners' blind panels take whole:
    // no cabinet can go under it, and the cabinets in the way are named.
    if params.row == RunRow::Base {
        for g in &gaps {
            for (name, key, center) in [
                ("A pia", "sink", params.sink),
                ("O cooktop", "cooktop", params.cooktop),
            ] {
                let left = g.to - g.from - g.blind_start - g.blind_end;
                if center.is_none_or(|c| c <= g.from || c >= g.to) || left > 0.0 {
                    continue;
                }
                let mut ids: Vec<String> = covering
                    .iter()
                    .filter(|(f, t, _)| *f < g.to && *t > g.from)
                    .map(|(_, _, id)| {
                        let run = piece(*id).and_then(other_run).unwrap_or_default();
                        format!("{id} ({run})")
                    })
                    .collect();
                ids.dedup();
                return Err(format!(
                    "{name} em {} cm de {wall_id} cai no canto que {} já cobre: sobram {} cm de {} a {} cm. Afaste p.{key} do canto, ou use p.{key}=null.",
                    brazilian(center.unwrap_or_default()),
                    ids.join(", "),
                    brazilian(left),
                    brazilian(g.from),
                    brazilian(g.to)
                ));
            }
        }
    }
    let over: Vec<RunOver> = over
        .into_iter()
        .filter(|o| o.from >= from - 0.5 && o.to <= to + 0.5)
        .collect();
    // Joints of the other row on this wall, to line the modules up with.
    let other_row = match params.row {
        RunRow::Base => Some("wall"),
        RunRow::Wall => Some("base"),
        RunRow::Tall => None,
    };
    let joints: Vec<f64> = other_row
        .map(|r| format!("{wall_id}:{r}"))
        .map(|t| {
            home.furniture
                .iter()
                .filter(|f| f.properties.get(RUN_KEY) == Some(&t))
                .filter(|f| matches!(joinery_kind(f).as_deref(), Some("cabinet" | "filler")))
                .flat_map(|f| {
                    let c =
                        (f.position.x - wall.start.x) * u.0 + (f.position.y - wall.start.y) * u.1;
                    [c - f.width / 2.0, c + f.width / 2.0]
                })
                .collect()
        })
        .unwrap_or_default();
    // Old cabinets outside every free stretch (behind a shaft, beside the
    // laundry sink) stay: nothing would take their place. Not where another
    // run now stands, or in front of its sink: that run took the place.
    let along_of = |id: &FurnitureId| {
        piece(*id)
            .map(|f| (f.position.x - wall.start.x) * u.0 + (f.position.y - wall.start.y) * u.1)
    };
    let taken = |c: f64| {
        run.obstacles.iter().any(|o| {
            c >= o.from
                && c <= o.to
                && matches!(&o.block, RunBlock::Piece { id, .. }
                    if piece(*id).and_then(other_run).is_some())
        })
    };
    let kept_back: Vec<FurnitureId> = removed
        .iter()
        .filter(|id| {
            along_of(id).is_some_and(|c| {
                !gaps.iter().any(|g| c >= g.from - 1.0 && c <= g.to + 1.0)
                    && !over.iter().any(|o| c >= o.from - 1.0 && c <= o.to + 1.0)
                    && !taken(c)
            })
        })
        .copied()
        .collect();
    let (modules, plan_notes) = crate::plan_run(&gaps, &over, &joints, &params)?;
    // A sink or cooktop where nothing can stand (in front of another run's
    // sink, behind the fridge) is said, not dropped in silence.
    if params.row == RunRow::Base {
        for (name, center, role) in [
            ("A pia", params.sink, crate::Role::Sink),
            ("O cooktop", params.cooktop, crate::Role::Cooktop),
        ] {
            if let Some(c) = center.filter(|c| (from..=to).contains(c))
                && !gaps.iter().any(|g| c > g.from && c < g.to)
                && !modules.iter().any(|m| m.role == role)
            {
                notes.push(format!(
                    "{name} em {} cm de {wall_id} ficou sem módulo: ali não há trecho livre.",
                    brazilian(c)
                ));
            }
        }
    }
    let removed: Vec<FurnitureId> = removed
        .into_iter()
        .filter(|id| !kept_back.contains(id))
        .collect();
    // Say where the sink or cooktop already there got its cabinet.
    for m in &modules {
        let (kind, text) = match m.role {
            crate::Role::Sink => ("sink", "Pia existente mantida"),
            crate::Role::Cooktop => ("cooktop", "Cooktop existente mantido"),
            _ => continue,
        };
        if found_notes.iter().any(|f| f == kind) {
            notes.insert(
                0,
                format!(
                    "{text}: módulo de {} cm embaixo, de {} a {} cm.",
                    brazilian(m.width),
                    brazilian(m.from),
                    brazilian(m.from + m.width)
                ),
            );
        }
    }
    notes.extend(plan_notes);
    let mut seen = std::collections::HashSet::new();
    notes.retain(|n| seen.insert(n.clone()));

    let angle = wall_angle + if side < 0.0 { 180.0 } else { 0.0 };
    let normal = (-u.1 * side, u.0 * side);
    let mut pending: Vec<Pending> = modules
        .into_iter()
        .map(|m| {
            let mut build = m.build;
            // Turned around, the cabinet's left is the run's end.
            if side < 0.0 {
                match &mut build {
                    Build::Cabinet(c) => std::mem::swap(&mut c.blind_left, &mut c.blind_right),
                    // …and a countertop's cutouts count from the other end.
                    Build::Countertop(top) => {
                        for cut in &mut top.cutouts {
                            cut.x = top.length - cut.x;
                        }
                    }
                    _ => {}
                }
            }
            let along = m.from + m.width / 2.0;
            let out = wall.thickness / 2.0 + m.depth / 2.0;
            Pending {
                build,
                role: m.role,
                from: m.from,
                width: m.width,
                position: Point2::new(
                    wall.start.x + u.0 * along + normal.0 * out,
                    wall.start.y + u.1 * along + normal.1 * out,
                ),
                angle,
                elevation: m.elevation,
            }
        })
        .collect();
    cut_tops(&mut pending, &mut notes);
    let request = Request {
        params,
        given: request.given.clone(),
        ..request.clone()
    };
    Ok(Planned {
        request,
        removed,
        modules: pending,
        notes,
        neighbors,
        carry,
        level: home.resolve_level(wall.level),
    })
}

/// Cuts the wooden top of each cabinet a bowl or a cooktop in the run's
/// countertop goes down through, as `embed` does for one set in later.
fn cut_tops(pending: &mut [Pending], notes: &mut Vec<String>) {
    let frame = |m: &Pending, [width, depth, height]: [f64; 3]| Furniture {
        position: m.position,
        angle: m.angle,
        elevation: m.elevation,
        width,
        depth,
        height,
        ..Furniture::default()
    };
    // What hangs under each hole: the bowl (18 cm) or the cooktop's body (6).
    let mut bodies: Vec<(Furniture, (f64, f64), f64)> = Vec::new();
    for m in pending.iter() {
        let Build::Countertop(top) = &m.build else {
            continue;
        };
        let slab = frame(m, [top.length, top.depth, top.height]);
        let counter = m.elevation + top.height;
        for c in &top.cutouts {
            let (w, d, deep, name) = match c.kind {
                CutoutKind::Sink => (c.w.unwrap_or(50.0), c.d.unwrap_or(40.0), 18.0, "A cuba"),
                CutoutKind::Cooktop => (c.w.unwrap_or(56.0), c.d.unwrap_or(48.0), 6.0, "O cooktop"),
                CutoutKind::Grommet => continue,
            };
            let y0 = (top.depth - d) / 2.0;
            let body = Furniture {
                name: name.to_owned(),
                position: slab.to_plan((c.x - top.length / 2.0, y0 + d / 2.0 - top.depth / 2.0)),
                angle: m.angle,
                elevation: counter - deep,
                ..Furniture::default()
            };
            bodies.push((body, (w, d), counter));
        }
    }
    for m in pending.iter_mut() {
        let Build::Cabinet(p) = &m.build else {
            continue;
        };
        let Ok(output) = crate::generate(&m.build) else {
            continue;
        };
        let cabinet = frame(m, output.size);
        let mut p = p.clone();
        for (body, hole, counter) in &bodies {
            match crate::embed::top_cut(&cabinet, &p, body, *hole, *counter) {
                Some(Ok(cut)) => p.top_cutout = Some(cut),
                Some(Err(why)) => notes.push(why),
                None => {}
            }
        }
        m.build = Build::Cabinet(p);
    }
}

/// Turns a plan into commands, taking furniture ids from `doc`.
fn commands(doc: &mut Document, planned: &Planned) -> Result<(Vec<Command>, Value), String> {
    let tag = format!(
        "{}:{}",
        planned.request.wall,
        row_key(planned.request.params.row)
    );
    // Replanned later, every explicit choice is already in `params`.
    let stored = serde_json::to_string(&Request {
        given: vec![
            "t".into(),
            "front".into(),
            "color".into(),
            "handle".into(),
            "handle_color".into(),
        ],
        keep: Vec::new(),
        ..planned.request.clone()
    })
    .map_err(|e| e.to_string())?;
    let mut list: Vec<Command> = planned
        .removed
        .iter()
        .map(|id| Command::remove(*id))
        .collect();
    let mut rows = Vec::new();
    let mut groups: Vec<(crate::Role, Furniture)> = Vec::new();
    for m in &planned.modules {
        let output = crate::generate(&m.build)?;
        let group_id = doc.new_furniture_id();
        let mut next = || doc.new_furniture_id();
        let mut group = crate::assemble(
            &m.build,
            &output,
            group_id,
            m.position,
            m.angle,
            m.elevation,
            &mut next,
        );
        group.level = planned.level;
        group.properties.insert(RUN_KEY.into(), tag.clone());
        group.properties.insert(REQUEST_KEY.into(), stored.clone());
        rows.push(json!([
            group_id.to_string(),
            m.role,
            num(m.from),
            num(m.width)
        ]));
        groups.push((m.role, group));
    }
    // Embedded items find their new host: sinks and cooktops the countertop
    // over them, anything else the module around it; otherwise they stand free.
    for child in &planned.carry {
        let fixture = crate::fixture_of(child);
        let on_top = matches!(fixture, crate::Fixture::Sink | crate::Fixture::Cooktop);
        let host = groups.iter_mut().find(|(role, g)| {
            g.contains(child.position) && (*role == crate::Role::Countertop) == on_top
        });
        if let Some((_, group)) = host {
            let mut item = child.clone();
            if on_top {
                let lift = if fixture == crate::Fixture::Sink {
                    0.5
                } else {
                    0.6
                };
                item.elevation = group.elevation + group.height - item.height + lift;
            }
            group.children.push(item);
        } else {
            let mut item = child.clone();
            item.properties.remove(crate::EMBED_KEY);
            list.push(Command::insert(item));
        }
    }
    list.extend(groups.into_iter().map(|(_, g)| Command::insert(g)));
    Ok((list, Value::Array(rows)))
}

/// Whether a new plan for a run builds exactly what is already there.
fn unchanged(home: &Home, tag: &str, planned: &Planned) -> bool {
    let key = |build: &str, p: Point2, elevation: f64| {
        format!("{build}@{:.1},{:.1},{:.1}", p.x, p.y, elevation)
    };
    let mut before: Vec<String> = home
        .furniture
        .iter()
        .filter(|f| f.properties.get(RUN_KEY).is_some_and(|t| t == tag))
        .filter_map(|f| Some(key(f.properties.get(PARAMS_KEY)?, f.position, f.elevation)))
        .collect();
    let mut after: Vec<String> = planned
        .modules
        .iter()
        .filter_map(|m| {
            Some(key(
                &serde_json::to_string(&m.build).ok()?,
                m.position,
                m.elevation,
            ))
        })
        .collect();
    before.sort();
    after.sort();
    before == after
}

#[allow(clippy::needless_pass_by_value)] // built for the reply
fn summary(planned: &Planned, modules: Value) -> Value {
    json!({
        "wall": planned.request.wall.to_string(),
        "row": row_key(planned.request.params.row),
        "modules": modules,
        "removed": planned.removed.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "notes": planned.notes,
    })
}

/// Plans (and unless `dry`, builds) the cabinets for one wall, and plans
/// again the runs it meets in a corner. Replies with the modules as
/// `[id, role, from, width]`, what was removed, notes and adjusted neighbors.
///
/// # Errors
/// Unknown walls or pieces, curved walls, and parameters no module can satisfy.
pub fn cabinet_run(doc: &mut Document, p: &CabinetRunParams) -> Result<Value, String> {
    let (request, said) = resolve(doc.home(), p)?;
    let mut planned = plan(doc.home(), &request)?;
    if let Some(said) = said {
        planned.notes.insert(0, said);
    }
    if p.dry {
        let rows: Vec<Value> = planned
            .modules
            .iter()
            .map(|m| json!(["", m.role, num(m.from), num(m.width)]))
            .collect();
        return Ok(summary(&planned, Value::Array(rows)));
    }
    // Work on a copy first: neighbors are planned against the new run.
    let mut scratch = Document::new(doc.home().clone());
    let (mut all, rows) = commands(&mut scratch, &planned)?;
    scratch
        .execute(Command::Batch {
            commands: all.clone(),
        })
        .map_err(|e| e.to_string())?;
    let mut reply = summary(&planned, rows);
    let mut adjusted = Vec::new();
    for tag in &planned.neighbors {
        let Some(stored) = scratch
            .home()
            .furniture
            .iter()
            .find(|f| f.properties.get(RUN_KEY) == Some(tag))
            .and_then(|f| f.properties.get(REQUEST_KEY))
            .and_then(|s| serde_json::from_str::<Request>(s).ok())
        else {
            continue;
        };
        let again = plan(scratch.home(), &stored)
            .map_err(|e| format!("{tag}, planejado de novo por encontrar este: {e}"))?;
        if unchanged(scratch.home(), tag, &again) {
            continue;
        }
        let (list, rows) = commands(&mut scratch, &again)?;
        scratch
            .execute(Command::Batch {
                commands: list.clone(),
            })
            .map_err(|e| e.to_string())?;
        all.extend(list);
        adjusted.push(summary(&again, rows));
    }
    // Same ids on the real document, then one undoable step.
    let last = scratch.new_furniture_id();
    while doc.new_furniture_id().0 + 1 < last.0 {}
    doc.execute(Command::Batch { commands: all })
        .map_err(|e| e.to_string())?;
    if !adjusted.is_empty() {
        reply["adjusted"] = Value::Array(adjusted);
    }
    Ok(reply)
}
