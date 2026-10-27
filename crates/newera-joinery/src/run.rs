//! A run of cabinets along a wall: the free stretches between corners,
//! doors, windows and appliances are split into modules of good proportion,
//! so no stretch is left too small to use. Clearances the workshop expects
//! (fillers at corners, air beside the fridge, trim beside frames) are kept.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    Build, CabinetParams, CountertopParams, Cutout, CutoutKind, DoorType, HandleColor, HandleStyle,
    Output, Part, cm, num,
};

/// Which row of a kitchen or wardrobe wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunRow {
    /// Floor cabinets under a countertop.
    #[default]
    Base,
    /// Wall-hung cabinets.
    Wall,
    /// Floor-to-top towers (pantry, oven tower, wardrobe).
    Tall,
}

/// What bounds a free stretch at one end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndKind {
    /// Nothing: the wall just ends.
    Free,
    /// A wall corner: fronts need a filler to open past it.
    Wall,
    /// A door or window frame: trim around it.
    Frame,
    /// A fridge: air on its sides.
    Fridge,
    /// A stove or cooktop: heat.
    Heat,
    /// Any other appliance or piece.
    Piece,
    /// Joinery that stays as it is.
    Kept,
}

impl EndKind {
    /// Space left empty, cm.
    fn clearance(self) -> f64 {
        match self {
            Self::Fridge => FRIDGE_AIR,
            Self::Frame => FRAME_TRIM,
            Self::Heat => HEAT_GAP,
            Self::Piece => 1.0,
            Self::Free | Self::Wall | Self::Kept => 0.0,
        }
    }

    /// Closed filler panel, cm.
    fn filler(self) -> f64 {
        if self == Self::Wall {
            CORNER_FILLER
        } else {
            0.0
        }
    }
}

/// A free stretch, cm along the wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunGap {
    pub from: f64,
    pub to: f64,
    pub start: EndKind,
    pub end: EndKind,
    /// Blind corner at the start: cm covered by another wall's run in front.
    pub blind_start: f64,
    /// Blind corner at the end, cm.
    pub blind_end: f64,
}

/// A stretch where the row passes over something (a fridge) and a shorter
/// cabinet fits above it: `from`, `to` along the wall, `bottom` its top, cm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunOver {
    pub from: f64,
    pub to: f64,
    pub bottom: f64,
}

/// Inside tall cabinets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Interior {
    /// Shelves (pantry, linen).
    #[default]
    Shelves,
    /// Hanging rails under a top shelf.
    Hanging,
    /// A wardrobe: hanging modules alternating with shelves and drawers.
    Wardrobe,
}

/// Choices for a run. Sizes in cm; unset ones follow the row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct RunParams {
    pub row: RunRow,
    /// Cabinet height (base 87 under a 3 cm top, wall 70, tall 220).
    pub h: Option<f64>,
    /// Depth including doors (base 55, wall 35, tall 58).
    pub d: Option<f64>,
    /// Bottom above the floor (wall 150, others 0).
    pub elev: Option<f64>,
    /// Board thickness, mm (default 18).
    pub t: f64,
    /// Front finish, e.g. `wood` or `#5f6e4a`.
    pub front: Option<String>,
    /// Carcass color `[r,g,b]` or `#rrggbb`.
    #[serde(deserialize_with = "crate::cabinet::rgb_or_hex")]
    #[schemars(with = "Option<HandleColor>")]
    pub color: Option<[u8; 3]>,
    /// Handles: bar, profile, knob, cava or none (default bar).
    pub handle: HandleStyle,
    /// Handle color `[r,g,b]` or `#rrggbb` (default brushed steel).
    pub handle_color: Option<HandleColor>,
    /// Modules made drawer units (default: 1 on a base row, next to the stove).
    pub drawers: Option<u32>,
    /// Widest module, cm (default 90).
    pub max: f64,
    /// Preferred module width, cm (default 60).
    pub target: f64,
    /// Base row: one countertop over each stretch (default true).
    pub top: bool,
    /// Countertop finish (default `stone`).
    pub top_material: Option<String>,
    /// Sink center, cm along the wall (where the plumbing is): a sink
    /// cabinet under it and the cutout in the countertop.
    pub sink: Option<f64>,
    /// Sink cabinet width, cm (default 80).
    pub sink_w: f64,
    /// Cooktop center, cm along the wall: a drawer unit under it, the cutout,
    /// and a hood gap in the wall row above.
    pub cooktop: Option<f64>,
    /// Cooktop cabinet width, cm (default 60; 75 or 90 for 5 burners).
    pub cooktop_w: f64,
    /// Tall row interior (default shelves; the MCP picks wardrobe in bedrooms).
    pub interior: Option<Interior>,
    /// A real sink bowl `[w, d]` stays in the cutout: sized from it, not drawn.
    pub sink_real: Option<[f64; 2]>,
    /// A real cooktop `[w, d]` stays in the cutout.
    pub cooktop_real: Option<[f64; 2]>,
}

