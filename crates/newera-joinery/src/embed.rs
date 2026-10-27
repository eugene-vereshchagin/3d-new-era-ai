//! Embedding: a sink bowl or a cooktop set into a countertop, an oven or a
//! microwave into a cabinet niche. The host is rebuilt by its rules with the
//! cutout or niche sized from the real item, and the item becomes part of
//! the host: moving or changing the host carries it along.

use newera_core::{Command, Document, Furniture, FurnitureId};
use serde_json::{Value, json};

use crate::{
    Build, CabinetParams, Cutout, CutoutKind, KIND_KEY, Niche, PARAMS_KEY, assemble, generate,
    merged,
};

/// A length for messages, the Brazilian way.
fn num(v: f64) -> String {
    crate::num(v)
}

/// Overall depth of a build, cm.
fn output_depth(build: &Build) -> Result<f64, String> {
    generate(build).map(|o| o.size[1])
}

/// A length for replies, as a JSON number rounded to a millimeter.
fn val(v: f64) -> Value {
    let r = (v * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        json!(r as i64)
    } else {
        json!(r)
    }
}

/// Property on an embedded item: what it is to its host (`sink`, `oven`…).
pub const EMBED_KEY: &str = "joinery:embedded";

/// What an item is, for embedding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixture {
    Sink,
    Cooktop,
    Oven,
    Microwave,
    Dishwasher,
    Fridge,
    /// A television on a panel.
    Tv,
    /// Anything else built into a cabinet (wine cooler, safe, speaker).
    Appliance,
}

impl Fixture {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sink => "sink",
            Self::Cooktop => "cooktop",
            Self::Oven => "oven",
            Self::Microwave => "microwave",
            Self::Dishwasher => "dishwasher",
            Self::Fridge => "fridge",
            Self::Tv => "tv",
            Self::Appliance => "appliance",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Sink => "a cuba",
            Self::Cooktop => "o cooktop",
            Self::Oven => "o forno",
            Self::Microwave => "o micro-ondas",
            Self::Dishwasher => "a lava-louças",
            Self::Fridge => "a geladeira",
            Self::Tv => "a TV",
            Self::Appliance => "o aparelho",
        }
    }
}

fn plain(text: &str) -> String {
    text.to_lowercase()
        .replace(['á', 'à', 'â', 'ã'], "a")
        .replace(['é', 'ê'], "e")
        .replace('í', "i")
        .replace(['ó', 'ô', 'õ'], "o")
        .replace('ú', "u")
        .replace('ç', "c")
}

/// What a piece is, from its catalog id or its name.
pub fn fixture_of(piece: &Furniture) -> Fixture {
    match piece.catalog.as_str() {
        "sink-bowl" => return Fixture::Sink,
        "cooktop" => return Fixture::Cooktop,
        "oven" => return Fixture::Oven,
        "microwave" => return Fixture::Microwave,
        "dishwasher" => return Fixture::Dishwasher,
        "fridge" => return Fixture::Fridge,
        "tv" => return Fixture::Tv,
        _ => {}
    }
    let n = plain(&piece.name);
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&["cooktop", "cook top"]) {
        Fixture::Cooktop
    } else if has(&["cuba", "pia"]) {
        Fixture::Sink
    } else if has(&["micro-ondas", "microondas", "micro ondas"]) {
        Fixture::Microwave
    } else if has(&["forno"]) {
        Fixture::Oven
    } else if has(&["lava-louca", "lava louca", "lava-loucas"]) {
        Fixture::Dishwasher
    } else if has(&["geladeira", "refrigerador", "frigobar", "adega"]) {
        Fixture::Fridge
    } else if has(&["televis", "smart tv"]) || n.starts_with("tv") {
        Fixture::Tv
    } else {
        Fixture::Appliance
    }
}

