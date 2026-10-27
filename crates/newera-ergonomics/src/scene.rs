//! The plan as the review sees it: pieces sorted into what they are for,
//! rooms sorted into how they are used, and a way to measure the free
//! floor beside a piece.

use geo::{Area, BooleanOps, Contains, Coord, LineString, Polygon, Translate};
use newera_core::{Furniture, FurnitureId, Home, Point2, Room};

/// What a piece is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// Sleeps this many (adults; cribs count as one child).
    Bed(u32),
    Crib,
    Nightstand,
    Wardrobe,
    Dresser,
    /// Seats this many.
    Sofa(u32),
    Armchair,
    CoffeeTable,
    /// A table and the seats around it.
    DiningTable(u32),
    /// Table with chairs drawn in.
    DiningSet(u32),
    Chair,
    Stool,
    Fridge,
    Stove,
    Sink,
    /// Low cabinets and worktops.
    Counter,
    /// Built into a niche under or beside the worktop — a dishwasher, an
    /// oven, a microwave. EN 1116 calls these appliances, not work surface,
    /// and their top is the appliance's own height, never the counter's.
    Appliance,
    /// Extraction over the cooktop.
    Hood,
    WallCabinet,
    Island,
    Toilet,
    Basin,
    Shower,
    Bathtub,
    Washer,
    LaundrySink,
    Desk,
    OfficeChair,
    Storage,
    Tv,
    Switch,
    Outlet,
    Other,
}

impl Use {
    /// Moved around in use: never an obstacle to circulation.
    pub fn movable(self) -> bool {
        matches!(self, Self::Chair | Self::Stool | Self::OfficeChair)
    }
}

/// A piece the review looks at: a catalog piece, or a whole joinery build.
#[derive(Debug, Clone)]
pub struct Unit<'a> {
    pub piece: &'a Furniture,
    pub what: Use,
    /// Joinery parameters, when it is a build.
    pub params: Option<serde_json::Value>,
    /// The piece turned to face where its doors are, when `angle` says
    /// another side; see [`Self::frame`].
    pub built: Option<Furniture>,
}

impl Unit<'_> {
    pub fn label(&self) -> String {
        format!("{} {}", self.piece.name, self.piece.id)
    }

    /// The frame "in front of" and "beside" are measured in: the side its
    /// doors and drawer fronts are built on, as `measure` reads it, not the
    /// `angle` it happened to be placed with.
    pub fn frame(&self) -> &Furniture {
        self.built.as_ref().unwrap_or(self.piece)
    }
}

fn joinery(piece: &Furniture) -> Option<serde_json::Value> {
    serde_json::from_str(piece.properties.get("joinery:params")?).ok()
}

fn classify(piece: &Furniture, params: Option<&serde_json::Value>) -> Use {
    match piece
        .properties
        .get(Furniture::ROLE_KEY)
        .map(String::as_str)
    {
        Some("trim" | "backsplash") => return Use::Other,
        Some("counter") => return Use::Counter,
        _ => {}
    }

    if let Some(p) = params {
        let (lo, hi) = piece.height_range();
        return match p["kind"].as_str() {
            Some("sofa") => {
                let modules = p["modules"]
                    .as_u64()
                    .unwrap_or_else(|| (piece.width / 70.0).floor() as u64);
                Use::Sofa(u32::try_from(modules.max(1)).unwrap_or(1))
            }
            Some("countertop") => {
                let sink = p["cutouts"]
                    .as_array()
                    .is_some_and(|c| c.iter().any(|c| c["kind"] == "sink"));
                if sink { Use::Sink } else { Use::Counter }
            }
            Some("cabinet") if lo >= 100.0 => Use::WallCabinet,
            Some("cabinet") if p["cooktop"] == true => Use::Stove,
            Some("cabinet") if hi > 150.0 => Use::Wardrobe,
            Some("cabinet") => Use::Counter,
            // Fillers close gaps; nobody uses them.
            _ => Use::Other,
        };
    }
    match piece.catalog.as_str() {
        "bed-double" | "bed-queen" | "bed-king" => Use::Bed(2),
        "bed-single" => Use::Bed(1),
        "crib" => Use::Crib,
        "nightstand" => Use::Nightstand,
        "wardrobe" => Use::Wardrobe,
        "dresser" => Use::Dresser,
        "sofa-2" => Use::Sofa(2),
        "sofa-3" => Use::Sofa(3),
        "sofa-l" => Use::Sofa(4),
        "armchair" | "win" => Use::Armchair,
        "coffee-table" | "side-table" => Use::CoffeeTable,
        "dining-table-4" | "round-table" => Use::DiningTable(4),
        "dining-table-6" => Use::DiningTable(6),
        "dining-set-4" => Use::DiningSet(4),
        "dining-set-6" => Use::DiningSet(6),
        "chair" | "ares" => Use::Chair,
        "stool" => Use::Stool,
        "fridge" => Use::Fridge,
        "stove" | "cooktop" => Use::Stove,
        "sink-counter" | "sink-bowl" => Use::Sink,
        "base-cabinet" => Use::Counter,
        "dishwasher" | "microwave" | "oven" => Use::Appliance,
        "wall-cabinet" => Use::WallCabinet,
        "hood" => Use::Hood,
        "kitchen-island" => Use::Island,
        "toilet" => Use::Toilet,
        "basin-cabinet" => Use::Basin,
        "shower" | "shower-glass" => Use::Shower,
        "bathtub" => Use::Bathtub,
        "washer" | "dryer" => Use::Washer,
        "laundry-sink" => Use::LaundrySink,
        "desk" => Use::Desk,
        "office-chair" => Use::OfficeChair,
        "bookcase" | "sideboard" | "tv-stand" => Use::Storage,
        "tv" => Use::Tv,
        c if c.starts_with("switch") => Use::Switch,
        c if c.starts_with("outlet") || c == "data-outlet" => Use::Outlet,
        // A point of a project (a sewer outlet, a water point, a panel) is
        // what its catalog says, whatever it is named after — "Esgoto — vaso"
        // is the pipe behind the toilet, not a toilet.
        _ if piece.discipline.is_some() => Use::Other,
        _ => by_name(piece),
    }
}