impl Default for RunParams {
    fn default() -> Self {
        Self {
            row: RunRow::Base,
            h: None,
            d: None,
            elev: None,
            t: 18.0,
            front: None,
            color: None,
            handle: HandleStyle::Bar,
            handle_color: None,
            drawers: None,
            max: 90.0,
            target: 60.0,
            top: true,
            top_material: None,
            sink: None,
            sink_w: 80.0,
            cooktop: None,
            cooktop_w: 60.0,
            interior: None,
            sink_real: None,
            cooktop_real: None,
        }
    }
}

impl RunParams {
    /// Height, depth and elevation for the row.
    pub fn sizes(&self) -> (f64, f64, f64) {
        let (h, d, elev) = match self.row {
            RunRow::Base => (87.0, 55.0, 0.0),
            RunRow::Wall => (70.0, 35.0, 150.0),
            RunRow::Tall => (220.0, 58.0, 0.0),
        };
        (
            self.h.unwrap_or(h),
            self.d.unwrap_or(d),
            self.elev.unwrap_or(elev),
        )
    }
}

/// Air beside a fridge, cm.
const FRIDGE_AIR: f64 = 5.0;
/// Frame trim (alizar) beside doors and windows, cm.
const FRAME_TRIM: f64 = 5.0;
/// Gap beside a freestanding stove, cm.
const HEAT_GAP: f64 = 2.0;
/// Filler at a wall corner so doors and handles clear the other wall, cm.
const CORNER_FILLER: f64 = 3.0;
/// Narrowest useful module (spice pull-out), cm.
const SLIM: f64 = 15.0;
/// Narrowest regular module, cm.
const NARROW: f64 = 30.0;
/// Countertop overhang past the fronts, cm.
const TOP_OVERHANG: f64 = 3.0;
/// Countertop thickness, cm.
const TOP_T: f64 = 3.0;

/// What a module is for, as reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Doors,
    Drawers,
    /// 15–30 cm: spice or tray pull-out.
    Slim,
    /// Blind corner: a fixed panel where the other wall's run meets it.
    Corner,
    /// Tall module with a hanging rail.
    Hanging,
    /// Above a fridge.
    Over,
    /// Under the sink.
    Sink,
    /// Under the cooktop (drawers).
    Cooktop,
    Filler,
    Countertop,
}

/// One build of the run, placed along the wall.
#[derive(Debug, Clone, PartialEq)]
pub struct RunModule {
    /// Left end along the wall, cm.
    pub from: f64,
    pub width: f64,
    pub depth: f64,
    pub elevation: f64,
    pub role: Role,
    pub build: Build,
}

/// A filler panel (tamponamento) closing a stretch too small for a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct FillerParams {
    /// Width, cm.
    pub w: f64,
    /// Height, cm.
    pub h: f64,
    /// Depth to the front plane, cm.
    pub d: f64,
    /// Board, mm (default 18).
    pub t: f64,
    #[serde(deserialize_with = "crate::cabinet::rgb_or_hex")]
    #[schemars(with = "Option<HandleColor>")]
    pub color: Option<[u8; 3]>,
    pub front: Option<String>,
}

impl Default for FillerParams {
    fn default() -> Self {
        Self {
            w: 3.0,
            h: 87.0,
            d: 55.0,
            t: 18.0,
            color: None,
            front: None,
        }
    }
}

pub(crate) fn filler(p: &FillerParams) -> Result<Output, String> {
    if p.w <= 0.0 || p.h <= 0.0 || p.d < cm(p.t) {
        return Err("O tamponamento precisa de largura e altura positivas e profundidade maior que a chapa.".into());
    }
    let t = cm(p.t);
    let mut part = Part::board(
        "Tamponamento",
        [0.0, p.d - t, 0.0],
        [p.w, t, p.h],
        &format!("MDF {}", num(p.t)),
        p.color.unwrap_or(crate::MDF_WHITE),
    )
    .banded(2, 0);
    part.finish = p
        .front
        .as_deref()
        .map(str::parse::<newera_core::Material>)
        .transpose()?;
    Ok(Output {
        parts: vec![part],
        size: [p.w, p.d, p.h],
        hardware: Vec::new(),
        notes: Vec::new(),
        extra_cuts: Vec::new(),
        name: format!("Tamponamento {} cm", num(p.w)),
    })
}

/// What a stretch of a row becomes.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Piece {
    Doors,
    Slim,
    Filler,
    Sink,
    Cooktop,
}

/// A countertop stretch `(from, to)` and its cutouts `(kind, center)`.
type Top = (f64, f64, Vec<(CutoutKind, f64)>);

/// A cabinet that must stand at a given place: `(from, to, piece, widest)`.
type Fixed = (f64, f64, Piece, f64);