/// Moves the embedded items of `old` onto `new`, keeping their place relative
/// to the host (after the host moved, turned or was rebuilt). On a countertop
/// a sink or cooktop goes with its cutout: the one at the same spot, or else
/// the nth of its kind, as it was the nth before; with no cutout for it, it
/// stays where it stood. Returns a note for each one left so.
pub fn carry_embedded(old: &Furniture, new: &mut Furniture) -> Vec<String> {
    let cutouts = |f: &Furniture| match serde_json::from_str::<Build>(f.properties.get(PARAMS_KEY)?)
    {
        Ok(Build::Countertop(top)) => Some((top.length, top.cutouts)),
        _ => None,
    };
    let tops = cutouts(old).zip(cutouts(new));
    let mut notes = Vec::new();
    for child in old
        .children
        .iter()
        .filter(|c| c.properties.contains_key(EMBED_KEY))
    {
        if new.children.iter().any(|c| c.id == child.id) {
            continue;
        }
        let local = old.to_local(child.position);
        let mut moved = child.clone();
        moved.position = new.to_plan(local);
        moved.angle = child.angle + new.angle - old.angle;
        moved.elevation = child.elevation + new.elevation - old.elevation;
        let kind = match fixture_of(child) {
            Fixture::Sink => Some(CutoutKind::Sink),
            Fixture::Cooktop => Some(CutoutKind::Cooktop),
            _ => None,
        };
        if let (Some(((old_length, old_cuts), (new_length, new_cuts))), Some(kind)) = (&tops, kind)
        {
            let xs = |cuts: &[Cutout]| {
                let mut xs: Vec<f64> = cuts
                    .iter()
                    .filter(|c| c.kind == kind)
                    .map(|c| c.x)
                    .collect();
                xs.sort_by(f64::total_cmp);
                xs
            };
            let spot = |x: f64| new.to_plan((x - new_length / 2.0, local.1));
            let after = xs(new_cuts);
            let nth = xs(old_cuts)
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    let off = |x: f64| (x - old_length / 2.0 - local.0).abs();
                    off(*a.1).total_cmp(&off(*b.1))
                })
                .map(|(i, _)| i);
            let bound = after
                .iter()
                .copied()
                .find(|x| spot(*x).distance(child.position) < 1.0)
                .or_else(|| nth.and_then(|i| after.get(i).copied()));
            // On the stone: it follows the top surface.
            moved.elevation =
                child.elevation + (new.elevation + new.height) - (old.elevation + old.height);
            if let Some(x) = bound {
                moved.position = spot(x);
            } else {
                moved.position = child.position;
                notes.push(format!(
                    "{} {} ficou onde estava: a bancada não tem mais recorte para {}.",
                    child.name,
                    child.id,
                    fixture_of(child).label()
                ));
            }
        }
        new.children.push(moved);
    }
    notes
}

/// A request to embed `item` (a piece already in the plan, or a new one) into
/// the joinery group `host`.
#[derive(Debug, Clone)]
pub struct EmbedRequest {
    pub item: Furniture,
    /// The item is already in the plan (it leaves its place for the host).
    pub existing: bool,
    pub host: FurnitureId,
    /// Center along the host's width from its left end, cm.
    pub at: Option<f64>,
    /// Niche floor above the room floor, cm (cabinets).
    pub z: Option<f64>,
    pub dry: bool,
}