/// Imported and grouped pieces carry no catalog id: read their name and size.
fn by_name(piece: &Furniture) -> Use {
    let n = newera_core::fold(&piece.name);
    // Designers number their modules: `10 — Pia: dois gavetões`.
    let n = n
        .split_once(" — ")
        .filter(|(head, _)| head.chars().all(|c| c.is_ascii_alphanumeric()))
        .map_or(n.as_str(), |(_, rest)| rest)
        .to_owned();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    let starts = |words: &[&str]| words.iter().any(|w| n.starts_with(w));
    let (w, h) = (piece.width, piece.height);
    let (lo, hi) = piece.height_range();
    let cabinet = has(&[
        "gavet", "armario", "gabinete", "portas", "modulo", "balcao", "nicho",
    ]);
    if h <= 2.0
        || has(&[
            "tapete",
            "luminaria",
            "pendente",
            "arandela",
            "spot",
            "livro",
            "quadro",
        ])
    {
        Use::Other
    } else if starts(&["cama"]) {
        if has(&["solteiro"]) || w < 120.0 {
            Use::Bed(1)
        } else {
            Use::Bed(2)
        }
    } else if has(&["berco"]) {
        Use::Crib
    } else if has(&["guarda-roupa", "guarda roupa", "roupeiro"]) {
        Use::Wardrobe
    } else if starts(&["sofa"]) {
        let seats = if has(&["2 lugares"]) {
            2
        } else if has(&["3 lugares"]) {
            3
        } else {
            ((w / 70.0).floor() as u32).max(1)
        };
        Use::Sofa(seats)
    } else if starts(&["poltrona"]) {
        Use::Armchair
    } else if has(&["cadeira"]) {
        if has(&["escritorio"]) {
            Use::OfficeChair
        } else {
            Use::Chair
        }
    } else if has(&["banqueta"]) {
        Use::Stool
    } else if has(&["cabeceira", "criado"]) {
        Use::Nightstand
    } else if has(&["escrivaninha"]) || (starts(&["mesa"]) && has(&["escritorio", "estudo"])) {
        Use::Desk
    } else if starts(&["mesa"])
        && !has(&["lateral", "centro", "basculante"])
        && w >= 90.0
        && (65.0..=85.0).contains(&h)
    {
        Use::DiningTable(if w < 140.0 {
            4
        } else if w < 200.0 {
            6
        } else {
            8
        })
    } else if starts(&["mesa"]) {
        Use::CoffeeTable
    } else if (starts(&["vaso", "bacia"]) || has(&["vaso sanit", "bacia sanit"])) && w <= 60.0 {
        Use::Toilet
    } else if has(&["lavatorio"]) {
        Use::Basin
    } else if (starts(&["box"]) || has(&["chuveiro"])) && h >= 150.0 {
        Use::Shower
    } else if has(&["banheira"]) {
        Use::Bathtub
    } else if has(&["geladeira", "refrigerador"]) {
        Use::Fridge
    } else if (has(&["cooktop"]) && h <= 15.0 && w >= 50.0 && !cabinet)
        || (has(&["fogao"]) && h >= 70.0)
    {
        Use::Stove
    } else if (starts(&["pia", "cuba"]) || has(&["pia centralizada"])) && !cabinet {
        Use::Sink
    } else if has(&[
        "lava e seca",
        "lava-roupa",
        "maquina de lavar",
        "lavadora",
        "secadora",
    ]) {
        Use::Washer
    } else if starts(&["tanque"]) {
        Use::LaundrySink
    } else if has(&["coifa", "depurador", "exaustor"]) {
        Use::Hood
    } else if (lo >= 100.0 && cabinet) || has(&["aereo"]) {
        Use::WallCabinet
    } else if has(&[
        "lava-louca",
        "lava louca",
        "lava-loucas",
        "forno",
        "micro-ondas",
        "microondas",
    ]) {
        // An appliance in a niche, not work surface: its top is its own
        // height and says nothing about the countertop (EN 1116).
        Use::Appliance
    } else if (cabinet && h >= 60.0 && lo < 20.0 && h <= 100.0)
        || (has(&["bancada", "tampo", "peninsula"])
            && lo >= 60.0
            && (60.0..=115.0).contains(&hi)
            && piece.width.min(piece.depth) >= 25.0)
    {
        Use::Counter
    } else if (cabinet && h > 150.0) || has(&["rack", "estante", "aparador", "cristaleira"]) {
        Use::Storage
    } else {
        Use::Other
    }
}