/// Splits `a..b` into modules: fixed ones where they must be, the rest in
/// widths near `target` (equal where nothing else matters), aligned with
/// `joints` of the other row when that costs little, blind panels of `b0`/`b1`
/// cm at the ends. Dynamic programming over candidate cut points.
#[allow(clippy::too_many_lines)]
fn segment(
    (a, b): (f64, f64),
    (b0, b1): (f64, f64),
    fixed: &[Fixed],
    joints: &[f64],
    p: &RunParams,
) -> Option<Vec<(f64, f64, Piece)>> {
    const EPS: f64 = 0.05;
    // Blind panels' edges too: doors beside a corner split like the rest.
    let mut anchors = vec![a, b, a + b0, b - b1];
    for &(s, e, _, _) in fixed {
        anchors.push(s);
        anchors.push(e);
    }
    let joints: Vec<f64> = joints
        .iter()
        .copied()
        .filter(|j| *j > a + SLIM && *j < b - SLIM)
        .collect();
    anchors.extend(&joints);
    anchors.sort_by(f64::total_cmp);
    anchors.dedup_by(|x, y| (*x - *y).abs() < EPS);
    let mut cuts = anchors.clone();
    for (i, &u) in anchors.iter().enumerate() {
        for &v in &anchors[i + 1..] {
            for n in 2..=8 {
                for k in 1..n {
                    let offset = ((v - u) * f64::from(k) / f64::from(n) * 10.0).round() / 10.0;
                    cuts.push(u + offset);
                }
            }
        }
    }
    cuts.retain(|c| *c >= a - EPS && *c <= b + EPS);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|x, y| (*x - *y).abs() < EPS);
    let is_joint = |x: f64| joints.iter().any(|j| (j - x).abs() < EPS);
    let near = |x: f64, y: f64| (x - y).abs() < EPS;
    let cost = |u: f64, v: f64| -> Option<(f64, Piece)> {
        let w = v - u;
        // A fixed cabinet: exactly over its place, maybe widened a little.
        let crossing: Vec<&Fixed> = fixed
            .iter()
            .filter(|(s, e, _, _)| u < e - EPS && v > s + EPS)
            .collect();
        let bonus = f64::from(u8::from(is_joint(u)) + u8::from(is_joint(v))) * 4.0;
        if let [(s, e, piece, widest)] = crossing[..] {
            let blinds = (near(u, a) && b0 > 0.0) || (near(v, b) && b1 > 0.0);
            if u <= s + EPS && v >= e - EPS && w <= widest + EPS && !blinds {
                return Some((3.0 + (w - (e - s)) * 0.8 - bonus, *piece));
            }
            return None;
        }
        if !crossing.is_empty() {
            return None;
        }
        let blind = if near(u, a) { b0 } else { 0.0 } + if near(v, b) { b1 } else { 0.0 };
        let door = w - blind;
        // A slim cabinet still needs a 15 cm bay between its two sides.
        let slim = SLIM.max(15.0 + 2.0 * p.t / 10.0);
        if (NARROW - EPS..=p.max + EPS).contains(&door) {
            Some(((door - p.target).powi(2) / 30.0 + 3.0 - bonus, Piece::Doors))
        } else if blind > 0.0 {
            // Too little beside the corner for a door (a sink or cooktop
            // takes the rest): the blind corner is closed by a panel, rather
            // than giving up on the whole run.
            (door > -EPS && door < NARROW).then(|| (40.0 + door.max(0.0), Piece::Filler))
        } else if (slim - EPS..NARROW).contains(&w) {
            Some((25.0 + (NARROW - w) - bonus, Piece::Slim))
        } else if w > EPS && w < slim {
            Some((60.0 + w, Piece::Filler))
        } else {
            None
        }
    };
    let n = cuts.len();
    let mut best: Vec<Option<(f64, usize, Piece)>> = vec![None; n];
    best[0] = Some((0.0, 0, Piece::Doors));
    for j in 1..n {
        for i in 0..j {
            let Some((so_far, _, _)) = best[i] else {
                continue;
            };
            if let Some((c, piece)) = cost(cuts[i], cuts[j]) {
                let total = so_far + c;
                if best[j].is_none_or(|(t, _, _)| total < t) {
                    best[j] = Some((total, i, piece));
                }
            }
        }
    }
    best[n - 1]?;
    let mut out = Vec::new();
    let mut j = n - 1;
    while j > 0 {
        let (_, i, piece) = best[j]?;
        out.push((cuts[i], cuts[j] - cuts[i], piece));
        j = i;
    }
    out.reverse();
    Some(out)
}