/// Embeds the item and rebuilds the host in one undoable step. Replies
/// `{host, item, kind, cutout|niche, notes}`.
///
/// # Errors
/// Hosts that are not joinery, items that don't go in that host, and items
/// that don't fit — each with the value to change.
#[allow(clippy::too_many_lines)]
pub fn embed(doc: &mut Document, request: &EmbedRequest) -> Result<Value, String> {
    let host = doc
        .home()
        .furniture
        .iter()
        .find(|f| f.id == request.host)
        .cloned()
        .ok_or_else(|| format!("{} not found", request.host))?;
    let stored = host.properties.get(PARAMS_KEY).ok_or_else(|| {
        format!(
            "{} não é marcenaria paramétrica: embuta em uma bancada ou armário feito com joinery ou cabinet_run.",
            host.id
        )
    })?;
    let build = merged(stored, &json!({}))?;
    let mut item = request.item.clone();
    let kind = fixture_of(&item);
    let local_of_item = host.to_local(item.position);
    let mut notes = Vec::new();
    // What goes through the countertop, `(w, d, top)`: its hole, and the
    // height of the stone it goes through.
    let mut through: Option<(f64, f64, f64)> = None;
    let (new_build, place, detail): (Build, (f64, f64, f64), Value) = match build {
        Build::Countertop(mut top) => {
            let cut = match kind {
                Fixture::Sink => CutoutKind::Sink,
                Fixture::Cooktop => CutoutKind::Cooktop,
                _ => {
                    return Err(format!(
                        "Numa bancada embutem cuba e cooktop; {} vai num armário com nicho (host = id do armário).",
                        kind.label()
                    ));
                }
            };
            // A drop-in bowl rests on a 2,5 cm rim; a cooktop's glass on 2 cm.
            let rim = if cut == CutoutKind::Sink { 2.5 } else { 2.0 };
            let (w, d) = (item.width - 2.0 * rim, item.depth - 2.0 * rim);
            if w <= 0.0 || d <= 0.0 {
                return Err(format!(
                    "{} de {} × {} cm é pequeno demais para um recorte.",
                    kind.label(),
                    num(item.width),
                    num(item.depth)
                ));
            }
            let from_item = (request.existing && local_of_item.0.abs() <= top.length / 2.0)
                .then_some(local_of_item.0 + top.length / 2.0);
            let at = request.at.or(from_item).unwrap_or(top.length / 2.0);
            // The same kind of hole already there, around that spot, is replaced.
            top.cutouts.retain(|c| {
                !(c.kind == cut && (c.x - at).abs() < c.w.unwrap_or(w).max(w) / 2.0 + 1.0)
            });
            top.cutouts.push(Cutout {
                kind: cut,
                x: at,
                w: Some(w),
                d: Some(d),
                drawn: false,
            });
            let lift = if cut == CutoutKind::Sink { 0.5 } else { 0.6 };
            let place = (at - top.length / 2.0, 0.0, top.height - item.height + lift);
            through = Some((w, d, host.elevation + top.height));
            let detail = json!({"cutout": [val(w), val(d)], "x": val(at)});
            (Build::Countertop(top), place, detail)
        }
        Build::Cabinet(mut cabinet) => {
            if matches!(kind, Fixture::Sink | Fixture::Cooktop) {
                return Err(format!(
                    "{} embute na bancada, não no armário: host = id da bancada sobre ele.",
                    kind.label()
                ));
            }
            let t = cabinet.t / 10.0;
            let inner_w = cabinet.w - 2.0 * t;
            // Built-in ovens and microwaves have a front frame 2 cm wider on each
            // side than their body, resting on the cabinet sides.
            let body = match kind {
                Fixture::Oven | Fixture::Microwave => item.width - 4.0,
                _ => item.width,
            };
            if body + 1.0 > inner_w || item.width > cabinet.w + 0.05 {
                notes.push(format!(
                    "{} tem {} cm de largura ({} cm de corpo) e o vão interno do armário {} cm: não entra sem alargar o armário para w = {}.",
                    kind.label(),
                    num(item.width),
                    num(body),
                    num(inner_w),
                    num((body + 1.0 + 2.0 * t).ceil().max(item.width))
                ));
            }
            let front_t = t;
            let inner_d = cabinet.d - front_t - 1.0 - cabinet.back / 10.0;
            // Without a door over it, the front may come out to the doors' plane.
            if item.depth > inner_d + front_t + 0.05 {
                notes.push(format!(
                    "{} tem {} cm de profundidade e o nicho {} cm: sobra para fora sem d = {} no armário.",
                    kind.label(),
                    num(item.depth),
                    num(inner_d + front_t),
                    num((item.depth + 1.0 + cabinet.back / 10.0).ceil())
                ));
            }
            let floor = cabinet.plinth + t;
            let tall = cabinet.h >= 150.0;
            let default_bottom = match kind {
                Fixture::Oven if tall => 80.0,
                Fixture::Microwave if tall => 145.0,
                _ => floor,
            };
            let from_item = request
                .existing
                .then_some(item.elevation - host.elevation)
                .filter(|z| *z >= floor && *z + item.height <= cabinet.h - t);
            // `z` is above the room floor; the niche is measured in the cabinet.
            let asked = request
                .z
                .map(|z| z - host.elevation)
                .or(from_item)
                .unwrap_or(default_bottom);
            // Where the cabinet has room for it: the item sits on the niche
            // that is built, not on the one that was asked.
            let lowest = floor
                + if cabinet.door == crate::DoorType::Drawers {
                    0.0
                } else {
                    f64::from(cabinet.drawers) * 18.0
                };
            let highest = cabinet.h - t - (item.height + 2.0);
            let bottom = if highest >= lowest {
                asked.clamp(lowest, highest)
            } else {
                asked
            };
            if (bottom - asked).abs() > 0.05 {
                notes.push(format!(
                    "Com z = {} {} não cabe no armário: fica a {} cm do chão, onde há lugar para o nicho.",
                    num(asked + host.elevation),
                    kind.label(),
                    num(bottom + host.elevation)
                ));
            }
            // A couple of centimeters of air above the appliance.
            // Niches this one would overlap make way, unless something is in them.
            let top_of = bottom + item.height + 2.0;
            let overlaps =
                |n: &Niche| !(n.bottom + n.height + t <= bottom || n.bottom >= top_of + t);
            for n in cabinet.niches.iter().filter(|n| overlaps(n)) {
                let occupied = host.children.iter().find(|c| {
                    c.id != item.id
                        && c.properties.contains_key(EMBED_KEY)
                        && (c.elevation - host.elevation - n.bottom).abs() < 1.0
                });
                if let Some(other) = occupied {
                    notes.push(format!(
                        "O nicho de {} a {} cm já tem {} {}: os dois ficam no mesmo lugar (z = {} poria um acima do outro).",
                        num(n.bottom),
                        num(n.bottom + n.height),
                        other.name,
                        other.id,
                        num(n.bottom + n.height + t)
                    ));
                }
            }
            cabinet.niches.retain(|n| !overlaps(n));
            cabinet.niches.push(Niche {
                bottom,
                height: item.height + 2.0,
            });
            if kind == Fixture::Oven {
                notes.push(
                    "Forno: confira no manual a ventilação pedida (fresta no rodapé ou no topo)."
                        .to_owned(),
                );
            }
            let at = request.at.unwrap_or(cabinet.w / 2.0);
            let dc = cabinet.d - front_t;
            let place = (
                at - cabinet.w / 2.0,
                dc - item.depth / 2.0 - cabinet.d / 2.0 + front_t,
                bottom,
            );
            let detail = json!({
                "niche": [val(inner_w), val(item.height + 2.0)],
                "bottom": val(host.elevation + bottom),
            });
            (Build::Cabinet(CabinetParams { ..cabinet }), place, detail)
        }
        Build::Slats(panel) => {
            if kind != Fixture::Tv {
                return Err(format!(
                    "No painel ripado vai a TV; {} vai numa bancada ou num armário.",
                    kind.label()
                ));
            }
            if item.width + 10.0 > panel.w {
                notes.push(format!(
                    "A TV tem {} cm de largura e o painel {} cm: sem folga nas laterais (w = {} daria 5 cm de cada lado).",
                    num(item.width),
                    num(panel.w),
                    num((item.width + 10.0).ceil())
                ));
            }
            // Seated eyes are about 105 cm from the floor: the screen's middle there.
            let center = request.z.unwrap_or(105.0);
            let bottom = center - item.height / 2.0;
            if bottom < 30.0 || center + item.height / 2.0 > panel.h - 5.0 {
                notes.push(format!(
                    "Com o centro a {} cm a TV de {} cm passa do painel de {} cm; z entre {} e {} a mantém dentro.",
                    num(center),
                    num(item.height),
                    num(panel.h),
                    num(30.0 + item.height / 2.0),
                    num(panel.h - 5.0 - item.height / 2.0)
                ));
            }
            let at = request.at.unwrap_or(panel.w / 2.0);
            if at - item.width / 2.0 < 0.0 || at + item.width / 2.0 > panel.w {
                notes.push(format!(
                    "A TV centrada em {} cm passa da borda do painel; at entre {} e {} a mantém dentro.",
                    num(at),
                    num(item.width / 2.0),
                    num(panel.w - item.width / 2.0)
                ));
            }
            let depth = output_depth(&Build::Slats(panel.clone()))?;
            notes.push(format!(
                "Passa-fios atrás da TV a {} cm do chão; tomada e ponto de antena a {} cm.",
                num(bottom + item.height * 0.3),
                num(bottom + item.height * 0.3)
            ));
            let place = (at - panel.w / 2.0, depth / 2.0 + item.depth / 2.0, bottom);
            let detail = json!({"x": val(at), "bottom": val(bottom)});
            (Build::Slats(panel), place, detail)
        }
        other => {
            return Err(format!(
                "{} é um(a) {}: embuta em bancada (countertop) ou armário (cabinet).",
                host.id,
                other.kind()
            ));
        }
    };
    let output = generate(&new_build).map_err(|e| format!("Para embutir {}: {e}", kind.label()))?;
    notes.extend(output.notes.iter().cloned());
    // The body goes on through the wooden top of the cabinet under the stone.
    let below = match through {
        Some((w, d, counter)) => {
            let body = Furniture {
                position: host.to_plan((place.0, place.1)),
                angle: host.angle,
                elevation: host.elevation + place.2,
                ..item.clone()
            };
            cut_below(doc.home(), &host, &body, (w, d), counter, &mut notes)
        }
        None => Vec::new(),
    };
    let reply = |item_id: &str| {
        let mut r = json!({
            "host": host.id.to_string(),
            "item": item_id,
            "kind": kind.name(),
            "notes": notes,
        });
        if !below.is_empty() {
            r["cut_below"] = json!(
                below
                    .iter()
                    .map(|(c, _)| c.id.to_string())
                    .collect::<Vec<_>>()
            );
        }
        if let (Some(obj), Some(extra)) = (r.as_object_mut(), detail.as_object()) {
            for (k, v) in extra {
                obj.insert(k.clone(), v.clone());
            }
        }
        r
    };
    if request.dry {
        return Ok(reply(""));
    }
    // Other items embedded before stay; this one takes its new place.
    let mut others = host.clone();
    others.children.retain(|c| c.id != item.id);
    let mut rebuilt = rebuild(doc, &others, &new_build, &output);
    item.position = rebuilt.to_plan((place.0, place.1));
    item.angle = host.angle;
    item.elevation = host.elevation + place.2;
    item.level = host.level;
    item.properties.insert(EMBED_KEY.into(), kind.name().into());
    let item_id = item.id;
    rebuilt.children.push(item);
    let mut commands = Vec::new();
    if request.existing && doc.home().furniture.iter().any(|f| f.id == item_id) {
        commands.push(Command::remove(item_id));
    }
    commands.push(Command::update(rebuilt));
    for (cabinet, params) in &below {
        let build = Build::Cabinet(params.clone());
        let output = generate(&build)?;
        commands.push(Command::update(rebuild(doc, cabinet, &build, &output)));
    }
    doc.execute(Command::Batch { commands })
        .map_err(|e| e.to_string())?;
    Ok(reply(&item_id.to_string()))
}