/// How a room is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomUse {
    Bedroom,
    Living,
    Dining,
    Kitchen,
    Bathroom,
    Laundry,
    Office,
    Corridor,
    Other,
}

impl RoomUse {
    /// Rooms people stay in for long (sleep, live, work).
    pub fn long_stay(self) -> bool {
        matches!(
            self,
            Self::Bedroom | Self::Living | Self::Dining | Self::Office
        )
    }

    /// Its name as a word of a sentence, translated with it.
    #[must_use]
    pub fn said(self) -> newera_core::text::Text {
        newera_core::text::Text::from(self.name())
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Bedroom => "dormitório",
            Self::Living => "sala",
            Self::Dining => "sala de jantar",
            Self::Kitchen => "cozinha",
            Self::Bathroom => "banheiro",
            Self::Laundry => "área de serviço",
            Self::Office => "escritório",
            Self::Corridor => "circulação",
            Self::Other => "ambiente",
        }
    }
}

/// What a room is by what it says it is — its declared use, or its name in
/// any language we hold. `None` when it says nothing the review reads, and
/// the pieces in it decide.
fn room_use_by_name(room: &Room) -> Option<RoomUse> {
    use newera_core::vocabulary::Mention as M;
    let says = room.mentions();
    Some(if says.any(&[M::Bathroom, M::Lavatory]) {
        RoomUse::Bathroom
    } else if says.has(M::Closet) {
        RoomUse::Other
    } else if says.has(M::Bedroom) {
        RoomUse::Bedroom
    } else if says.any(&[M::Kitchen, M::Pantry]) && !says.has(M::Living) {
        // An open kitchen named with the living room is the living room.
        RoomUse::Kitchen
    } else if says.has(M::Laundry) {
        RoomUse::Laundry
    } else if says.has(M::Dining) {
        RoomUse::Dining
    } else if says.any(&[M::Living, M::Tv]) {
        RoomUse::Living
    } else if says.any(&[M::Office, M::Study]) {
        RoomUse::Office
    } else if says.has(M::Corridor) {
        RoomUse::Corridor
    } else {
        return None;
    })
}

pub(crate) fn polygon(points: &[Point2]) -> Polygon<f64> {
    let mut coords: Vec<Coord<f64>> = points.iter().map(|p| Coord { x: p.x, y: p.y }).collect();
    if let Some(first) = coords.first().copied() {
        coords.push(first);
    }
    Polygon::new(LineString::new(coords), vec![])
}

/// One side of a piece, seen from its front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Front,
    Left,
    Right,
}

/// A room with what is in it.
#[derive(Debug, Clone)]
pub struct Space<'a> {
    pub room: &'a Room,
    pub what: RoomUse,
    pub units: Vec<usize>,
}

impl Space<'_> {
    /// A dedicated clothing room, not a bedroom or a laundry linen cabinet.
    pub(crate) fn is_closet(&self) -> bool {
        self.what == RoomUse::Other
            && self
                .room
                .mentions()
                .has(newera_core::vocabulary::Mention::Closet)
    }

    pub fn label(&self) -> String {
        if self.room.name.is_empty() {
            format!("{} {}", self.what.name(), self.room.id)
        } else {
            format!("{} {}", self.room.name, self.room.id)
        }
    }
}

/// The current storey, ready to review.
#[allow(missing_debug_implementations)]
pub struct Scene<'a> {
    pub home: &'a Home,
    pub units: Vec<Unit<'a>>,
    pub spaces: Vec<Space<'a>>,
    /// Plan outlines that stop movement: walls, then each unit's footprint.
    walls: Vec<Polygon<f64>>,
    footprints: Vec<Polygon<f64>>,
    /// Floor swept by each hinged door: `(door, swing)`.
    pub swings: Vec<(&'a Furniture, Polygon<f64>)>,
    rooms: Vec<Polygon<f64>>,
}