/// Plans the modules for the free stretches of one row.
///
/// # Errors
/// Parameters that make no module possible, as a sentence with the fix.
#[allow(clippy::too_many_lines)]
pub fn plan_run(
    gaps: &[RunGap],
    over: &[RunOver],
    joints: &[f64],
    p: &RunParams,
) -> Result<(Vec<RunModule>, Vec<String>), String> {
    let (h, d, elev) = p.sizes();
    if p.max <= 0.0 || p.target <= 0.0 {
        return Err("A modulação precisa de max e target positivos.".into());
    }
    let mut rules: Vec<String> = Vec::new();
    if !(NARROW..=120.0).contains(&p.max) || p.target < NARROW || p.target > p.max {
        rules.push(format!(
            "Modulação fora do usual (max entre {} e 120 cm, target entre {} e max): hoje max = {}, target = {}.",
            num(NARROW),
            num(NARROW),
            num(p.max),
            num(p.target)
        ));
    }
    let plinth = if p.row == RunRow::Wall { 0.0 } else { 10.0 };
    let shelves = match p.row {
        RunRow::Base | RunRow::Wall => 1,
        RunRow::Tall => 4,
    };
    let cabinet = |w: f64, door: DoorType, drawers: u32, h: f64, shelves: u32| CabinetParams {
        w,
        h,
        d,
        t: p.t,
        door,
        drawers,
        shelves,
        plinth,
        color: p.color,
        front: p.front.clone(),
        handle: p.handle,
        handle_color: p.handle_color.clone(),
        ..CabinetParams::default()
    };
    let mut modules: Vec<RunModule> = Vec::new();
    let mut notes = rules;
    let mut tops: Vec<Top> = Vec::new();
    let mut tall_count = 0u32;
    // Where a drawer unit helps most: next to the stove.
    let mut heat_ends: Vec<f64> = Vec::new();
    for g in gaps {
        if g.start == EndKind::Heat {
            heat_ends.push(g.from);
        }
        if g.end == EndKind::Heat {
            heat_ends.push(g.to);
        }
        let (c0, c1) = (g.start.clearance(), g.end.clearance());
        let (b0, b1) = (g.blind_start.max(0.0), g.blind_end.max(0.0));
        // A blind panel closes the corner by itself.
        let f0 = if b0 > 0.0 { 0.0 } else { g.start.filler() };
        let f1 = if b1 > 0.0 { 0.0 } else { g.end.filler() };
        let usable = g.to - g.from - c0 - c1 - f0 - f1;
        let mut x = g.from + c0;
        if c0 + c1 > 0.0 && g.to - g.from > c0 + c1 {
            for (kind, c) in [(g.start, c0), (g.end, c1)] {
                if c > 0.0 {
                    let why = match kind {
                        EndKind::Fridge => "ventilação da geladeira",
                        EndKind::Frame => "guarnição do vão",
                        EndKind::Heat => "folga do fogão",
                        _ => "folga",
                    };
                    let note = format!("{} cm livres para {why}.", num(c));
                    if !notes.contains(&note) {
                        notes.push(note);
                    }
                }
            }
        }
        if usable <= 0.5 {
            if g.to - g.from - c0 - c1 > 0.5 {
                // Only room for the fillers: close it whole.
                let w = g.to - g.from - c0 - c1;
                modules.push(filler_module(x, w, h, d, elev, p));
            }
            continue;
        }
        if usable < SLIM {
            // Too small for a module: one filler closes it, corner fillers included.
            let w = usable + f0 + f1;
            modules.push(filler_module(x, w, h, d, elev, p));
            notes.push(format!(
                "Vão de {} cm em {} cm fechado com tamponamento: um módulo útil precisa de {} cm.",
                num(w),
                num(x),
                num(SLIM)
            ));
            tops.push((g.from + c0, x + w, Vec::new()));
            continue;
        }
        if f0 > 0.0 {
            modules.push(filler_module(x, f0, h, d, elev, p));
            x += f0;
        }
        let (a, b) = (x, x + usable);
        // Sink and cooktop cabinets centered where the connections are.
        let mut fixed: Vec<Fixed> = Vec::new();
        if p.row == RunRow::Base {
            for (center, width, piece, widest, name) in [
                (p.sink, p.sink_w, Piece::Sink, p.sink_w + 20.0, "pia"),
                (
                    p.cooktop,
                    p.cooktop_w,
                    Piece::Cooktop,
                    p.cooktop_w + 15.0,
                    "cooktop",
                ),
            ] {
                let Some(center) = center.filter(|c| *c > g.from && *c < g.to) else {
                    continue;
                };
                let (lo, hi) = (a + b0, b - b1);
                if hi - lo < width {
                    notes.push(format!(
                        "O módulo de {} cm da {name} não cabe no vão de {} cm em {} cm.",
                        num(width),
                        num(hi - lo),
                        num(lo)
                    ));
                    continue;
                }
                let s = (center - width / 2.0).clamp(lo, hi - width);
                if (s + width / 2.0 - center).abs() > 0.5 {
                    notes.push(format!(
                        "A {name} ficou centrada em {} cm (pedida em {} cm) para caber no vão.",
                        num(s + width / 2.0),
                        num(center)
                    ));
                }
                if fixed
                    .iter()
                    .any(|(fs, fe, _, _)| s < *fe && s + width > *fs)
                {
                    notes.push(format!(
                        "A {name} cairia sobre outro módulo fixo; afaste os pontos."
                    ));
                    continue;
                }
                fixed.push((s, s + width, piece, widest));
            }
        }
        let Some(pieces) = segment((a, b), (b0, b1), &fixed, joints, p) else {
            modules.push(filler_module(a, usable, h, d, elev, p));
            notes.push(format!(
                "Vão de {} cm em {} cm sem divisão possível: fechado com tamponamento.",
                num(usable),
                num(a)
            ));
            x = b;
            if f1 > 0.0 {
                modules.push(filler_module(x, f1, h, d, elev, p));
                x += f1;
            }
            tops.push((g.from + c0, x, Vec::new()));
            continue;
        };
        let mut cutouts = Vec::new();
        let mut blind_note = false;
        for (from, w, piece) in pieces {
            let (role, build) = match piece {
                Piece::Filler => {
                    let m = filler_module(from, w, h, d, elev, p);
                    (m.role, m.build)
                }
                Piece::Slim => (
                    Role::Slim,
                    Build::Cabinet(cabinet(
                        w,
                        DoorType::Hinged,
                        0,
                        h,
                        if p.row == RunRow::Wall { 1 } else { 2 },
                    )),
                ),
                Piece::Sink => {
                    // The bowl over the drain, as long as it fits the cabinet;
                    // one narrower than the bowl (a `sink_w` under 60) has it
                    // in the middle.
                    let margin = 30.0_f64.min(w / 2.0);
                    let at = p
                        .sink
                        .unwrap_or(from + w / 2.0)
                        .clamp(from + margin, from + w - margin);
                    cutouts.push((CutoutKind::Sink, at));
                    (
                        Role::Sink,
                        Build::Cabinet(cabinet(w, DoorType::Hinged, 0, h, 0)),
                    )
                }
                Piece::Cooktop => {
                    let margin = 29.0_f64.min(w / 2.0);
                    let at = p
                        .cooktop
                        .unwrap_or(from + w / 2.0)
                        .clamp(from + margin, from + w - margin);
                    cutouts.push((CutoutKind::Cooktop, at));
                    heat_ends.push(from + w / 2.0);
                    let mut c = cabinet(w, DoorType::Drawers, 2, h, 0);
                    c.cooktop = true;
                    (Role::Cooktop, Build::Cabinet(c))
                }
                Piece::Doors => {
                    let left = if (from - a).abs() < 0.05 { b0 } else { 0.0 };
                    let right = if (from + w - b).abs() < 0.05 { b1 } else { 0.0 };
                    let mut c = cabinet(w, DoorType::Hinged, 0, h, shelves);
                    if p.row == RunRow::Tall {
                        let hang = match p.interior.unwrap_or_default() {
                            Interior::Shelves => false,
                            Interior::Hanging => true,
                            Interior::Wardrobe => tall_count.is_multiple_of(2),
                        };
                        tall_count += 1;
                        if hang {
                            c.rod = true;
                            c.shelves = 0;
                        } else if p.interior == Some(Interior::Wardrobe) {
                            c.shelves = 3;
                            c.drawers = 2;
                        }
                    }
                    c.blind_left = left;
                    c.blind_right = right;
                    if left + right > 0.0 {
                        blind_note = true;
                        (Role::Corner, Build::Cabinet(c))
                    } else if c.rod {
                        (Role::Hanging, Build::Cabinet(c))
                    } else {
                        (Role::Doors, Build::Cabinet(c))
                    }
                }
            };
            modules.push(RunModule {
                from,
                width: w,
                depth: d,
                elevation: elev,
                role,
                build,
            });
        }
        if blind_note {
            notes.push(
                "Canto em L: módulo de canto cego com painel fixo onde a outra bancada encosta."
                    .into(),
            );
        }
        x = b;
        if f1 > 0.0 {
            modules.push(filler_module(x, f1, h, d, elev, p));
            x += f1;
        }
        tops.push((g.from + c0, x, cutouts));
    }

    // Drawer units: on a base row, the modules closest to the stove.
    let has_cooktop = modules.iter().any(|m| m.role == Role::Cooktop);
    let wanted = p
        .drawers
        .unwrap_or(u32::from(p.row == RunRow::Base && !has_cooktop));
    if wanted > 0 {
        let mut eligible: Vec<usize> = modules
            .iter()
            .enumerate()
            .filter(|(_, m)| m.role == Role::Doors && (40.0..=90.0).contains(&m.width))
            .map(|(i, _)| i)
            .collect();
        let distance = |m: &RunModule| {
            heat_ends
                .iter()
                .map(|e| (m.from + m.width / 2.0 - e).abs())
                .fold(f64::MAX, f64::min)
        };
        eligible.sort_by(|a, b| distance(&modules[*a]).total_cmp(&distance(&modules[*b])));
        let drawer_rows = if h - plinth >= 70.0 { 3 } else { 2 };
        for &i in eligible.iter().take(wanted as usize) {
            let m = &mut modules[i];
            m.role = Role::Drawers;
            m.build = Build::Cabinet(cabinet(m.width, DoorType::Drawers, drawer_rows, h, 0));
        }
        if p.drawers.is_some() && eligible.len() < wanted as usize {
            notes.push(format!(
                "Só {} módulo(s) têm entre 40 e 90 cm para virar gaveteiro; pedidos {wanted}.",
                eligible.len()
            ));
        }
    }

    // Above a fridge, a shorter cabinet up to the row's top line.
    let top_line = elev + h;
    for o in over {
        let height = top_line - o.bottom - 2.0;
        if o.to - o.from < NARROW || height < 25.0 {
            continue;
        }
        let w = o.to - o.from;
        let mut params = cabinet(w, DoorType::Hinged, 0, height, 0);
        params.plinth = 0.0;
        params.doors = Some(if w > 60.0 { 2 } else { 1 });
        modules.push(RunModule {
            from: o.from,
            width: w,
            depth: d,
            elevation: top_line - height,
            role: Role::Over,
            build: Build::Cabinet(params),
        });
    }

    if p.row == RunRow::Base && p.top {
        for (from, to, cuts) in tops.into_iter().filter(|(a, b, _)| b - a >= 20.0) {
            modules.push(RunModule {
                from,
                width: to - from,
                depth: d + TOP_OVERHANG,
                elevation: 0.0,
                role: Role::Countertop,
                build: Build::Countertop(CountertopParams {
                    length: to - from,
                    depth: d + TOP_OVERHANG,
                    height: h + TOP_T,
                    thickness: TOP_T,
                    material: p.top_material.clone(),
                    cutouts: cuts
                        .iter()
                        .map(|(kind, at)| {
                            let (real, rim) = match kind {
                                CutoutKind::Sink => (p.sink_real, 2.5),
                                _ => (p.cooktop_real, 2.0),
                            };
                            Cutout {
                                kind: *kind,
                                x: at - from,
                                w: real.map(|r| r[0] - 2.0 * rim),
                                d: real.map(|r| r[1] - 2.0 * rim),
                                drawn: real.is_none(),
                            }
                        })
                        .collect(),
                    ..CountertopParams::default()
                }),
            });
        }
    }
    // Every build must stand on its own rules.
    for m in &modules {
        crate::generate(&m.build)
            .map_err(|e| format!("Módulo de {} cm em {} cm: {e}", num(m.width), num(m.from)))?;
    }
    modules.sort_by(|a, b| a.from.total_cmp(&b.from));
    Ok((modules, notes))
}