/// `group` built again from `build`, where it stands, keeping its name,
/// storey, properties and the items embedded in it.
fn rebuild(
    doc: &mut Document,
    group: &Furniture,
    build: &Build,
    output: &crate::Output,
) -> Furniture {
    let mut next = || doc.new_furniture_id();
    let mut rebuilt = assemble(
        build,
        output,
        group.id,
        group.position,
        group.angle,
        group.elevation,
        &mut next,
    );
    rebuilt.name.clone_from(&group.name);
    rebuilt.level = group.level;
    for (key, value) in &group.properties {
        if key != PARAMS_KEY && key != KIND_KEY {
            rebuilt.properties.insert(key.clone(), value.clone());
        }
    }
    carry_embedded(group, &mut rebuilt);
    rebuilt
}

/// The joinery cabinets under `countertop` whose wooden top `body` — a bowl
/// or a cooktop set into the stone at `counter` cm — goes through, each with
/// its top cut to the stone's hole `(w, d)`. Without it the render shows the
/// board inside the bowl, and nothing else notices.
fn cut_below(
    home: &newera_core::Home,
    countertop: &Furniture,
    body: &Furniture,
    hole: (f64, f64),
    counter: f64,
    notes: &mut Vec<String>,
) -> Vec<(Furniture, CabinetParams)> {
    let level = home.resolve_level(countertop.level);
    let mut out = Vec::new();
    for cabinet in home
        .furniture
        .iter()
        .filter(|f| f.id != countertop.id && home.resolve_level(f.level) == level)
    {
        let Some(Ok(Build::Cabinet(mut p))) = cabinet
            .properties
            .get(PARAMS_KEY)
            .map(|stored| merged(stored, &json!({})))
        else {
            continue;
        };
        match top_cut(cabinet, &p, body, hole, counter) {
            Some(Ok(cut)) => {
                p.top_cutout = Some(cut);
                out.push((cabinet.clone(), p));
            }
            Some(Err(why)) => notes.push(why),
            None => {}
        }
    }
    out
}