impl<'a> Scene<'a> {
    /// A wardrobe in this room or a closet directly across a real wall opening.
    pub(crate) fn wardrobe_serves(&self, bedroom: &Space<'_>) -> bool {
        let has_wardrobe = |space: &Space<'_>| {
            space
                .units
                .iter()
                .any(|&i| self.units[i].what == Use::Wardrobe)
        };
        if has_wardrobe(bedroom) {
            return true;
        }
        let bedroom_shape = polygon(&bedroom.room.points);
        let cuts = self.home.wall_cuts_of(&self.home.furniture);
        self.spaces
            .iter()
            .filter(|s| s.is_closet() && has_wardrobe(s))
            .any(|closet| {
                let shape = polygon(&closet.room.points);
                self.home.walls.iter().zip(&cuts).any(|(wall, cuts)| {
                    cuts.iter().any(|cut| {
                        if cut.bottom > 2.0 || cut.top < 150.0 {
                            return false;
                        }
                        let Some(door) = self.home.furniture.iter().find(|f| f.id == cut.furniture)
                        else {
                            return false;
                        };
                        if !door
                            .opening
                            .as_ref()
                            .is_some_and(|o| o.kind != newera_core::OpeningKind::Window)
                        {
                            return false;
                        }
                        let (sin, cos) = door.angle.to_radians().sin_cos();
                        let reach = wall.thickness.max(door.depth) / 2.0 + 20.0;
                        let a = geo::Point::new(
                            door.position.x - sin * reach,
                            door.position.y + cos * reach,
                        );
                        let b = geo::Point::new(
                            door.position.x + sin * reach,
                            door.position.y - cos * reach,
                        );
                        (bedroom_shape.contains(&a) && shape.contains(&b))
                            || (bedroom_shape.contains(&b) && shape.contains(&a))
                    })
                })
            })
    }

    /// `home` must already be a single storey's view.
    pub fn new(home: &'a Home) -> Self {
        let mut units = Vec::new();
        for top in home.furniture.iter().filter(|f| f.visible) {
            if top.is_opening() {
                continue;
            }
            // A group is one piece of furniture made of parts.
            let params = joinery(top);
            // Perimeter ceiling builds enclose empty room space. Their
            // generator frame extends to the floor; it is not a solid box.
            if !top.children.is_empty()
                && params
                    .as_ref()
                    .is_some_and(|p| matches!(p["kind"].as_str(), Some("cove" | "shadow_gap")))
            {
                units.extend(top.visible_leaves().into_iter().map(|piece| Unit {
                    piece,
                    what: Use::Other,
                    params: None,
                    built: None,
                }));
                continue;
            }
            let what = classify(top, params.as_ref());
            units.push(Unit {
                piece: top,
                what,
                params,
                built: newera_core::built_frame(top),
            });
            // Items embedded in joinery count on their own (a TV on its panel),
            // however deep in the group, for what the host says they are:
            // their name may be in any language.
            for child in top.flatten().into_iter().skip(1).filter(|c| c.visible) {
                let what = match child.properties.get("joinery:embedded").map(String::as_str) {
                    None => continue,
                    Some("cooktop") => Use::Stove,
                    Some("sink") => Use::Sink,
                    Some("fridge") => Use::Fridge,
                    Some("oven" | "microwave" | "dishwasher") => Use::Appliance,
                    Some("tv") => Use::Tv,
                    Some(_) => classify(child, None),
                };
                units.push(Unit {
                    piece: child,
                    what,
                    params: None,
                    built: None,
                });
            }
        }
        // A cabinet marked for cooking describes a zone until an actual
        // cooktop is installed. Once present, the appliance supplies the
        // cooking dimensions and fuel identity; the host remains cabinetry.
        let cooking_hosts: Vec<usize> = units
            .iter()
            .enumerate()
            .filter(|(_, host)| {
                host.what == Use::Stove
                    && host.params.as_ref().is_some_and(|p| p["kind"] == "cabinet")
                    && units.iter().any(|appliance| {
                        if appliance.what != Use::Stove || appliance.params.is_some() {
                            return false;
                        }
                        let (lo, hi) = appliance.piece.height_range();
                        let top = host.piece.height_range().1;
                        hi - lo <= 20.0
                            && lo >= top - 15.0
                            && lo <= top + 10.0
                            && hi >= top - 5.0
                            && appliance.piece.projected_footprint().iter().all(|&p| {
                                let (x, y) = host.piece.to_local(p);
                                x.abs() <= host.piece.width / 2.0 + 1.0
                                    && y.abs() <= host.piece.depth / 2.0 + 1.0
                            })
                    })
            })
            .map(|(i, _)| i)
            .collect();
        for i in cooking_hosts {
            units[i].what = Use::Counter;
        }
        let footprints: Vec<Polygon<f64>> = units
            .iter()
            .map(|u| polygon(&u.piece.projected_footprint()))
            .collect();
        let walls = home
            .wall_outlines()
            .iter()
            .filter(|o| o.len() >= 3)
            .map(|o| polygon(o))
            .collect();
        let mut spaces: Vec<Space<'a>> = home
            .rooms
            .iter()
            .filter(|r| r.points.len() >= 3)
            .map(|room| {
                let shape = polygon(&room.points);
                let inside: Vec<usize> = units
                    .iter()
                    .enumerate()
                    .filter(|(_, u)| {
                        let p = u.piece.position;
                        shape.contains(&geo::Point::new(p.x, p.y))
                    })
                    .map(|(i, _)| i)
                    .collect();
                Space {
                    room,
                    what: RoomUse::Other,
                    units: inside,
                }
            })
            .collect();
        for space in &mut spaces {
            let has = |f: &dyn Fn(Use) -> bool| space.units.iter().any(|&i| f(units[i].what));
            space.what = room_use_by_name(space.room).unwrap_or_else(|| {
                if !space.room.usage.is_auto() {
                    return RoomUse::Other;
                }
                if has(&|u| matches!(u, Use::Toilet | Use::Shower | Use::Bathtub)) {
                    RoomUse::Bathroom
                } else if has(&|u| matches!(u, Use::Bed(_) | Use::Crib)) {
                    RoomUse::Bedroom
                } else if has(&|u| matches!(u, Use::Stove | Use::Fridge | Use::Sink)) {
                    RoomUse::Kitchen
                } else if has(&|u| matches!(u, Use::Washer | Use::LaundrySink)) {
                    RoomUse::Laundry
                } else if has(&|u| matches!(u, Use::Sofa(_) | Use::Armchair)) {
                    RoomUse::Living
                } else if has(&|u| matches!(u, Use::DiningTable(_) | Use::DiningSet(_))) {
                    RoomUse::Dining
                } else if has(&|u| matches!(u, Use::Desk)) {
                    RoomUse::Office
                } else {
                    RoomUse::Other
                }
            });
        }
        // A bowl in a bathroom is a basin, whatever it is called or set in:
        // a "cuba" in a vanity top is washed at, not cooked beside.
        for space in spaces.iter().filter(|s| s.what == RoomUse::Bathroom) {
            for &i in &space.units {
                if units[i].what == Use::Sink {
                    units[i].what = Use::Basin;
                }
            }
        }
        let swings = home
            .furniture
            .iter()
            .filter(|f| f.visible)
            .filter_map(|door| Some((door, polygon(&newera_core::door_swing(door)?))))
            .collect();
        let rooms = home.rooms.iter().map(|r| polygon(&r.points)).collect();
        Self {
            home,
            units,
            spaces,
            walls,
            footprints,
            swings,
            rooms,
        }
    }

    /// Free floor beside unit `i`, cm, up to `max`: the nearest wall or
    /// standing piece within the band `from..to` along that side (fractions
    /// of the side's length, from the back or the left).
    pub fn free(&self, i: usize, side: Side, max: f64, span: (f64, f64)) -> f64 {
        self.free_and_blocker(i, side, max, span).0
    }

    /// Like [`Self::free`], with the unit that stops it (`None` for a wall or nothing).
    pub fn free_and_blocker(
        &self,
        i: usize,
        side: Side,
        max: f64,
        span: (f64, f64),
    ) -> (f64, Option<usize>) {
        let (free, blocker, _) = self.free_along(i, side, max, span, 0.0);
        (free, blocker)
    }

    /// Like [`Self::free_and_blocker`], plus how many centimeters of the
    /// side have less than `need` free.
    ///
    /// The free floor is the worst point of the side, and the worst point
    /// alone makes a wardrobe with 93 cm in front of its doors read like one
    /// that does not open, because a nightstand takes 38 cm of one corner.
    pub fn free_along(
        &self,
        i: usize,
        side: Side,
        max: f64,
        span: (f64, f64),
        need: f64,
    ) -> (f64, Option<usize>, f64) {
        let piece = self.units[i].frame();
        let (hw, hd) = (piece.width / 2.0, piece.depth / 2.0);
        // The band in the piece's frame, 2 cm in from the corners so
        // neighbours that merely touch a corner don't count.
        let (x0, x1, y0, y1) = match side {
            Side::Front => (
                -hw + 2.0 + (piece.width - 4.0) * span.0,
                -hw + 2.0 + (piece.width - 4.0) * span.1,
                hd + 0.5,
                hd + max,
            ),
            Side::Left | Side::Right => {
                let (a, b) = (
                    -hd + 2.0 + (piece.depth - 4.0) * span.0,
                    -hd + 2.0 + (piece.depth - 4.0) * span.1,
                );
                if side == Side::Left {
                    (-hw - max, -hw - 0.5, a, b)
                } else {
                    (hw + 0.5, hw + max, a, b)
                }
            }
        };
        let band: Vec<Point2> = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
            .into_iter()
            .map(|p| piece.to_plan(p))
            .collect();
        let band = polygon(&band);
        let mut free = max;
        let mut blocker = None;
        // Stretches of the side, `(from, to)` along it, with less than `need`.
        let mut tight: Vec<(f64, f64)> = Vec::new();
        let mut measure = |shape: &Polygon<f64>, who: Option<usize>| {
            for poly in shape.intersection(&band) {
                let (mut near, mut from, mut to) = (f64::MAX, f64::MAX, f64::MIN);
                for c in poly.exterior().coords() {
                    let (x, y) = piece.to_local(Point2::new(c.x, c.y));
                    let (d, along) = match side {
                        Side::Front => (y - hd, x),
                        Side::Left => (-hw - x, y),
                        Side::Right => (x - hw, y),
                    };
                    near = near.min(d.max(0.0));
                    from = from.min(along);
                    to = to.max(along);
                    if d.max(0.0) < free {
                        free = d.max(0.0);
                        blocker = who;
                    }
                }
                if near + 0.5 < need {
                    tight.push((from, to));
                }
            }
        };
        for wall in &self.walls {
            measure(wall, None);
        }
        for (j, other) in self.units.iter().enumerate() {
            let (lo, hi) = other.piece.height_range();
            if j == i
                || self.thin(j)
                || self.embedded(j)
                || other.what.movable()
                || matches!(other.what, Use::Switch | Use::Outlet)
                || other.piece.discipline.is_some()
                || hi - lo < 20.0
                || lo >= 190.0
                || hi <= 5.0
                // Cabinets on the wall above a counter don't stop a person.
                || (lo >= 100.0 && self.units[i].piece.height_range().1 <= 100.0)
            {
                continue;
            }
            // Resting on or joined to this piece (a countertop over cabinets).
            if self.footprints[j]
                .intersection(&self.footprints[i])
                .unsigned_area()
                > 25.0
            {
                continue;
            }
            measure(&self.footprints[j], Some(j));
        }
        tight.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mut length, mut reach) = (0.0, f64::MIN);
        for (from, to) in tight {
            let from = from.max(reach);
            if to > from {
                length += to - from;
                reach = to;
            }
        }
        (free, blocker, length)
    }

    /// Whether a unit stands in the way of people and other pieces.
    pub fn solid(&self, i: usize) -> bool {
        let u = &self.units[i];
        let (lo, hi) = u.piece.height_range();
        !u.what.movable()
            && !self.thin(i)
            && !self.embedded(i)
            && !self.embedded_item(i)
            && !matches!(u.what, Use::Switch | Use::Outlet)
            && u.piece.discipline.is_none()
            && hi - lo >= 20.0
            && lo < 190.0
            && hi > 5.0
    }

    /// A joinery countertop: it rests on cabinets and holds sinks and cooktops.
    pub fn countertop(&self, i: usize) -> bool {
        self.units[i]
            .params
            .as_ref()
            .is_some_and(|p| p["kind"] == "countertop")
    }

    /// An item embedded in joinery: its host answers for it.
    pub fn embedded_item(&self, i: usize) -> bool {
        self.units[i]
            .piece
            .properties
            .contains_key("joinery:embedded")
    }

    /// Panels, facings, glass and rails: parts of something else.
    pub fn thin(&self, i: usize) -> bool {
        let p = self.units[i].piece;
        p.width.min(p.depth) <= 3.0
    }

    /// Sink bowls, basins and cooktops set into a countertop.
    pub fn embedded(&self, i: usize) -> bool {
        let u = &self.units[i];
        let (lo, hi) = u.piece.height_range();
        matches!(u.what, Use::Sink | Use::Basin | Use::Stove) && lo >= 50.0 && hi - lo <= 45.0
    }

    /// What unit `i` fills between the floor and the ceiling, cm. A
    /// countertop is its slab: its box reaches the floor, and under the slab
    /// is where a washer or a dishwasher stands.
    fn heights(&self, i: usize) -> (f64, f64) {
        let (lo, hi) = self.units[i].piece.height_range();
        match &self.units[i].params {
            Some(p) if p["kind"] == "countertop" => {
                let thickness = p["thickness"].as_f64().unwrap_or(3.0);
                ((hi - thickness).max(lo), hi)
            }
            _ => (lo, hi),
        }
    }

    /// Area two outlines share, cm².
    fn shared(a: &Polygon<f64>, b: &Polygon<f64>) -> f64 {
        a.intersection(b).unsigned_area()
    }

    /// Whether units `i` (moved by `dx`, `dy`) and `j` collide: same heights,
    /// real shared floor, and not one built into the other (a cooktop in its
    /// countertop, an oven in its tower).
    fn collide(&self, i: usize, moved: &Polygon<f64>, j: usize) -> bool {
        if i == j || !self.solid(j) {
            return false;
        }
        let ((a0, a1), (b0, b1)) = (self.heights(i), self.heights(j));
        if a0 >= b1 - 0.5 || b0 >= a1 - 0.5 {
            return false;
        }
        // A countertop on its cabinets, or next to appliances under it, is the kitchen working.
        let kitchen = |k: usize| {
            matches!(
                self.units[k].what,
                Use::Counter | Use::Sink | Use::Stove | Use::WallCabinet | Use::Fridge
            )
        };
        if (self.countertop(i) && kitchen(j)) || (self.countertop(j) && kitchen(i)) {
            return false;
        }
        let common = moved.intersection(&self.footprints[j]);
        let shared = common.unsigned_area();
        if shared <= 25.0 {
            return false;
        }
        // Pieces touching with a centimeter or two of drawing slack don't collide.
        let depth = common
            .iter()
            .filter_map(geo::MinimumRotatedRect::minimum_rotated_rect)
            .map(|r| {
                let c: Vec<_> = r.exterior().coords().copied().collect();
                let a = (c[1].x - c[0].x).hypot(c[1].y - c[0].y);
                let b = (c[2].x - c[1].x).hypot(c[2].y - c[1].y);
                a.min(b)
            })
            .fold(0.0, f64::max);
        if depth <= 2.0 {
            return false;
        }
        // Mostly inside a cabinet or counter, and something that goes in one
        // (an oven in its tower, a dishwasher under the counter): built in.
        let (area_i, area_j) = (moved.unsigned_area(), self.footprints[j].unsigned_area());
        let (host, guest) = if area_i >= area_j { (i, j) } else { (j, i) };
        let cabinetry = |u: Use| {
            matches!(
                u,
                Use::Counter
                    | Use::Storage
                    | Use::Wardrobe
                    | Use::WallCabinet
                    | Use::Island
                    | Use::Other
            )
        };
        let fits_in = |u: Use| {
            !matches!(
                u,
                Use::Toilet
                    | Use::Basin
                    | Use::Shower
                    | Use::Bathtub
                    | Use::Bed(_)
                    | Use::Crib
                    | Use::Sofa(_)
                    | Use::Armchair
                    | Use::DiningTable(_)
                    | Use::DiningSet(_)
                    | Use::Desk
            )
        };
        let built_in = shared >= 0.5 * area_i.min(area_j)
            && cabinetry(self.units[host].what)
            && fits_in(self.units[guest].what);
        !built_in
    }

    /// Problems unit `i` would have moved by `dx`, `dy`: walls it enters,
    /// pieces it collides with, door swings it stands in.
    pub fn conflicts(&self, i: usize, dx: f64, dy: f64) -> usize {
        if !self.solid(i) {
            return 0;
        }
        let moved = self.footprints[i].translate(dx, dy);
        let walls = self
            .walls
            .iter()
            .filter(|w| Self::shared(&moved, w) > 25.0)
            .count();
        let pieces = (0..self.units.len())
            .filter(|&j| self.collide(i, &moved, j))
            .count();
        let low = self.units[i].piece.height_range().0 < 200.0;
        let doors = self
            .swings
            .iter()
            .filter(|(_, swing)| low && Self::shared(&moved, swing) > 25.0)
            .count();
        walls + pieces + doors
    }

    /// Pairs of units that collide where they stand.
    pub fn overlaps(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for i in 0..self.units.len() {
            if !self.solid(i) {
                continue;
            }
            for j in i + 1..self.units.len() {
                if self.collide(i, &self.footprints[i], j) {
                    out.push((i, j));
                }
            }
        }
        out
    }

    /// Doors whose leaf sweeps through a unit: `(door, unit)`.
    pub fn door_hits(&self) -> Vec<(&'a Furniture, usize)> {
        let mut out = Vec::new();
        for (door, swing) in &self.swings {
            for i in 0..self.units.len() {
                if self.solid(i)
                    && self.units[i].piece.height_range().0 < 200.0
                    && Self::shared(swing, &self.footprints[i]) > 25.0
                {
                    out.push((*door, i));
                }
            }
        }
        out
    }

    /// Whether a swing polygon is clear of every unit.
    pub fn swing_clear(&self, swing: &[Point2]) -> bool {
        let swing = polygon(swing);
        (0..self.units.len()).all(|i| {
            !self.solid(i)
                || self.units[i].piece.height_range().0 >= 200.0
                || Self::shared(&swing, &self.footprints[i]) <= 25.0
        })
    }

    /// Centimeters of unit `i`'s sides (moved by `dx`, `dy`) held against
    /// walls and standing pieces at its own height: the two cabinets a
    /// dishwasher sits between, the tower beside an oven.
    ///
    /// A piece built into a run is not glued to it — it is a separate piece
    /// in a niche — so the only trace of the niche is this contact. A move
    /// that loses it takes the appliance out of its joinery, whatever it
    /// gains in the corridor.
    pub fn held(&self, i: usize, dx: f64, dy: f64) -> f64 {
        const BAND: f64 = 2.0;
        let piece = self.units[i].piece;
        let (hw, hd) = (piece.width / 2.0, piece.depth / 2.0);
        let (z0, z1) = piece.height_range();
        let bands = [
            (-hw, hw, hd, hd + BAND),
            (-hw, hw, -hd - BAND, -hd),
            (-hw - BAND, -hw, -hd, hd),
            (hw, hw + BAND, -hd, hd),
        ];
        let others: Vec<&Polygon<f64>> = self
            .walls
            .iter()
            .chain((0..self.units.len()).filter_map(|j| {
                let (b0, b1) = self.units[j].piece.height_range();
                (j != i && self.solid(j) && b0 < z1 - 1.0 && z0 < b1 - 1.0)
                    // Resting on it or built into it is not beside it.
                    .then_some(&self.footprints[j])
                    .filter(|f| Self::shared(f, &self.footprints[i]) <= 25.0)
            }))
            .collect();
        bands
            .iter()
            .map(|&(x0, x1, y0, y1)| {
                let band: Vec<Point2> = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
                    .into_iter()
                    .map(|p| {
                        let at = piece.to_plan(p);
                        Point2::new(at.x + dx, at.y + dy)
                    })
                    .collect();
                let band = polygon(&band);
                others
                    .iter()
                    .map(|o| Self::shared(&band, o))
                    .sum::<f64>()
                    .min(band.unsigned_area())
                    / BAND
            })
            .sum()
    }

    /// How many sides of unit `i` (moved by `dx`, `dy`) rest against a wall.
    pub fn contacts(&self, i: usize, dx: f64, dy: f64) -> usize {
        let piece = self.units[i].piece;
        let (hw, hd) = (piece.width / 2.0, piece.depth / 2.0);
        [
            (0.0, -hd - 2.0),
            (0.0, hd + 2.0),
            (-hw - 2.0, 0.0),
            (hw + 2.0, 0.0),
        ]
        .into_iter()
        .filter(|&p| {
            let at = piece.to_plan(p);
            self.walls
                .iter()
                .any(|w| w.contains(&geo::Point::new(at.x + dx, at.y + dy)))
        })
        .count()
    }

    /// Index of the room containing a point.
    pub fn room_at(&self, p: Point2) -> Option<usize> {
        self.rooms
            .iter()
            .position(|r| r.contains(&geo::Point::new(p.x, p.y)))
    }

    /// A unit's id.
    pub fn id(&self, i: usize) -> FurnitureId {
        self.units[i].piece.id
    }

    /// Largest circle (diameter, cm) that fits on the free floor of a room,
    /// sampled every 5 cm — enough to tell whether a wheelchair turns.
    pub fn turning_diameter(&self, space: &Space<'_>) -> f64 {
        let pts = &space.room.points;
        let (lo, hi) = pts.iter().fold(
            (
                Point2::new(f64::MAX, f64::MAX),
                Point2::new(f64::MIN, f64::MIN),
            ),
            |(lo, hi), p| {
                (
                    Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                    Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
                )
            },
        );
        let shape = polygon(pts);
        let obstacles: Vec<Vec<Point2>> = space
            .units
            .iter()
            .filter(|&&i| {
                let u = &self.units[i];
                let (lo, hi) = u.piece.height_range();
                !u.what.movable() && u.piece.discipline.is_none() && hi - lo >= 20.0 && lo < 70.0
            })
            .map(|&i| self.units[i].piece.projected_footprint().to_vec())
            .collect();
        let edges = |poly: &[Point2]| {
            (0..poly.len())
                .map(|k| (poly[k], poly[(k + 1) % poly.len()]))
                .collect::<Vec<_>>()
        };
        let mut segments = edges(pts);
        for o in &obstacles {
            segments.extend(edges(o));
        }
        let mut best: f64 = 0.0;
        let step = 5.0;
        let mut y = lo.y + step / 2.0;
        while y < hi.y {
            let mut x = lo.x + step / 2.0;
            while x < hi.x {
                let p = Point2::new(x, y);
                let inside = shape.contains(&geo::Point::new(x, y))
                    && !obstacles
                        .iter()
                        .any(|o| polygon(o).contains(&geo::Point::new(x, y)));
                if inside {
                    let r = segments
                        .iter()
                        .map(|(a, b)| p.distance_to_segment(*a, *b))
                        .fold(f64::MAX, f64::min);
                    best = best.max(2.0 * r);
                }
                x += step;
            }
            y += step;
        }
        best
    }
}