fn filler_module(from: f64, w: f64, h: f64, d: f64, elev: f64, p: &RunParams) -> RunModule {
    RunModule {
        from,
        width: w,
        depth: d,
        elevation: elev,
        role: Role::Filler,
        build: Build::Filler(FillerParams {
            w,
            h,
            d,
            t: p.t,
            color: p.color,
            front: p.front.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    fn gap(from: f64, to: f64, start: EndKind, end: EndKind) -> RunGap {
        RunGap {
            from,
            to,
            start,
            end,
            blind_start: 0.0,
            blind_end: 0.0,
        }
    }

    fn cabinets(modules: &[RunModule]) -> Vec<(Role, f64)> {
        modules
            .iter()
            .filter(|m| m.role != Role::Countertop)
            .map(|m| (m.role, (m.width * 10.0).round() / 10.0))
            .collect()
    }

    #[test]
    fn stretches_split_into_even_modules_without_leftovers() {
        // Corner at 0, fridge from 250: 3 cm filler, 5 cm air, 242 cm of modules.
        let (modules, notes) = plan_run(
            &[gap(0.0, 250.0, EndKind::Wall, EndKind::Fridge)],
            &[],
            &[],
            &RunParams::default(),
        )
        .unwrap();
        let sizes = cabinets(&modules);
        assert_eq!(sizes[0], (Role::Filler, 3.0));
        let widths: f64 = sizes.iter().map(|s| s.1).sum();
        assert!((widths - 245.0).abs() < 1e-6, "{sizes:?}");
        // 242 cm → four modules of 60,5 cm, one of them a drawer unit.
        assert_eq!(sizes.len(), 5, "{sizes:?}");
        assert!(sizes[1..].iter().all(|s| (s.1 - 60.5).abs() < 0.11));
        assert_eq!(sizes.iter().filter(|s| s.0 == Role::Drawers).count(), 1);
        assert!(notes.iter().any(|n| n.contains("ventilação")), "{notes:?}");
        // One countertop over modules and filler, stopping at the fridge air.
        let top = modules.iter().find(|m| m.role == Role::Countertop).unwrap();
        assert!((top.from - 0.0).abs() < 1e-9 && (top.width - 245.0).abs() < 1e-6);
        let last = modules
            .iter()
            .rfind(|m| m.role != Role::Countertop)
            .unwrap();
        assert!((last.from + last.width - 245.0).abs() < 1e-6);
    }

    #[test]
    fn a_sink_beside_a_blind_corner_keeps_the_run_divided() {
        // 250 cm ending at a corner covered 60 cm deep by the other run, the
        // sink centered 85 cm from that end: no door fits beside the corner.
        let mut corner = gap(0.0, 250.0, EndKind::Frame, EndKind::Wall);
        corner.blind_end = 60.0;
        let p = RunParams {
            sink: Some(165.0),
            ..RunParams::default()
        };
        let (modules, notes) = plan_run(&[corner], &[], &[], &p).unwrap();
        let sizes = cabinets(&modules);
        assert!(
            sizes.iter().any(|s| s.0 == Role::Sink),
            "{sizes:?} {notes:?}"
        );
        assert!(
            sizes
                .iter()
                .filter(|s| s.0 == Role::Doors || s.0 == Role::Drawers)
                .count()
                >= 2,
            "{sizes:?}"
        );
        assert!(
            !notes.iter().any(|n| n.contains("sem divisão")),
            "{notes:?}"
        );
    }

    #[test]
    fn slim_pull_outs_leave_a_real_bay_inside() {
        // 16.5 cm after the stove would give a 12.9 cm bay in 18 mm boards:
        // closed with a filler instead of a cabinet that can't be built.
        let (modules, _) = plan_run(
            &[
                gap(0.0, 120.0, EndKind::Free, EndKind::Heat),
                gap(194.5, 216.0, EndKind::Heat, EndKind::Free),
            ],
            &[],
            &[],
            &RunParams::default(),
        )
        .unwrap();
        let sizes = cabinets(&modules);
        assert!(
            !sizes.iter().any(|s| s.0 == Role::Slim && s.1 < 18.6),
            "{sizes:?}"
        );
    }

    #[test]
    fn the_drawer_unit_goes_beside_the_stove_and_small_stretches_are_used() {
        let (modules, notes) = plan_run(
            &[
                gap(0.0, 200.0, EndKind::Free, EndKind::Heat),
                gap(276.0, 300.0, EndKind::Heat, EndKind::Wall),
                gap(400.0, 410.0, EndKind::Frame, EndKind::Free),
            ],
            &[],
            &[],
            &RunParams::default(),
        )
        .unwrap();
        let sizes = cabinets(&modules);
        // Modules before the stove: the one touching it has the drawers.
        let before: Vec<&RunModule> = modules
            .iter()
            .filter(|m| m.role != Role::Countertop && m.from < 200.0)
            .collect();
        assert_eq!(before.last().unwrap().role, Role::Drawers, "{sizes:?}");
        // 24 cm after the stove − 2 cm heat gap − 3 cm filler = 19 cm pull-out.
        assert!(sizes.contains(&(Role::Slim, 19.0)), "{sizes:?}");
        // 10 cm − 5 cm trim → closed with a filler, and said so.
        assert!(sizes.contains(&(Role::Filler, 5.0)), "{sizes:?}");
        assert!(
            notes.iter().any(|n| n.contains("tamponamento")),
            "{notes:?}"
        );
    }

    #[test]
    fn corners_under_another_run_become_blind_modules() {
        // 7,5..300 with the other run's fronts covering 58 cm from the corner.
        let mut g = gap(7.5, 300.0, EndKind::Wall, EndKind::Wall);
        g.blind_start = 58.0;
        let (modules, notes) = plan_run(&[g], &[], &[], &RunParams::default()).unwrap();
        let sizes = cabinets(&modules);
        // No filler at the blind end, one at the far corner.
        assert_eq!(sizes.first().unwrap().0, Role::Corner, "{sizes:?}");
        assert_eq!(*sizes.last().unwrap(), (Role::Filler, 3.0));
        let Build::Cabinet(corner) = &modules
            .iter()
            .find(|m| m.role == Role::Corner)
            .unwrap()
            .build
        else {
            panic!("corner is a cabinet")
        };
        assert_eq!(corner.blind_left, 58.0);
        // Doors all the same width: the corner's door equals its neighbours'.
        let doors: Vec<f64> = sizes
            .iter()
            .filter(|s| s.0 == Role::Doors || s.0 == Role::Drawers)
            .map(|s| s.1)
            .collect();
        assert!(
            doors.iter().all(|w| (w - (corner.w - 58.0)).abs() < 0.2),
            "{sizes:?}"
        );
        assert!(notes.iter().any(|n| n.contains("canto cego")));
        let total: f64 = sizes.iter().map(|s| s.1).sum();
        assert!((total - 292.5).abs() < 1e-6);
    }

    #[test]
    fn a_sink_or_cooktop_cabinet_narrower_than_its_bowl_keeps_the_cutout_inside() {
        // A cabinet narrower than the 60 and 58 cm the bowl and the cooktop
        // are kept clear of its sides, between two 60 cm ones.
        for role in [Role::Sink, Role::Cooktop] {
            let w = if role == Role::Sink { 50.0 } else { 48.0 };
            let at = 60.0 + w / 2.0;
            let params = RunParams {
                sink: (role == Role::Sink).then_some(at),
                sink_w: 50.0,
                cooktop: (role == Role::Cooktop).then_some(at),
                cooktop_w: 48.0,
                ..RunParams::default()
            };
            let (modules, notes) = plan_run(
                &[gap(0.0, 120.0 + w, EndKind::Free, EndKind::Free)],
                &[],
                &[],
                &params,
            )
            .unwrap();
            let cabinet = modules.iter().find(|m| m.role == role).unwrap();
            assert!((cabinet.width - w).abs() < 1e-6, "{cabinet:?}");
            let top = modules.iter().find(|m| m.role == Role::Countertop).unwrap();
            let Build::Countertop(t) = &top.build else {
                panic!()
            };
            assert_eq!(t.cutouts.len(), 1, "{notes:?}");
            let x = top.from + t.cutouts[0].x;
            assert!(
                x >= cabinet.from && x <= cabinet.from + cabinet.width,
                "{x} {cabinet:?}"
            );
        }
    }

    #[test]
    fn sink_and_cooktop_cabinets_stand_under_their_points() {
        let params = RunParams {
            sink: Some(100.0),
            cooktop: Some(230.0),
            ..RunParams::default()
        };
        let (modules, notes) = plan_run(
            &[gap(0.0, 330.0, EndKind::Wall, EndKind::Wall)],
            &[],
            &[],
            &params,
        )
        .unwrap();
        let sizes = cabinets(&modules);
        let sink = modules.iter().find(|m| m.role == Role::Sink).unwrap();
        let cooktop = modules.iter().find(|m| m.role == Role::Cooktop).unwrap();
        // Centered on the plumbing and the gas point, or widened around them.
        assert!(
            (sink.from - 60.0).abs() < 10.5 && sink.from + sink.width >= 140.0,
            "{sizes:?}"
        );
        assert!(
            cooktop.from <= 200.0 && cooktop.from + cooktop.width >= 260.0,
            "{sizes:?}"
        );
        let Build::Cabinet(c) = &cooktop.build else {
            panic!()
        };
        assert!(c.cooktop && c.door == DoorType::Drawers);
        // No extra drawer unit: the cooktop's is enough.
        assert!(sizes.iter().all(|s| s.0 != Role::Drawers), "{sizes:?}");
        // The countertop has both holes where the cabinets are.
        let top = modules.iter().find(|m| m.role == Role::Countertop).unwrap();
        let Build::Countertop(t) = &top.build else {
            panic!()
        };
        assert_eq!(t.cutouts.len(), 2);
        assert!((t.cutouts[0].x - (100.0 - top.from)).abs() < 1e-6, "{t:?}");
        // Nothing left over and no module out of range.
        let total: f64 = sizes.iter().map(|s| s.1).sum();
        assert!((total - 330.0).abs() < 1e-6, "{sizes:?} {notes:?}");
        assert!(sizes.iter().all(|s| s.0 == Role::Filler || s.1 >= 15.0));
    }

    #[test]
    fn modules_line_up_with_the_other_row_when_it_costs_little() {
        // Alone, 240 cm would be 4 × 60; the base below has joints at 55, 120 and 185.
        let params = RunParams {
            row: RunRow::Wall,
            ..RunParams::default()
        };
        let (modules, _) = plan_run(
            &[gap(0.0, 240.0, EndKind::Free, EndKind::Free)],
            &[],
            &[55.0, 120.0, 185.0],
            &params,
        )
        .unwrap();
        let cuts: Vec<f64> = modules.iter().skip(1).map(|m| m.from).collect();
        assert_eq!(cuts, vec![55.0, 120.0, 185.0], "{:?}", cabinets(&modules));
        // A joint that would force an 80 cm and a 50 cm module is not worth it.
        let (modules, _) = plan_run(
            &[gap(0.0, 240.0, EndKind::Free, EndKind::Free)],
            &[],
            &[35.0],
            &params,
        )
        .unwrap();
        assert!(modules.iter().all(|m| (m.from - 35.0).abs() > 1.0));
    }

    #[test]
    fn wardrobes_alternate_hanging_and_shelves() {
        let params = RunParams {
            row: RunRow::Tall,
            interior: Some(Interior::Wardrobe),
            ..RunParams::default()
        };
        let (modules, _) = plan_run(
            &[gap(0.0, 240.0, EndKind::Wall, EndKind::Wall)],
            &[],
            &[],
            &params,
        )
        .unwrap();
        let roles: Vec<Role> = modules.iter().map(|m| m.role).collect();
        assert_eq!(
            roles.iter().filter(|r| **r == Role::Hanging).count(),
            2,
            "{roles:?}"
        );
        let shelves = modules
            .iter()
            .filter_map(|m| match &m.build {
                Build::Cabinet(c) if !c.rod && m.role == Role::Doors => Some(c.drawers),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            shelves.iter().all(|d| *d == 2) && !shelves.is_empty(),
            "{roles:?}"
        );
        assert!(
            roles
                .iter()
                .all(|r| *r != Role::Drawers && *r != Role::Countertop)
        );
    }

    #[test]
    fn wall_rows_add_a_cabinet_over_the_fridge() {
        let params = RunParams {
            row: RunRow::Wall,
            ..RunParams::default()
        };
        let (modules, _) = plan_run(
            &[gap(0.0, 90.0, EndKind::Free, EndKind::Free)],
            &[RunOver {
                from: 100.0,
                to: 170.0,
                bottom: 180.0,
            }],
            &[],
            &params,
        )
        .unwrap();
        let over = modules.iter().find(|m| m.role == Role::Over).unwrap();
        // Top line 220, fridge top 180, 2 cm air: 38 cm tall from 182.
        assert!((over.elevation - 182.0).abs() < 1e-9 && over.width == 70.0);
        assert!(modules.iter().all(|m| m.role != Role::Countertop));
        assert!(modules.iter().all(|m| m.role != Role::Drawers));
        // A modulation nobody builds is planned anyway, with the note: the
        // rule is the workshop's opinion, not a gate on the drawing.
        let (_, notes) = plan_run(
            &[gap(0.0, 200.0, EndKind::Wall, EndKind::Wall)],
            &[],
            &[],
            &RunParams {
                max: 20.0,
                target: 20.0,
                ..RunParams::default()
            },
        )
        .unwrap();
        assert!(
            notes.iter().any(|n| n.contains("fora do usual")),
            "{notes:?}"
        );
    }
}