/// The `top_cutout` a cabinet standing as `cabinet` needs for `body` — set
/// into a stone at `counter` cm through a hole `(w, d)` — to pass through its
/// wooden top: `None` when the body does not reach it (or it is already cut
/// so), an error saying why when it cannot be cut.
pub(crate) fn top_cut(
    cabinet: &Furniture,
    p: &CabinetParams,
    body: &Furniture,
    (w, d): (f64, f64),
    counter: f64,
) -> Option<Result<[f64; 4], String>> {
    // Under the stone, and reaching up to where the body hangs.
    let top = cabinet.elevation + cabinet.height;
    if top > counter + 0.5 || top <= body.elevation + 0.1 {
        return None;
    }
    let (lx, ly) = cabinet.to_local(body.position);
    if lx.abs() > cabinet.width / 2.0 || ly.abs() > cabinet.depth / 2.0 {
        return None;
    }
    let turn = (body.angle - cabinet.angle).rem_euclid(180.0);
    let (cw, cd) = if !(0.5..=179.5).contains(&turn) {
        (w, d)
    } else if (turn - 90.0).abs() < 0.5 {
        (d, w)
    } else {
        return None;
    };
    let x = lx + cabinet.width / 2.0 - cw / 2.0;
    let y = ly + cabinet.depth / 2.0 - cd / 2.0;
    let cut = [x, y, cw, cd].map(|v| (v * 10.0).round() / 10.0);
    if let Some(old) = p.top_cutout {
        if old.iter().zip(cut).all(|(a, b)| (a - b).abs() < 0.05) {
            return None;
        }
        let apart = old[0] + old[2] <= cut[0]
            || cut[0] + cut[2] <= old[0]
            || old[1] + old[3] <= cut[1]
            || cut[1] + cut[3] <= old[1];
        if apart {
            return Some(Err(format!(
                "O tampo de {} já tem um recorte em outro lugar e só leva um: recorte-o para {} com joinery id={} p.top_cutout.",
                cabinet.id, body.name, cabinet.id
            )));
        }
    }
    let cut_one = CabinetParams {
        top_cutout: Some(cut),
        ..p.clone()
    };
    Some(match generate(&Build::Cabinet(cut_one)) {
        Ok(_) => Ok(cut),
        Err(why) => Err(format!(
            "{} atravessa o tampo de {} e o recorte não cabe nele: {why}",
            body.name, cabinet.id
        )),
    })
}

#[cfg(test)]
mod tests {
    use newera_core::Point2;

    use super::*;
    use crate::CountertopParams;