#[cfg(test)]
mod role_tests {
    use super::*;

    #[test]
    fn ceiling_perimeters_use_visible_components_instead_of_the_room_envelope() {
        for kind in ["cove", "shadow_gap"] {
            let mut home = Home::default();
            let block = Furniture {
                id: FurnitureId(2),
                catalog: "armchair".into(),
                width: 40.0,
                depth: 40.0,
                height: 80.0,
                position: Point2::new(120.0, 60.0),
                ..Furniture::default()
            };
            let mut ceiling_part = block.clone();
            ceiling_part.id = FurnitureId(3);
            ceiling_part.elevation = 260.0;
            ceiling_part.height = 40.0;
            let mut hidden = block.clone();
            hidden.id = FurnitureId(4);
            hidden.visible = false;
            let group = Furniture {
                id: FurnitureId(1),
                width: 400.0,
                depth: 400.0,
                height: 300.0,
                position: Point2::new(200.0, 200.0),
                children: vec![ceiling_part, hidden],
                properties: [("joinery:params".into(), format!(r#"{{"kind":"{kind}"}}"#))]
                    .into_iter()
                    .collect(),
                ..Furniture::default()
            };
            let door = Furniture {
                id: FurnitureId(5),
                position: Point2::new(150.0, 0.0),
                width: 70.0,
                depth: 15.0,
                height: 210.0,
                opening: Some(newera_core::Opening::default()),
                ..Furniture::default()
            };
            home.furniture = vec![group, block, door];
            let scene = Scene::new(&home);
            assert_eq!(scene.units.len(), 2);
            assert!(
                scene.overlaps().is_empty(),
                "{kind}: empty envelope is not solid"
            );
            assert!(
                scene
                    .door_hits()
                    .iter()
                    .all(|(_, i)| scene.id(*i) == FurnitureId(2))
            );
            // A real component lowered into the furniture/door must still
            // obstruct them. The kind is not a blanket collision exemption.
            home.furniture[0].children[0].elevation = 20.0;
            let scene = Scene::new(&home);
            assert_eq!(scene.overlaps().len(), 1, "{kind}");
            assert!(
                scene
                    .door_hits()
                    .iter()
                    .any(|(_, i)| scene.id(*i) == FurnitureId(3))
            );
            home.furniture[0].visible = false;
            assert_eq!(Scene::new(&home).units.len(), 1);
        }
    }

    #[test]
    fn a_vertical_backsplash_is_not_a_counter_because_of_its_name() {
        let mut piece = Furniture {
            catalog: "box".into(),
            width: 355.0,
            depth: 2.0,
            height: 18.0,
            elevation: 90.0,
            ..Furniture::default()
        };
        for name in [
            "Frontão de pedra — bancada norte",
            "Frontão de pedra — cozinha",
            "Tampo",
            "Bancada",
        ] {
            piece.name = name.into();
            assert_eq!(classify(&piece, None), Use::Other);
        }
        piece.depth = 60.0;
        piece.height = 3.0;
        piece.elevation = 87.0;
        assert_eq!(classify(&piece, None), Use::Counter);
        piece
            .properties
            .insert(Furniture::ROLE_KEY.into(), "backsplash".into());
        assert_eq!(classify(&piece, None), Use::Other);
        piece.name = "Painel sem nome de bancada".into();
        assert_eq!(classify(&piece, None), Use::Other);
        piece
            .properties
            .insert(Furniture::ROLE_KEY.into(), "counter".into());
        piece.name = "Ilha de preparo".into();
        assert_eq!(classify(&piece, None), Use::Counter);
    }

    #[test]
    fn branded_seats_keep_their_role_when_renamed() {
        for (catalog, expected) in [("win", Use::Armchair), ("ares", Use::Chair)] {
            let piece = Furniture {
                catalog: catalog.into(),
                name: "Renamed product".into(),
                ..Furniture::default()
            };
            assert_eq!(classify(&piece, None), expected);
            assert!(piece.is_seat());
        }
    }
}