    fn host(doc: &mut Document, build: &Build) -> FurnitureId {
        let output = generate(build).unwrap();
        let id = doc.new_furniture_id();
        let mut next = || doc.new_furniture_id();
        let group = assemble(
            build,
            &output,
            id,
            Point2::new(200.0, 40.0),
            0.0,
            0.0,
            &mut next,
        );
        doc.execute(Command::insert(group)).unwrap();
        id
    }

    fn piece(doc: &mut Document, catalog: &str, name: &str, size: [f64; 3]) -> Furniture {
        Furniture {
            id: doc.new_furniture_id(),
            catalog: catalog.into(),
            name: name.into(),
            position: Point2::new(150.0, 40.0),
            width: size[0],
            depth: size[1],
            height: size[2],
            ..Furniture::default()
        }
    }

    #[test]
    fn a_cooktop_goes_into_a_countertop_and_moves_with_it() {
        let mut doc = Document::default();
        let top = host(
            &mut doc,
            &Build::Countertop(CountertopParams {
                length: 200.0,
                ..CountertopParams::default()
            }),
        );
        let cooktop = piece(&mut doc, "cooktop", "Cooktop", [60.0, 50.0, 6.0]);
        doc.execute(Command::insert(cooktop.clone())).unwrap();
        let reply = embed(
            &mut doc,
            &EmbedRequest {
                item: cooktop.clone(),
                existing: true,
                host: top,
                at: None,
                z: None,
                dry: false,
            },
        )
        .unwrap();
        // Placed where it was: 150 is 50 cm from the countertop's left end at 100.
        assert_eq!(reply["x"], 50);
        assert_eq!(reply["cutout"], json!([56, 46]));
        let home = doc.home();
        assert!(
            !home.furniture.iter().any(|f| f.id == cooktop.id),
            "no longer loose"
        );
        let group = home.furniture.iter().find(|f| f.id == top).unwrap();
        let inside = group.children.iter().find(|c| c.id == cooktop.id).unwrap();
        // Glass 0,6 cm above the 90 cm top.
        assert!((inside.elevation + inside.height - 90.6).abs() < 1e-9);
        assert!((inside.position.x - 150.0).abs() < 1e-9);
        let params = &group.properties[PARAMS_KEY];
        assert!(params.contains("\"drawn\":false"), "{params}");
        // No generic glass drawn over the real cooktop.
        assert!(!group.children.iter().any(|c| c.name == "Cooktop (vidro)"));
        // Moved and rebuilt, the host carries it.
        let mut moved = group.clone();
        moved.translate(30.0, 0.0);
        let mut rebuilt = group.clone();
        rebuilt
            .children
            .retain(|c| !c.properties.contains_key(EMBED_KEY));
        rebuilt.translate(30.0, 0.0);
        carry_embedded(group, &mut rebuilt);
        let carried = rebuilt
            .children
            .iter()
            .find(|c| c.id == cooktop.id)
            .unwrap();
        assert!((carried.position.x - 180.0).abs() < 1e-9);
        // An oven doesn't go in a countertop, and says where it does.
        let oven = piece(&mut doc, "oven", "Forno", [60.0, 55.0, 60.0]);
        let err = embed(
            &mut doc,
            &EmbedRequest {
                item: oven,
                existing: false,
                host: top,
                at: None,
                z: None,
                dry: true,
            },
        )
        .unwrap_err();
        assert!(err.contains("armário com nicho"), "{err}");
    }

    #[test]
    fn a_cooktop_keeps_its_cutout_when_the_countertop_is_resized() {
        let mut doc = Document::default();
        let top = host(
            &mut doc,
            &Build::Countertop(CountertopParams {
                length: 400.0,
                ..CountertopParams::default()
            }),
        );
        let cooktop = piece(&mut doc, "cooktop", "Cooktop", [60.0, 50.0, 6.0]);
        embed(
            &mut doc,
            &EmbedRequest {
                item: cooktop.clone(),
                existing: false,
                host: top,
                at: Some(138.0),
                z: None,
                dry: false,
            },
        )
        .unwrap();
        let group = doc
            .home()
            .furniture
            .iter()
            .find(|f| f.id == top)
            .unwrap()
            .clone();
        let before = group
            .children
            .iter()
            .find(|c| c.id == cooktop.id)
            .unwrap()
            .position;
        assert!((before.x - 138.0).abs() < 1e-9);
        // Cut to 324 cm from the left, the right end kept: the cooktop's
        // cutout is now 62 cm from the new left end, the same spot.
        let resized = |cutouts: Vec<Cutout>| {
            let build = Build::Countertop(CountertopParams {
                length: 324.0,
                cutouts,
                ..CountertopParams::default()
            });
            let output = generate(&build).unwrap();
            let mut n = 1000;
            let mut next = || {
                n += 1;
                FurnitureId(n)
            };
            let mut new = assemble(
                &build,
                &output,
                top,
                Point2::new(238.0, 40.0),
                0.0,
                0.0,
                &mut next,
            );
            let notes = carry_embedded(&group, &mut new);
            let at = new
                .children
                .iter()
                .find(|c| c.id == cooktop.id)
                .unwrap()
                .position;
            (at, notes)
        };
        let (after, notes) = resized(vec![Cutout {
            kind: CutoutKind::Cooktop,
            x: 62.0,
            w: None,
            d: None,
            drawn: true,
        }]);
        assert!(after.distance(before) < 1e-6, "{after:?} {before:?}");
        assert!(notes.is_empty(), "{notes:?}");
        // No cutout for it any more: it stays where it stood, and says so.
        let (after, notes) = resized(Vec::new());
        assert!(after.distance(before) < 1e-6, "{after:?} {before:?}");
        assert_eq!(notes.len(), 1, "{notes:?}");
    }

    #[test]
    fn a_bowl_set_into_the_stone_cuts_the_cabinet_top_under_it() {
        let mut doc = Document::default();
        // A vanity: a cabinet, and a stone countertop 3 cm over its top.
        let cabinet = host(
            &mut doc,
            &Build::Cabinet(CabinetParams {
                w: 80.0,
                h: 87.0,
                d: 57.0,
                ..CabinetParams::default()
            }),
        );
        let top = host(
            &mut doc,
            &Build::Countertop(CountertopParams {
                length: 80.0,
                depth: 57.0,
                ..CountertopParams::default()
            }),
        );
        let bowl = piece(&mut doc, "sink-bowl", "Cuba", [50.0, 40.0, 18.0]);
        let request = |dry| EmbedRequest {
            item: bowl.clone(),
            existing: false,
            host: top,
            at: None,
            z: None,
            dry,
        };
        let dry = embed(&mut doc, &request(true)).unwrap();
        assert_eq!(dry["cut_below"], json!([cabinet.to_string()]), "{dry}");
        let reply = embed(&mut doc, &request(false)).unwrap();
        assert_eq!(reply["cut_below"], json!([cabinet.to_string()]), "{reply}");
        let group = doc
            .home()
            .furniture
            .iter()
            .find(|f| f.id == cabinet)
            .unwrap();
        let Ok(Build::Cabinet(p)) = merged(&group.properties[PARAMS_KEY], &json!({})) else {
            panic!("still a cabinet");
        };
        // The stone's hole, centered over the 80 × 57 cabinet.
        assert_eq!(p.top_cutout, Some([17.5, 11.0, 45.0, 35.0]));
        assert!(
            !group.children.iter().any(|c| c.name == "Tampo"),
            "the whole board is gone"
        );
        // Embedded again, nothing more to cut.
        let bowl_id = reply["item"].as_str().unwrap().parse().unwrap();
        let again = doc.home().find_piece(bowl_id).unwrap().clone();
        let reply = embed(
            &mut doc,
            &EmbedRequest {
                item: again,
                existing: true,
                ..request(false)
            },
        )
        .unwrap();
        assert!(reply.get("cut_below").is_none(), "{reply}");
    }

    #[test]
    fn a_tv_hangs_on_a_slatted_panel_at_seated_eye_level() {
        let mut doc = Document::default();
        let panel = host(
            &mut doc,
            &Build::Slats(crate::SlatsParams {
                w: 180.0,
                h: 240.0,
                ..crate::SlatsParams::default()
            }),
        );
        let tv = piece(&mut doc, "tv", "Televisão 55\"", [124.0, 8.0, 72.0]);
        let reply = embed(
            &mut doc,
            &EmbedRequest {
                item: tv.clone(),
                existing: false,
                host: panel,
                at: None,
                z: None,
                dry: false,
            },
        )
        .unwrap();
        assert_eq!(reply["bottom"], 69);
        assert!(reply["notes"].to_string().contains("Passa-fios"));
        let group = doc.home().furniture.iter().find(|f| f.id == panel).unwrap();
        let screen = group.children.iter().find(|c| c.id == tv.id).unwrap();
        // In front of the 1,5 cm backing + 2 cm slats.
        assert!(
            (screen.position.y - (40.0 + 3.5 / 2.0 + 4.0)).abs() < 1e-9,
            "{:?}",
            screen.position
        );
        let small = host(
            &mut doc,
            &Build::Slats(crate::SlatsParams {
                w: 120.0,
                ..crate::SlatsParams::default()
            }),
        );
        let err = embed(
            &mut doc,
            &EmbedRequest {
                item: tv,
                existing: false,
                host: small,
                at: None,
                z: None,
                dry: true,
            },
        )
        .unwrap();
        assert!(
            err["notes"].as_array().is_some_and(|n| n
                .iter()
                .any(|note| note.as_str().is_some_and(|text| text.contains("w = 134")))),
            "{err}"
        );
    }

    #[test]
    fn z_on_a_raised_cabinet_is_above_the_room_floor() {
        let mut doc = Document::default();
        let wall = host(
            &mut doc,
            &Build::Cabinet(CabinetParams {
                w: 60.0,
                h: 100.0,
                d: 40.0,
                plinth: 0.0,
                ..CabinetParams::default()
            }),
        );
        let mut raised = doc
            .home()
            .furniture
            .iter()
            .find(|f| f.id == wall)
            .unwrap()
            .clone();
        // Hung on the wall: its pieces follow the group up.
        raised.elevation = 150.0;
        doc.execute(Command::update(raised)).unwrap();
        let micro = piece(&mut doc, "microwave", "Micro-ondas", [50.0, 35.0, 30.0]);
        let reply = embed(
            &mut doc,
            &EmbedRequest {
                item: micro.clone(),
                existing: false,
                host: wall,
                at: None,
                z: Some(160.0),
                dry: false,
            },
        )
        .unwrap();
        assert_eq!(reply["bottom"], 160, "{reply}");
        let group = doc.home().furniture.iter().find(|f| f.id == wall).unwrap();
        let inside = group.children.iter().find(|c| c.id == micro.id).unwrap();
        assert!(
            (inside.elevation - 160.0).abs() < 1e-9,
            "{}",
            inside.elevation
        );
        // Past the top of the cabinet: in the highest niche it has, and said.
        let other = piece(&mut doc, "microwave", "Micro-ondas", [50.0, 35.0, 30.0]);
        let high = embed(
            &mut doc,
            &EmbedRequest {
                item: other,
                existing: false,
                host: wall,
                at: None,
                z: Some(240.0),
                dry: true,
            },
        )
        .unwrap();
        assert_eq!(high["bottom"], 216.2, "{high}");
    }

    #[test]
    fn an_oven_gets_a_niche_that_fits_or_the_size_to_use() {
        let mut doc = Document::default();
        let tower = host(
            &mut doc,
            &Build::Cabinet(CabinetParams {
                w: 64.0,
                h: 220.0,
                d: 58.0,
                ..CabinetParams::default()
            }),
        );
        let oven = piece(&mut doc, "oven", "Forno de embutir", [60.0, 55.0, 60.0]);
        let reply = embed(
            &mut doc,
            &EmbedRequest {
                item: oven.clone(),
                existing: false,
                host: tower,
                at: None,
                z: None,
                dry: false,
            },
        )
        .unwrap();
        assert_eq!(reply["bottom"], 80);
        let group = doc.home().furniture.iter().find(|f| f.id == tower).unwrap();
        assert!(group.children.iter().any(|c| c.name == "Porta superior 1"));
        let inside = group.children.iter().find(|c| c.id == oven.id).unwrap();
        assert!((inside.elevation - 80.0).abs() < 1e-9);
        // A microwave above it: a second niche, the oven kept.
        let micro = piece(&mut doc, "microwave", "Micro-ondas", [50.0, 40.0, 30.0]);
        let reply = embed(
            &mut doc,
            &EmbedRequest {
                item: micro.clone(),
                existing: false,
                host: tower,
                at: None,
                z: None,
                dry: false,
            },
        )
        .unwrap();
        assert_eq!(reply["bottom"], 145);
        let group = doc.home().furniture.iter().find(|f| f.id == tower).unwrap();
        assert!(
            group.children.iter().any(|c| c.id == oven.id)
                && group.children.iter().any(|c| c.id == micro.id)
        );
        assert_eq!(group.properties[PARAMS_KEY].matches("bottom").count(), 2);
        // Into the oven's niche: done, and said, with the height that
        // would have kept them apart.
        let another = piece(&mut doc, "microwave", "Micro-ondas", [50.0, 40.0, 30.0]);
        let clash = embed(
            &mut doc,
            &EmbedRequest {
                item: another,
                existing: false,
                host: tower,
                at: None,
                z: Some(90.0),
                dry: true,
            },
        )
        .unwrap();
        assert!(
            clash["notes"].as_array().is_some_and(|n| n
                .iter()
                .any(|note| note.as_str().is_some_and(|text| text.contains("z = ")))),
            "{clash}"
        );
        // Too wide for a 50 cm cabinet: the width that works.
        let narrow = host(
            &mut doc,
            &Build::Cabinet(CabinetParams {
                w: 50.0,
                h: 220.0,
                ..CabinetParams::default()
            }),
        );
        let err = embed(
            &mut doc,
            &EmbedRequest {
                item: oven,
                existing: false,
                host: narrow,
                at: None,
                z: None,
                dry: true,
            },
        )
        .unwrap();
        assert!(
            err["notes"].as_array().is_some_and(|n| n
                .iter()
                .any(|note| note.as_str().is_some_and(|text| text.contains("w = 61")))),
            "{err}"
        );
    }
}
