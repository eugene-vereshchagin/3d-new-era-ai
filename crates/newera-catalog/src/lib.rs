//! Parametric furniture catalog.
//!
//! Procedural furniture and bundled product models at exact sizes. Each item
//! provides a 3D mesh and an architectural plan symbol; bundled assets include
//! their materials and textures in the application.

mod bundled;
pub use bundled::{
    files as bundled_files, generation as bundled_generation, mount_fetched as mount_bundled,
};
mod check;
mod finish;
mod fixtures;
mod import;
mod mesh;
mod models;
mod symbols;

pub use check::{Clash, clashes, degenerate};
pub use fixtures::DIFFUSER;
pub use import::{ImportError, ImportReport, ImportedModel, Unit, load_model, load_model_in};
pub use mesh::{Axis, GLASS, Mesh, MeshMaterial, MeshPart, Rgb, is_glass, rgb, shade};
use newera_core::{Furniture, Opening, OpeningKind};
/// Build-time source fingerprint for persistent generated-image caches.
pub const GENERATOR_REVISION: &str = env!("NEWERA_SOURCE_REVISION");
pub use symbols::{SymbolShape, plan_symbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Living,
    Dining,
    Kitchen,
    Bedroom,
    Bathroom,
    Laundry,
    Office,
    DoorsWindows,
    Structure,
    Decor,
    Lighting,
    Outdoor,
    Electrical,
    Plumbing,
}

impl Category {
    pub const ALL: [Self; 14] = [
        Self::Living,
        Self::Dining,
        Self::Kitchen,
        Self::Bedroom,
        Self::Bathroom,
        Self::Laundry,
        Self::Office,
        Self::DoorsWindows,
        Self::Structure,
        Self::Decor,
        Self::Lighting,
        Self::Outdoor,
        Self::Electrical,
        Self::Plumbing,
    ];

    /// Technical project whose symbols this category holds.
    pub fn discipline(self) -> Option<newera_core::Discipline> {
        match self {
            Self::Electrical => Some(newera_core::Discipline::Electrical),
            Self::Plumbing => Some(newera_core::Discipline::Plumbing),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Living => "living",
            Self::Dining => "dining",
            Self::Kitchen => "kitchen",
            Self::Bedroom => "bedroom",
            Self::Bathroom => "bathroom",
            Self::Laundry => "laundry",
            Self::Office => "office",
            Self::DoorsWindows => "doors-windows",
            Self::Structure => "structure",
            Self::Decor => "decor",
            Self::Lighting => "lighting",
            Self::Outdoor => "outdoor",
            Self::Electrical => "electrical",
            Self::Plumbing => "plumbing",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Living => "Sala de estar",
            Self::Dining => "Sala de jantar",
            Self::Kitchen => "Cozinha",
            Self::Bedroom => "Quarto",
            Self::Bathroom => "Banheiro",
            Self::Laundry => "Lavanderia",
            Self::Office => "Escritório",
            Self::DoorsWindows => "Portas e janelas",
            Self::Structure => "Estrutura",
            Self::Decor => "Decoração",
            Self::Lighting => "Iluminação",
            Self::Outdoor => "Área externa",
            Self::Electrical => "Elétrica",
            Self::Plumbing => "Hidráulica",
        }
    }
}

/// Risers of a straight flight climbing `height` cm: the fewest that keep
/// each at most 18 cm, the top of what a step comfortably rises (NBR 9050:
/// 16 to 18). The 2.8 m catalog flight has 16 of 17.5 cm, and one resized to
/// 1.31 m has 8 of 16.4 — not sixteen of 8 cm, as when the count was fixed.
#[must_use]
pub fn stair_risers(height: f64) -> u32 {
    // At least 2, and a count any height a plan can hold fits in.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (height / 18.0).ceil().clamp(2.0, f64::from(u32::MAX)) as u32;
    n
}

/// Which procedural generator builds an item, with its style parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Model {
    Sofa {
        seats: u8,
    },
    Bed {
        pillows: u8,
    },
    Table {
        round: bool,
    },
    Chair,
    Stool,
    OfficeChair,
    Cabinet {
        doors: u8,
        drawers: u8,
    },
    Shelf {
        levels: u8,
    },
    TvStand,
    Tv,
    Fridge,
    Stove,
    SinkCounter,
    Counter,
    WallCabinet,
    Appliance {
        round_door: bool,
    },
    Microwave,
    /// Extraction over the cooktop: a canopy narrowing into a duct.
    Hood,
    /// Glass cooktop set into a countertop (its height is the part above it).
    Cooktop,
    /// Steel bowl set into a countertop: the rim at the top, the bowl below.
    SinkBowl,
    /// Built-in oven: glass door, control strip and handle.
    Oven,
    Toilet,
    Basin,
    Shower,
    Bathtub,
    Door,
    Window {
        panes: u8,
    },
    Passage,
    /// A straight flight; its risers follow its height ([`stair_risers`]).
    Stairs,
    Plant,
    Tree,
    Lamp,
    Rug,
    Car,
    Desk,
    Crib,
    Column {
        round: bool,
    },
    Box,
    /// Concrete footing: wide base under a pedestal.
    Footing,
    /// Posts with top and middle rails.
    Railing,
    /// Posts with horizontal boards.
    Fence,
    /// Corrugated roof or wall sheet, ridges along the depth.
    Corrugated,
    /// Glass panel in a thin metal frame (shower screens, balustrades).
    GlassPanel,
    /// Laminated glass infill between posts, under a metal handrail.
    GlassRailing,
    /// Retractable balcony glazing: frameless glass leaves between a top and
    /// a bottom track, from the slab to the ceiling.
    BalconyGlazing,
    /// Sectional garage door.
    GarageDoor,
    /// Pool: coping around a water surface, rectangular or oval.
    Pool {
        oval: bool,
    },
    /// Sun lounger with a raised back.
    Lounger,
    /// Masonry barbecue with a chimney.
    Grill,
    /// Sofa with a chaise on its right end.
    SofaL,
    Bench,
    /// Table with chairs around it.
    DiningSet {
        chairs: u8,
    },
    /// Planter box with greenery.
    Planter,
    /// A technical point drawn with a conventional plan symbol.
    Point(PointSymbol),
    /// Pop-up outlet tower set into a countertop: the flange on the stone,
    /// the raised body with its sockets, the body hanging under the top.
    OutletTower,
    /// Outlet box set into a desk or table: a collar with a hinged lid.
    DeskBox,
}

/// Plan symbols of electrical and plumbing points, after the usual
/// Brazilian drawing conventions (NBR 5444 style).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointSymbol {
    OutletLow,
    OutletMid,
    OutletHigh,
    Switch1,
    Switch2,
    Switch3Way,
    LightCeiling,
    LightWall,
    Panel,
    AirConditioner,
    ShowerPoint,
    DataOutlet,
    NetworkOutlet,
    TvOutlet,
    WifiPoint,
    TelecomPanel,
    Doorbell,
    SmartRelay,
    SmartSwitch,
    Dimmer,
    PresenceSensor,
    SmartLock,
    ColdWater,
    HotWater,
    Sewer,
    FloorDrain,
    Valve,
    GreaseTrap,
    InspectionBox,
    VentPipe,
    VentGrille,
    DryDrain,
    LinearDrain,
    RainDrain,
    WaterMeter,
    Gas,
}

/// Opening settings a catalog door or window starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpeningPreset {
    pub kind: OpeningKind,
    pub leaves: u8,
    pub sliding: bool,
}

impl OpeningPreset {
    pub fn to_opening(self) -> Opening {
        Opening {
            kind: self.kind,
            leaves: self.leaves,
            sliding: self.sliding,
            ..Opening::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CatalogItem {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    /// Default width, depth, height in cm.
    pub size: [f64; 3],
    pub elevation: f64,
    pub color: [u8; 3],
    pub model: Model,
    pub opening: Option<OpeningPreset>,
    /// Light it gives, for fixtures.
    pub light: Option<LightPreset>,
    /// Extra search words (synonyms, English).
    pub keywords: &'static str,
}

/// Photometric data of a fixture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightPreset {
    /// Luminous flux, lm (per meter of width for strips).
    pub lumens: f64,
    pub watts: f64,
    pub kelvin: f64,
    /// Full beam angle, degrees; 0 for light in every direction.
    pub beam: f64,
    /// Emits from its underside as a panel (LED panels and strips).
    pub panel: bool,
    /// Flux scales with the width (LED strips).
    pub per_meter: bool,
    /// Height of the source as a fraction of the piece's height.
    pub z: f64,
}

impl CatalogItem {
    const fn with_light(self, light: LightPreset) -> Self {
        lit(self, light)
    }
}

const fn lit(mut item: CatalogItem, light: LightPreset) -> CatalogItem {
    item.light = Some(light);
    item
}

const fn led(lumens: f64, watts: f64, kelvin: f64, beam: f64, z: f64) -> LightPreset {
    LightPreset {
        lumens,
        watts,
        kelvin,
        beam,
        panel: false,
        per_meter: false,
        z,
    }
}

/// The light a catalog fixture gives at the piece's current size, if any.
pub fn light_for(piece: &Furniture) -> Option<newera_core::Light> {
    let preset = find(&piece.catalog)?.light?;
    let scale = if preset.per_meter {
        piece.width / 100.0
    } else {
        1.0
    };
    let mut light =
        newera_core::Light::led(preset.lumens * scale, preset.kelvin, (0.5, 0.5, preset.z));
    light.watts = Some(preset.watts * scale);
    light.beam = (preset.beam > 0.0).then_some(preset.beam);
    light.area = preset
        .panel
        .then_some([piece.width * 0.96, piece.depth * 0.9]);
    Some(light)
}

/// What a piece counts as in a bill of quantities: its catalog entry's name
/// when it has one — every "Tomada — cozinha, bancada (centro)" is a low
/// outlet — and its own name otherwise.
pub fn quantity_name(piece: &newera_core::Furniture) -> String {
    find(&piece.catalog).map_or_else(|| piece.name.clone(), |item| item.name.to_owned())
}

const fn item(
    id: &'static str,
    name: &'static str,
    category: Category,
    size: [f64; 3],
    color: [u8; 3],
    model: Model,
    keywords: &'static str,
) -> CatalogItem {
    CatalogItem {
        id,
        name,
        category,
        size,
        elevation: 0.0,
        color,
        model,
        opening: None,
        light: None,
        keywords,
    }
}

const fn raised(mut item: CatalogItem, elevation: f64) -> CatalogItem {
    item.elevation = elevation;
    item
}

const fn opening(
    mut item: CatalogItem,
    kind: OpeningKind,
    leaves: u8,
    sliding: bool,
) -> CatalogItem {
    item.opening = Some(OpeningPreset {
        kind,
        leaves,
        sliding,
    });
    item
}

const FABRIC: [u8; 3] = [118, 128, 140];
const WOOD: [u8; 3] = [168, 124, 84];
const WHITE: [u8; 3] = [236, 236, 232];
const STEEL: [u8; 3] = [196, 199, 204];
const DARK: [u8; 3] = [52, 54, 60];
const ELECTRIC: [u8; 3] = [236, 234, 228];

use Category as C;

pub static CATALOG: &[CatalogItem] = &[
    // Living room
    item(
        "win",
        "Poltrona Win",
        C::Living,
        [69.5, 83.5, 73.0],
        [74, 83, 61],
        Model::Sofa { seats: 1 },
        "win poltrona armchair nozes english green pinus",
    ),
    item(
        "ares",
        "Cadeira Ares",
        C::Dining,
        [47.0, 62.0, 81.0],
        [184, 139, 89],
        Model::Chair,
        "ares cadeira chair jantar dining palhinha tauari natural",
    ),
    item(
        "sofa-3",
        "Sofá 3 lugares",
        C::Living,
        [210.0, 90.0, 85.0],
        FABRIC,
        Model::Sofa { seats: 3 },
        "sofa couch sofá",
    ),
    item(
        "sofa-2",
        "Sofá 2 lugares",
        C::Living,
        [160.0, 90.0, 85.0],
        FABRIC,
        Model::Sofa { seats: 2 },
        "sofa couch loveseat",
    ),
    item(
        "sofa-l",
        "Sofá em L (chaise)",
        C::Living,
        [270.0, 160.0, 85.0],
        FABRIC,
        Model::SofaL,
        "sofa l chaise canto",
    ),
    item(
        "armchair",
        "Poltrona",
        C::Living,
        [85.0, 85.0, 85.0],
        [150, 110, 90],
        Model::Sofa { seats: 1 },
        "armchair poltrona cadeira",
    ),
    item(
        "coffee-table",
        "Mesa de centro",
        C::Living,
        [100.0, 60.0, 40.0],
        WOOD,
        Model::Table { round: false },
        "coffee table centro",
    ),
    item(
        "side-table",
        "Mesa lateral",
        C::Living,
        [50.0, 50.0, 55.0],
        WOOD,
        Model::Table { round: true },
        "side table apoio",
    ),
    item(
        "tv-stand",
        "Rack de TV",
        C::Living,
        [180.0, 45.0, 50.0],
        WOOD,
        Model::TvStand,
        "tv stand rack",
    ),
    raised(
        item(
            "tv",
            "Televisão 55\"",
            C::Living,
            [124.0, 8.0, 72.0],
            DARK,
            Model::Tv,
            "tv television tela",
        ),
        60.0,
    ),
    item(
        "bookcase",
        "Estante",
        C::Living,
        [90.0, 35.0, 200.0],
        WOOD,
        Model::Shelf { levels: 5 },
        "bookcase shelf estante livros",
    ),
    item(
        "rug",
        "Tapete",
        C::Living,
        [200.0, 140.0, 1.0],
        [190, 170, 140],
        Model::Rug,
        "rug carpet tapete",
    ),
    raised(
        item(
            "radiator-60",
            "Radiador de painel 60×10×50",
            C::Living,
            [60.0, 10.0, 50.0],
            WHITE,
            Model::Box,
            "radiador radiator aquecedor aquecimento calefacao heater heating hvac termo painel panel agua water hidronico",
        ),
        15.0,
    ),
    raised(
        item(
            "radiator-100",
            "Radiador de painel 100×10×50",
            C::Living,
            [100.0, 10.0, 50.0],
            WHITE,
            Model::Box,
            "radiador radiator aquecedor aquecimento calefacao heater heating hvac termo painel panel agua water hidronico",
        ),
        15.0,
    ),
    raised(
        item(
            "radiator-120",
            "Radiador de painel 120×10×50",
            C::Living,
            [120.0, 10.0, 50.0],
            WHITE,
            Model::Box,
            "radiador radiator aquecedor aquecimento calefacao heater heating hvac termo painel panel agua water hidronico",
        ),
        15.0,
    ),
    raised(
        item(
            "convector-100",
            "Convector elétrico 100×10×45",
            C::Living,
            [100.0, 10.0, 45.0],
            WHITE,
            Model::Box,
            "radiador radiator aquecedor aquecimento calefacao heater heating hvac termo convector convetor eletrico electric",
        ),
        15.0,
    ),
    lit(
        item(
            "floor-lamp",
            "Luminária de piso",
            C::Lighting,
            [40.0, 40.0, 165.0],
            [230, 220, 200],
            Model::Lamp,
            "lamp light abajur luminaria coluna",
        ),
        led(1055.0, 11.0, 2700.0, 0.0, 0.84),
    ),
    lit(
        item(
            "table-lamp",
            "Abajur de mesa",
            C::Lighting,
            [30.0, 30.0, 50.0],
            [235, 225, 205],
            Model::Lamp,
            "abajur lamp table luminaria mesa cabeceira",
        ),
        led(470.0, 5.0, 2700.0, 0.0, 0.82),
    ),
    lit(
        raised(
            item(
                "downlight",
                "Spot embutido LED 7 W",
                C::Lighting,
                [10.0, 10.0, 3.0],
                [245, 245, 242],
                Model::Box,
                "spot embutido downlight led teto sanca luminaria recessed",
            ),
            259.4,
        ),
        led(630.0, 7.0, 3000.0, 60.0, 0.0),
    ),
    lit(
        raised(
            item(
                "pendant",
                "Pendente",
                C::Lighting,
                [35.0, 35.0, 90.0],
                [40, 40, 42],
                Model::Box,
                "pendente pendant luminaria bancada jantar lustre",
            ),
            170.0,
        ),
        led(806.0, 9.0, 2700.0, 110.0, 0.12),
    ),
    lit(
        raised(
            item(
                "led-panel",
                "Painel LED 60×60 36 W",
                C::Lighting,
                [62.0, 62.0, 2.0],
                [240, 240, 238],
                Model::Box,
                "painel led plafon panel luminaria teto escritorio",
            ),
            258.0,
        ),
        LightPreset {
            lumens: 3600.0,
            watts: 36.0,
            kelvin: 4000.0,
            beam: 0.0,
            panel: true,
            per_meter: false,
            z: 0.0,
        },
    ),
    lit(
        raised(
            item(
                "led-strip",
                "Fita LED (perfil)",
                C::Lighting,
                [100.0, 2.0, 1.0],
                [200, 202, 206],
                Model::Box,
                "fita led strip perfil sob armario sanca iluminacao indireta",
            ),
            140.0,
        ),
        LightPreset {
            lumens: 900.0,
            watts: 9.0,
            kelvin: 3000.0,
            beam: 0.0,
            panel: true,
            per_meter: true,
            z: 0.0,
        },
    ),
    // Dining
    item(
        "dining-table-4",
        "Mesa de jantar 4 lugares",
        C::Dining,
        [120.0, 80.0, 75.0],
        WOOD,
        Model::Table { round: false },
        "dining table mesa",
    ),
    item(
        "dining-table-6",
        "Mesa de jantar 6 lugares",
        C::Dining,
        [180.0, 90.0, 75.0],
        WOOD,
        Model::Table { round: false },
        "dining table mesa",
    ),
    item(
        "dining-set-4",
        "Mesa com 4 cadeiras",
        C::Dining,
        [200.0, 180.0, 90.0],
        WOOD,
        Model::DiningSet { chairs: 4 },
        "dining set mesa cadeiras jantar conjunto",
    ),
    item(
        "dining-set-6",
        "Mesa com 6 cadeiras",
        C::Dining,
        [260.0, 180.0, 90.0],
        WOOD,
        Model::DiningSet { chairs: 6 },
        "dining set mesa 6 cadeiras jantar conjunto",
    ),
    item(
        "round-table",
        "Mesa redonda",
        C::Dining,
        [110.0, 110.0, 75.0],
        WOOD,
        Model::Table { round: true },
        "round table mesa",
    ),
    item(
        "chair",
        "Cadeira",
        C::Dining,
        [45.0, 50.0, 90.0],
        WOOD,
        Model::Chair,
        "chair cadeira",
    ),
    item(
        "sideboard",
        "Aparador",
        C::Dining,
        [160.0, 45.0, 80.0],
        WOOD,
        Model::Cabinet {
            doors: 4,
            drawers: 0,
        },
        "sideboard buffet aparador",
    ),
    // Kitchen
    item(
        "fridge",
        "Geladeira",
        C::Kitchen,
        [70.0, 72.0, 180.0],
        STEEL,
        Model::Fridge,
        "fridge refrigerator geladeira",
    ),
    item(
        "stove",
        "Fogão 4 bocas",
        C::Kitchen,
        [60.0, 62.0, 90.0],
        STEEL,
        Model::Stove,
        "stove cooker oven fogao",
    ),
    item(
        "sink-counter",
        "Bancada com pia",
        C::Kitchen,
        [120.0, 60.0, 90.0],
        WHITE,
        Model::SinkCounter,
        "sink counter pia cuba",
    ),
    item(
        "base-cabinet",
        "Armário de cozinha (baixo)",
        C::Kitchen,
        [60.0, 60.0, 90.0],
        WHITE,
        Model::Counter,
        "base cabinet balcao armario",
    ),
    raised(
        item(
            "wall-cabinet",
            "Armário aéreo",
            C::Kitchen,
            [60.0, 35.0, 70.0],
            WHITE,
            Model::WallCabinet,
            "wall cabinet aereo armario",
        ),
        150.0,
    ),
    raised(
        item(
            "hood",
            "Coifa",
            C::Kitchen,
            [60.0, 50.0, 60.0],
            STEEL,
            Model::Hood,
            "hood coifa exaustor depurador extractor",
        ),
        150.0,
    ),
    item(
        "dishwasher",
        "Lava-louças",
        C::Kitchen,
        [60.0, 60.0, 85.0],
        STEEL,
        Model::Appliance { round_door: false },
        "dishwasher lava louças",
    ),
    raised(
        item(
            "microwave",
            "Micro-ondas",
            C::Kitchen,
            [50.0, 40.0, 30.0],
            DARK,
            Model::Microwave,
            "microwave micro ondas",
        ),
        90.0,
    ),
    raised(
        item(
            "cooktop",
            "Cooktop 4 bocas (embutir)",
            C::Kitchen,
            [60.0, 50.0, 6.0],
            DARK,
            Model::Cooktop,
            "cooktop cook top embutir fogao vidro",
        ),
        90.0,
    ),
    raised(
        item(
            "sink-bowl",
            "Cuba de inox (embutir)",
            C::Kitchen,
            [50.0, 40.0, 18.0],
            STEEL,
            Model::SinkBowl,
            "sink bowl cuba pia inox embutir",
        ),
        72.0,
    ),
    raised(
        item(
            "oven",
            "Forno de embutir",
            C::Kitchen,
            [60.0, 55.0, 60.0],
            DARK,
            Model::Oven,
            "oven forno embutir eletrico",
        ),
        80.0,
    ),
    item(
        "kitchen-island",
        "Ilha de cozinha",
        C::Kitchen,
        [180.0, 90.0, 90.0],
        WHITE,
        Model::Counter,
        "island ilha bancada",
    ),
    item(
        "stool",
        "Banqueta alta",
        C::Kitchen,
        [40.0, 40.0, 75.0],
        DARK,
        Model::Stool,
        "stool banqueta",
    ),
    // Bedroom
    item(
        "bed-double",
        "Cama de casal",
        C::Bedroom,
        [158.0, 208.0, 55.0],
        [214, 206, 190],
        Model::Bed { pillows: 2 },
        "double bed cama casal",
    ),
    item(
        "bed-queen",
        "Cama queen",
        C::Bedroom,
        [168.0, 208.0, 55.0],
        [214, 206, 190],
        Model::Bed { pillows: 2 },
        "queen bed cama",
    ),
    item(
        "bed-king",
        "Cama king",
        C::Bedroom,
        [203.0, 213.0, 55.0],
        [214, 206, 190],
        Model::Bed { pillows: 2 },
        "king bed cama",
    ),
    item(
        "bed-single",
        "Cama de solteiro",
        C::Bedroom,
        [98.0, 198.0, 50.0],
        [200, 212, 224],
        Model::Bed { pillows: 1 },
        "single bed cama solteiro",
    ),
    item(
        "nightstand",
        "Criado-mudo",
        C::Bedroom,
        [50.0, 40.0, 55.0],
        WOOD,
        Model::Cabinet {
            doors: 0,
            drawers: 2,
        },
        "nightstand bedside criado mudo",
    ),
    item(
        "wardrobe",
        "Guarda-roupa",
        C::Bedroom,
        [180.0, 60.0, 220.0],
        WHITE,
        Model::Cabinet {
            doors: 3,
            drawers: 0,
        },
        "wardrobe closet guarda roupa armario",
    ),
    item(
        "dresser",
        "Cômoda",
        C::Bedroom,
        [100.0, 50.0, 85.0],
        WOOD,
        Model::Cabinet {
            doors: 0,
            drawers: 4,
        },
        "dresser chest comoda gaveteiro",
    ),
    item(
        "crib",
        "Berço",
        C::Bedroom,
        [70.0, 130.0, 100.0],
        WHITE,
        Model::Crib,
        "crib baby berco",
    ),
    // Bathroom
    item(
        "toilet",
        "Vaso sanitário",
        C::Bathroom,
        [40.0, 68.0, 78.0],
        WHITE,
        Model::Toilet,
        "toilet wc vaso privada",
    ),
    item(
        "basin-cabinet",
        "Gabinete com lavatório",
        C::Bathroom,
        [80.0, 48.0, 85.0],
        WHITE,
        Model::Basin,
        "basin sink lavatorio pia gabinete",
    ),
    item(
        "shower",
        "Box de chuveiro",
        C::Bathroom,
        [90.0, 90.0, 200.0],
        [170, 205, 225],
        Model::Shower,
        "shower box chuveiro alvenaria",
    ),
    item(
        "shower-glass",
        "Box de vidro",
        C::Bathroom,
        [120.0, 2.0, 190.0],
        [168, 206, 226],
        Model::GlassPanel,
        "shower glass box vidro chuveiro vidro temperado",
    ),
    item(
        "bathtub",
        "Banheira",
        C::Bathroom,
        [170.0, 75.0, 55.0],
        WHITE,
        Model::Bathtub,
        "bathtub banheira",
    ),
    // Laundry
    item(
        "washer",
        "Máquina de lavar",
        C::Laundry,
        [60.0, 65.0, 85.0],
        WHITE,
        Model::Appliance { round_door: true },
        "washing machine washer lavar roupa",
    ),
    item(
        "dryer",
        "Secadora",
        C::Laundry,
        [60.0, 65.0, 85.0],
        WHITE,
        Model::Appliance { round_door: true },
        "dryer secadora",
    ),
    item(
        "laundry-sink",
        "Tanque",
        C::Laundry,
        [60.0, 55.0, 85.0],
        WHITE,
        Model::SinkCounter,
        "laundry sink tanque",
    ),
    // Office
    item(
        "desk",
        "Escrivaninha",
        C::Office,
        [120.0, 60.0, 75.0],
        WOOD,
        Model::Desk,
        "desk escrivaninha mesa escritorio",
    ),
    item(
        "office-chair",
        "Cadeira de escritório",
        C::Office,
        [62.0, 62.0, 105.0],
        DARK,
        Model::OfficeChair,
        "office chair cadeira escritorio",
    ),
    // Doors and windows
    opening(
        item(
            "door",
            "Porta",
            C::DoorsWindows,
            [80.0, 15.0, 210.0],
            WOOD,
            Model::Door,
            "door porta",
        ),
        OpeningKind::Door,
        1,
        false,
    ),
    opening(
        item(
            "door-double",
            "Porta dupla",
            C::DoorsWindows,
            [140.0, 15.0, 210.0],
            WOOD,
            Model::Door,
            "double door porta dupla",
        ),
        OpeningKind::Door,
        2,
        false,
    ),
    opening(
        item(
            "door-sliding",
            "Porta de correr",
            C::DoorsWindows,
            [160.0, 15.0, 210.0],
            [220, 230, 235],
            Model::Door,
            "sliding door porta correr",
        ),
        OpeningKind::Door,
        2,
        true,
    ),
    opening(
        item(
            "garage-door",
            "Portão de garagem",
            C::DoorsWindows,
            [300.0, 15.0, 230.0],
            [210, 212, 214],
            Model::GarageDoor,
            "garage door portao garagem basculante seccional",
        ),
        OpeningKind::Door,
        1,
        true,
    ),
    opening(
        item(
            "passage",
            "Vão livre",
            C::DoorsWindows,
            [90.0, 15.0, 210.0],
            WHITE,
            Model::Passage,
            "passage opening vao passagem",
        ),
        OpeningKind::Passage,
        1,
        false,
    ),
    opening(
        raised(
            item(
                "window",
                "Janela",
                C::DoorsWindows,
                [120.0, 15.0, 110.0],
                WHITE,
                Model::Window { panes: 2 },
                "window janela",
            ),
            100.0,
        ),
        OpeningKind::Window,
        2,
        true,
    ),
    opening(
        raised(
            item(
                "window-small",
                "Janela basculante",
                C::DoorsWindows,
                [60.0, 15.0, 60.0],
                WHITE,
                Model::Window { panes: 1 },
                "small window basculante janela banheiro",
            ),
            150.0,
        ),
        OpeningKind::Window,
        1,
        true,
    ),
    opening(
        item(
            "french-window",
            "Porta-janela",
            C::DoorsWindows,
            [200.0, 15.0, 220.0],
            WHITE,
            Model::Window { panes: 4 },
            "french window porta janela sacada",
        ),
        OpeningKind::Window,
        2,
        true,
    ),
    // Structure
    item(
        "stairs",
        "Escada reta",
        C::Structure,
        [90.0, 300.0, 280.0],
        WOOD,
        Model::Stairs,
        "stairs staircase escada",
    ),
    item(
        "column",
        "Pilar",
        C::Structure,
        [30.0, 30.0, 250.0],
        WHITE,
        Model::Column { round: false },
        "column pillar pilar coluna",
    ),
    item(
        "column-round",
        "Coluna redonda",
        C::Structure,
        [30.0, 30.0, 250.0],
        WHITE,
        Model::Column { round: true },
        "round column coluna",
    ),
    item(
        "footing",
        "Sapata",
        C::Structure,
        [60.0, 60.0, 50.0],
        [170, 170, 165],
        Model::Footing,
        "footing sapata fundacao bloco",
    ),
    item(
        "beam",
        "Viga / caibro",
        C::Structure,
        [10.0, 300.0, 20.0],
        WOOD,
        Model::Box,
        "beam rafter viga caibro terca barrote",
    ),
    item(
        "panel",
        "Painel",
        C::Structure,
        [120.0, 2.0, 240.0],
        [214, 206, 190],
        Model::Box,
        "panel painel chapa placa osb compensado",
    ),
    item(
        "roof-sheet",
        "Telha ondulada",
        C::Structure,
        [110.0, 244.0, 5.0],
        [150, 156, 160],
        Model::Corrugated,
        "roof sheet telha ondulada fibrocimento metalica",
    ),
    item(
        "railing",
        "Gradil de ferro (guarda-corpo de barras verticais)",
        C::Structure,
        [300.0, 5.0, 110.0],
        STEEL,
        Model::Railing,
        "gradil grade ferro aco aluminio guarda corpo barras corrimao parapeito sacada varanda railing",
    ),
    item(
        "glass-railing",
        "Guarda-corpo de vidro laminado com corrimão metálico",
        C::Structure,
        [300.0, 5.0, 110.0],
        [168, 206, 226],
        Model::GlassRailing,
        "guarda corpo vidro laminado corrimao perfil aluminio sacada varanda glass railing",
    ),
    item(
        "balcony-glazing",
        "Fechamento de vidro da sacada (envidraçamento retrátil)",
        C::Structure,
        [300.0, 3.0, 240.0],
        [190, 220, 232],
        Model::BalconyGlazing,
        "fechamento vidro sacada envidracamento retratil cortina de vidro varanda reiki",
    ),
    item(
        "box",
        "Caixa",
        C::Structure,
        [50.0, 50.0, 50.0],
        [200, 190, 170],
        Model::Box,
        "box caixa volume",
    ),
    // Decor and outdoor
    item(
        "plant",
        "Planta em vaso",
        C::Decor,
        [50.0, 50.0, 110.0],
        [70, 140, 80],
        Model::Plant,
        "plant vaso planta",
    ),
    item(
        "planter",
        "Floreira",
        C::Decor,
        [100.0, 40.0, 60.0],
        [150, 110, 80],
        Model::Planter,
        "planter floreira jardineira canteiro vaso",
    ),
    item(
        "fence",
        "Muro / cerca",
        C::Outdoor,
        [300.0, 10.0, 180.0],
        WOOD,
        Model::Fence,
        "fence cerca muro gradil",
    ),
    item(
        "pool",
        "Piscina",
        C::Outdoor,
        [600.0, 300.0, 20.0],
        [70, 165, 210],
        Model::Pool { oval: false },
        "pool piscina",
    ),
    item(
        "pool-oval",
        "Piscina oval",
        C::Outdoor,
        [600.0, 300.0, 20.0],
        [70, 165, 210],
        Model::Pool { oval: true },
        "pool oval piscina oval redonda feijao",
    ),
    item(
        "lounger",
        "Espreguiçadeira",
        C::Outdoor,
        [65.0, 190.0, 80.0],
        WHITE,
        Model::Lounger,
        "lounger espreguicadeira chaise",
    ),
    item(
        "grill",
        "Churrasqueira",
        C::Outdoor,
        [120.0, 60.0, 220.0],
        [180, 90, 60],
        Model::Grill,
        "grill barbecue churrasqueira",
    ),
    item(
        "bench",
        "Banco",
        C::Outdoor,
        [150.0, 40.0, 45.0],
        WOOD,
        Model::Bench,
        "bench banco",
    ),
    item(
        "tree",
        "Árvore",
        C::Outdoor,
        [300.0, 300.0, 500.0],
        [80, 140, 70],
        Model::Tree,
        "tree arvore",
    ),
    item(
        "car",
        "Carro",
        C::Outdoor,
        [450.0, 180.0, 150.0],
        [70, 110, 170],
        Model::Car,
        "car automovel carro garagem",
    ),
    // --- Elétrica -----------------------------------------------------------
    raised(
        item(
            "outlet-low",
            "Tomada baixa (30 cm)",
            C::Electrical,
            [10.0, 4.0, 10.0],
            ELECTRIC,
            Model::Point(PointSymbol::OutletLow),
            "tomada baixa outlet socket plug",
        ),
        25.0,
    ),
    raised(
        item(
            "outlet-mid",
            "Tomada média (1,10 m)",
            C::Electrical,
            [10.0, 4.0, 10.0],
            ELECTRIC,
            Model::Point(PointSymbol::OutletMid),
            "tomada media bancada outlet counter",
        ),
        105.0,
    ),
    raised(
        item(
            "outlet-high",
            "Tomada alta (2,20 m)",
            C::Electrical,
            [10.0, 4.0, 10.0],
            ELECTRIC,
            Model::Point(PointSymbol::OutletHigh),
            "tomada alta outlet high",
        ),
        215.0,
    ),
    raised(
        item(
            "switch",
            "Interruptor simples",
            C::Electrical,
            [8.0, 4.0, 12.0],
            ELECTRIC,
            Model::Point(PointSymbol::Switch1),
            "interruptor simples switch light",
        ),
        104.0,
    ),
    raised(
        item(
            "switch-double",
            "Interruptor duplo",
            C::Electrical,
            [8.0, 4.0, 12.0],
            ELECTRIC,
            Model::Point(PointSymbol::Switch2),
            "interruptor duplo two switches",
        ),
        104.0,
    ),
    raised(
        item(
            "switch-3way",
            "Interruptor paralelo (three-way)",
            C::Electrical,
            [8.0, 4.0, 12.0],
            ELECTRIC,
            Model::Point(PointSymbol::Switch3Way),
            "interruptor paralelo three way hotel",
        ),
        104.0,
    ),
    raised(
        item(
            "light-ceiling",
            "Ponto de luz no teto",
            C::Electrical,
            [20.0, 20.0, 6.0],
            [250, 245, 220],
            Model::Point(PointSymbol::LightCeiling),
            "ponto luz teto lampada plafon ceiling light",
        ),
        244.0,
    )
    .with_light(led(1000.0, 10.0, 3000.0, 0.0, 0.0)),
    raised(
        item(
            "light-wall",
            "Arandela (ponto de luz na parede)",
            C::Electrical,
            [20.0, 10.0, 15.0],
            [250, 245, 220],
            Model::Point(PointSymbol::LightWall),
            "arandela luz parede wall light sconce",
        ),
        180.0,
    )
    .with_light(led(400.0, 4.5, 2700.0, 0.0, 0.5)),
    raised(
        item(
            "electrical-panel",
            "Quadro de distribuição",
            C::Electrical,
            [40.0, 10.0, 60.0],
            [210, 210, 205],
            Model::Point(PointSymbol::Panel),
            "quadro distribuicao disjuntores panel breaker qdc",
        ),
        120.0,
    ),
    raised(
        item(
            "ac-point",
            "Ponto de ar-condicionado",
            C::Electrical,
            [15.0, 6.0, 15.0],
            ELECTRIC,
            Model::Point(PointSymbol::AirConditioner),
            "ar condicionado split tomada ac air conditioner",
        ),
        220.0,
    ),
    raised(
        item(
            "shower-point",
            "Ponto para chuveiro elétrico",
            C::Electrical,
            [12.0, 6.0, 12.0],
            ELECTRIC,
            Model::Point(PointSymbol::ShowerPoint),
            "chuveiro eletrico ponto shower",
        ),
        210.0,
    ),
    raised(
        item(
            "data-outlet",
            "Tomada de dados / TV",
            C::Electrical,
            [10.0, 4.0, 10.0],
            [90, 90, 100],
            Model::Point(PointSymbol::DataOutlet),
            "tomada rede dados internet tv antena rj45 data",
        ),
        30.0,
    ),
    // --- Telecomunicações (NBR 14565, cabeamento estruturado) ----------------
    raised(
        item(
            "network-outlet",
            "Ponto de rede RJ45 (Cat 6)",
            C::Electrical,
            [10.0, 4.0, 10.0],
            [60, 120, 70],
            Model::Point(PointSymbol::NetworkOutlet),
            "ponto rede internet dados rj45 cat6 cabo lan ethernet network",
        ),
        30.0,
    ),
    raised(
        item(
            "tv-outlet",
            "Ponto de TV (coaxial)",
            C::Electrical,
            [10.0, 4.0, 10.0],
            [60, 120, 70],
            Model::Point(PointSymbol::TvOutlet),
            "ponto tv antena coaxial cabo televisao tv",
        ),
        30.0,
    ),
    raised(
        item(
            "wifi-point",
            "Ponto de Wi-Fi no teto (access point)",
            C::Electrical,
            [16.0, 16.0, 4.0],
            [60, 120, 70],
            Model::Point(PointSymbol::WifiPoint),
            "wifi wi-fi access point roteador teto internet sem fio",
        ),
        250.0,
    ),
    raised(
        item(
            "telecom-panel",
            "Quadro de telecomunicações (rack / DG)",
            C::Electrical,
            [40.0, 12.0, 50.0],
            [60, 120, 70],
            Model::Point(PointSymbol::TelecomPanel),
            "quadro telecom rack dg distribuidor geral rede internet switch roteador modem",
        ),
        150.0,
    ),
    raised(
        item(
            "doorbell",
            "Campainha",
            C::Electrical,
            [8.0, 4.0, 8.0],
            ELECTRIC,
            Model::Point(PointSymbol::Doorbell),
            "campainha doorbell",
        ),
        140.0,
    ),
    // --- Tomadas embutidas em móveis -----------------------------------------
    raised(
        item(
            "outlet-tower",
            "Torre de tomadas retrátil de embutir (furo 60 mm, 3 tomadas, plugue)",
            C::Electrical,
            [8.4, 8.4, 19.0],
            [60, 60, 64],
            Model::OutletTower,
            "torre tomada retratil embutir bancada ilha pia multiplug elevor renna bmax",
        ),
        90.0,
    ),
    raised(
        item(
            "outlet-tower-auto",
            "Torre de tomadas automática com indução (furo 85 mm, ligada à instalação)",
            C::Electrical,
            [10.0, 10.0, 20.0],
            [60, 60, 64],
            Model::OutletTower,
            "torre tomada automatica inducao carregador embutir bancada caixatomada tr03aw",
        ),
        90.0,
    ),
    raised(
        item(
            "outlet-tower-4",
            "Torre de 4 tomadas + USB e indução (furo 100 mm)",
            C::Electrical,
            [11.0, 11.0, 22.0],
            [40, 40, 44],
            Model::OutletTower,
            "torre 4 tomadas usb inducao powerdock hafele h5 embutir bancada",
        ),
        90.0,
    ),
    raised(
        item(
            "desk-outlet-box",
            "Caixa de tomadas de mesa (4 tomadas, RJ45, USB, HDMI; recorte 120×335 mm)",
            C::Electrical,
            [34.4, 13.6, 1.5],
            [150, 152, 156],
            Model::DeskBox,
            "caixa tomada mesa reuniao escritorio embutir rj45 hdmi usb cx44",
        ),
        75.0,
    ),
    raised(
        item(
            "furniture-outlet",
            "Tomada de embutir em móvel (furo 35 mm)",
            C::Electrical,
            [4.5, 2.0, 4.5],
            ELECTRIC,
            Model::Point(PointSymbol::OutletMid),
            "tomada embutir movel marcenaria painel gabinete furo 35 mm",
        ),
        100.0,
    ),
    // --- Automação ----------------------------------------------------------
    raised(
        item(
            "smart-relay",
            "Relé de automação (atrás do interruptor ou da luminária)",
            C::Electrical,
            [5.0, 5.0, 2.0],
            ELECTRIC,
            Model::Point(PointSymbol::SmartRelay),
            "rele automacao sonoff shelly modulo wi-fi zigbee smart relay",
        ),
        104.0,
    ),
    raised(
        item(
            "smart-switch",
            "Interruptor inteligente",
            C::Electrical,
            [8.0, 4.0, 12.0],
            ELECTRIC,
            Model::Point(PointSymbol::SmartSwitch),
            "interruptor inteligente touch wi-fi zigbee automacao smart switch",
        ),
        104.0,
    ),
    raised(
        item(
            "dimmer",
            "Dimmer",
            C::Electrical,
            [8.0, 4.0, 12.0],
            ELECTRIC,
            Model::Point(PointSymbol::Dimmer),
            "dimmer variador luminosidade automacao",
        ),
        104.0,
    ),
    raised(
        item(
            "presence-sensor",
            "Sensor de presença de teto",
            C::Electrical,
            [10.0, 10.0, 4.0],
            ELECTRIC,
            Model::Point(PointSymbol::PresenceSensor),
            "sensor presenca movimento teto automacao iluminacao",
        ),
        270.0,
    ),
    raised(
        item(
            "smart-lock",
            "Fechadura eletrônica",
            C::Electrical,
            [8.0, 4.0, 30.0],
            ELECTRIC,
            Model::Point(PointSymbol::SmartLock),
            "fechadura eletronica digital biometria senha automacao smart lock",
        ),
        90.0,
    ),
    // --- Hidráulica ---------------------------------------------------------
    raised(
        item(
            "cold-water",
            "Ponto de água fria",
            C::Plumbing,
            [8.0, 8.0, 8.0],
            [40, 110, 210],
            Model::Point(PointSymbol::ColdWater),
            "agua fria ponto af cold water",
        ),
        60.0,
    ),
    raised(
        item(
            "hot-water",
            "Ponto de água quente",
            C::Plumbing,
            [8.0, 8.0, 8.0],
            [210, 60, 40],
            Model::Point(PointSymbol::HotWater),
            "agua quente ponto aq hot water",
        ),
        60.0,
    ),
    item(
        "sewer",
        "Ponto de esgoto",
        C::Plumbing,
        [10.0, 10.0, 5.0],
        [120, 90, 60],
        Model::Point(PointSymbol::Sewer),
        "esgoto ponto saida sewer drain waste",
    ),
    item(
        "floor-drain",
        "Caixa sifonada 150×150×50 (ralo)",
        C::Plumbing,
        [15.0, 15.0, 2.0],
        [170, 170, 170],
        Model::Point(PointSymbol::FloorDrain),
        "ralo caixa sifonada 150 grelha piso floor drain",
    ),
    item(
        "floor-drain-100",
        "Caixa sifonada 100×150×50 (ralo)",
        C::Plumbing,
        [10.0, 10.0, 2.0],
        [170, 170, 170],
        Model::Point(PointSymbol::FloorDrain),
        "ralo caixa sifonada 100 pequena grelha piso",
    ),
    item(
        "floor-drain-75",
        "Caixa sifonada 150×185×75 (ralo)",
        C::Plumbing,
        [15.0, 15.0, 2.0],
        [170, 170, 170],
        Model::Point(PointSymbol::FloorDrain),
        "ralo caixa sifonada 150 185 saida 75 grande",
    ),
    item(
        "trap-drain-small",
        "Ralo sifonado 100 mm (fecho hídrico curto)",
        C::Plumbing,
        [10.0, 10.0, 2.0],
        [170, 170, 170],
        Model::Point(PointSymbol::FloorDrain),
        "ralo sifonado pequeno 100 redondo quadrado",
    ),
    item(
        "dry-drain",
        "Ralo seco 100 mm",
        C::Plumbing,
        [10.0, 10.0, 2.0],
        [170, 170, 170],
        Model::Point(PointSymbol::DryDrain),
        "ralo seco caixa seca 100 sem sifao",
    ),
    item(
        "linear-drain",
        "Ralo linear 70 cm (sem sifão)",
        C::Plumbing,
        [70.0, 7.2, 4.2],
        [170, 170, 170],
        Model::Point(PointSymbol::LinearDrain),
        "ralo linear oculto invisivel grelha inox box 50 70 90",
    ),
    item(
        "linear-drain-trap",
        "Ralo linear sifonado 70 cm",
        C::Plumbing,
        [70.0, 7.2, 4.2],
        [170, 170, 170],
        Model::Point(PointSymbol::LinearDrain),
        "ralo linear sifonado fecho hidrico box 50 75",
    ),
    item(
        "rain-drain",
        "Ralo de águas pluviais (varanda descoberta, terraço)",
        C::Plumbing,
        [10.0, 10.0, 2.0],
        [120, 160, 190],
        Model::Point(PointSymbol::RainDrain),
        "ralo pluvial chuva varanda terraco area externa calha",
    ),
    raised(
        item(
            "valve",
            "Registro",
            C::Plumbing,
            [8.0, 8.0, 8.0],
            [180, 150, 60],
            Model::Point(PointSymbol::Valve),
            "registro gaveta pressao valve",
        ),
        110.0,
    ),
    item(
        "grease-trap",
        "Caixa de gordura",
        C::Plumbing,
        [40.0, 40.0, 40.0],
        [150, 150, 145],
        Model::Point(PointSymbol::GreaseTrap),
        "caixa gordura grease trap",
    ),
    raised(
        item(
            "vent-grille",
            "Grelha de ventilação permanente (gás)",
            C::Plumbing,
            [20.0, 4.0, 15.0],
            [200, 200, 195],
            Model::Point(PointSymbol::VentGrille),
            "grelha veneziana ventilacao permanente abertura gas ar vent grille louver",
        ),
        20.0,
    ),
    item(
        "vent-pipe",
        "Tubo de ventilação",
        C::Plumbing,
        [10.0, 10.0, 10.0],
        [120, 90, 60],
        Model::Point(PointSymbol::VentPipe),
        "tubo ventilacao ventilador coluna respiro esgoto vent",
    ),
    item(
        "inspection-box",
        "Caixa de inspeção",
        C::Plumbing,
        [60.0, 60.0, 60.0],
        [150, 150, 145],
        Model::Point(PointSymbol::InspectionBox),
        "caixa inspecao passagem inspection",
    ),
    raised(
        item(
            "water-meter",
            "Hidrômetro",
            C::Plumbing,
            [20.0, 15.0, 15.0],
            [60, 60, 70],
            Model::Point(PointSymbol::WaterMeter),
            "hidrometro medidor agua water meter",
        ),
        80.0,
    ),
    raised(
        item(
            "gas-point",
            "Ponto de gás",
            C::Plumbing,
            [8.0, 8.0, 8.0],
            [220, 170, 30],
            Model::Point(PointSymbol::Gas),
            "gas ponto fogao cooktop glp",
        ),
        60.0,
    ),
];

pub fn find(id: &str) -> Option<&'static CatalogItem> {
    CATALOG.iter().find(|item| item.id == id)
}

/// Accent-insensitive search over ids, names and keywords. An empty query
/// returns everything in catalog order.
pub fn search(query: &str) -> Vec<&'static CatalogItem> {
    let words: Vec<String> = fold(query).split_whitespace().map(str::to_owned).collect();
    // Items matching any word, the ones matching more words first.
    let mut scored: Vec<(usize, usize, &'static CatalogItem)> = CATALOG
        .iter()
        .enumerate()
        .filter_map(|(order, item)| {
            let haystack = fold(&format!(
                "{} {} {} {}",
                item.id,
                item.name,
                item.keywords,
                item.category.name()
            ));
            let hits = words
                .iter()
                .filter(|w| haystack.contains(w.as_str()))
                .count();
            (hits > 0 || words.is_empty()).then_some((hits, order, item))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, _, item)| item).collect()
}

/// Text the way the search compares it: lower case, without accents, so
/// `sofá` and `sofa` are the same word.
pub fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'ê' | 'è' => 'e',
            'í' | 'î' => 'i',
            'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

impl CatalogItem {
    /// Which side is its front, what that front and its back are, and
    /// whether the back belongs on a wall. Every model is built facing its
    /// local `+y`: at `angle` 0 the front looks down the plan.
    pub fn front(&self) -> newera_core::front::Front {
        newera_core::front::of(self.id)
    }

    /// A new piece of this item, at its default size, centered at `position`.
    pub fn instantiate(
        &self,
        id: newera_core::FurnitureId,
        position: newera_core::Point2,
    ) -> Furniture {
        let mut piece = Furniture {
            id,
            catalog: self.id.to_owned(),
            name: self.name.to_owned(),
            position,
            elevation: self.elevation,
            angle: 0.0,
            width: self.size[0],
            depth: self.size[1],
            height: self.size[2],
            mirrored: false,
            color: None,
            opening: self.opening.map(OpeningPreset::to_opening),
            model: None,
            visible: true,
            level: None,
            discipline: self.category.discipline(),
            opacity: match self.model {
                Model::GlassPanel | Model::GlassRailing => Some(0.35),
                Model::BalconyGlazing => Some(0.25),
                _ => None,
            },
            ..Default::default()
        };
        bundled::configure(&mut piece);
        piece.light = light_for(&piece);
        piece
    }
}

/// A whole, non-negative repeat count from an already rounded float.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn count(value: f64) -> u32 {
    value.max(0.0).min(f64::from(u32::MAX)) as u32
}

/// 3D mesh of a piece in its local frame (see [`mesh`] module docs), at its
/// exact size. Unknown catalog ids render as a plain box.
pub fn piece_mesh(piece: &Furniture) -> Mesh {
    if let Some(shape) = &piece.shape {
        let color = piece.color.unwrap_or([200, 190, 170]);
        return models::solid(shape, piece, rgb(color));
    }
    if let Some(mesh) = bundled::mesh(piece) {
        return mesh;
    }
    let item = find(&piece.catalog);
    if let Some(mesh) = fixtures::build(piece) {
        return mesh;
    }
    let model = item.map_or(Model::Box, |i| i.model);
    let color = piece
        .color
        .or(item.map(|i| i.color))
        .unwrap_or([200, 190, 170]);
    models::build(model, piece, rgb(color))
}

#[cfg(test)]
mod tests {
    use newera_core::{FurnitureId, Point2};

    use super::*;

    #[test]
    fn a_radiator_is_found_in_either_language() {
        for q in [
            "radiator",
            "radiador",
            "aquecedor",
            "heater convector",
            "hvac",
        ] {
            let first = search(q)[0];
            assert!(
                first.id.starts_with("radiator") || first.id.starts_with("convector"),
                "{q} found {}",
                first.id
            );
        }
        let widths: Vec<f64> = ["radiator-60", "radiator-100", "radiator-120"]
            .iter()
            .map(|id| find(id).unwrap().size[0])
            .collect();
        assert_eq!(widths, [60.0, 100.0, 120.0]);
    }

    #[test]
    fn a_resized_flight_keeps_its_steps_a_step_high() {
        assert_eq!(stair_risers(280.0), 16, "the catalog flight is unchanged");
        assert_eq!(stair_risers(131.0), 8);
        for h in [36.0, 90.0, 131.0, 262.0, 300.0, 450.0, 6000.0] {
            let rise = h / f64::from(stair_risers(h));
            assert!(rise <= 18.0 + 1e-9, "{h}: {rise}");
        }
        // The plan draws the same steps: a line between each two treads.
        let shapes = |height: f64| {
            let mut flight = find("stairs")
                .unwrap()
                .instantiate(FurnitureId(1), Point2::new(0.0, 0.0));
            flight.width = 110.0;
            flight.depth = 224.0;
            flight.height = height;
            crate::symbols::plan_symbol(&flight).len()
        };
        assert_eq!(shapes(131.0) - shapes(36.0), 8 - 2);
    }

    #[test]
    fn explicit_colors_recolor_every_primary_surface_and_preserve_fixed_materials() {
        for item in CATALOG {
            let mut piece = item.instantiate(FurnitureId(1), Point2::new(0.0, 0.0));
            let original = piece_mesh(&piece);
            piece.color = Some([13, 77, 151]);
            let recolored = piece_mesh(&piece);
            assert_eq!(
                original.positions, recolored.positions,
                "{} geometry changed",
                item.id
            );
            assert_eq!(original.finishable, recolored.finishable);
            for (i, (before, after)) in original.colors.iter().zip(&recolored.colors).enumerate() {
                let same = before.iter().zip(after).all(|(a, b)| (a - b).abs() < 1e-6);
                if original.finishable.get(i).copied().unwrap_or(true) {
                    assert!(
                        !same,
                        "{} ignores the explicit body color at vertex {i}",
                        item.id
                    );
                } else {
                    assert!(same, "{} recolored a fixed material at vertex {i}", item.id);
                }
            }
            piece.color = None;
            assert_eq!(
                piece_mesh(&piece),
                original,
                "{} default appearance changed",
                item.id
            );
        }
    }

    /// Every item says which way it faces: either `newera_core::front` names
    /// its front, or it is listed here as having none. A new item that is in
    /// neither fails, so nobody adds a sofa an agent will turn to the wall.
    #[test]
    fn every_item_is_marked_with_its_front_or_as_having_none() {
        const NO_FRONT: &[&str] = &[
            "coffee-table",
            "side-table",
            "rug",
            "floor-lamp",
            "table-lamp",
            "downlight",
            "pendant",
            "led-panel",
            "led-strip",
            "dining-table-4",
            "dining-table-6",
            "dining-set-4",
            "dining-set-6",
            "round-table",
            "cooktop",
            "sink-bowl",
            "stool",
            "crib",
            "shower-glass",
            "bathtub",
            "door",
            "door-double",
            "door-sliding",
            "garage-door",
            "passage",
            "window",
            "window-small",
            "french-window",
            "column",
            "column-round",
            "footing",
            "beam",
            "panel",
            "roof-sheet",
            "railing",
            "glass-railing",
            "balcony-glazing",
            "box",
            "plant",
            "planter",
            "fence",
            "pool",
            "pool-oval",
            "bench",
            "tree",
            "car",
            "outlet-low",
            "outlet-mid",
            "outlet-high",
            "switch",
            "switch-double",
            "switch-3way",
            "light-ceiling",
            "light-wall",
            "electrical-panel",
            "ac-point",
            "shower-point",
            "data-outlet",
            "network-outlet",
            "tv-outlet",
            "wifi-point",
            "telecom-panel",
            "doorbell",
            "outlet-tower",
            "outlet-tower-auto",
            "outlet-tower-4",
            "desk-outlet-box",
            "furniture-outlet",
            "smart-relay",
            "smart-switch",
            "dimmer",
            "presence-sensor",
            "smart-lock",
            "cold-water",
            "hot-water",
            "sewer",
            "floor-drain",
            "floor-drain-100",
            "floor-drain-75",
            "trap-drain-small",
            "dry-drain",
            "linear-drain",
            "linear-drain-trap",
            "rain-drain",
            "valve",
            "grease-trap",
            "vent-grille",
            "vent-pipe",
            "inspection-box",
            "water-meter",
            "gas-point",
        ];
        for item in CATALOG {
            let front = item.front();
            let none = NO_FRONT.contains(&item.id);
            assert!(
                front.stance.has_front() != none,
                "{}: mark its front in newera_core::front, or list it in NO_FRONT",
                item.id
            );
            if front.stance.has_front() {
                assert!(
                    !front.front.is_empty() && !front.back.is_empty(),
                    "{}",
                    item.id
                );
            }
        }
        for id in newera_core::front::listed() {
            assert!(
                find(id).is_some(),
                "newera_core::front names `{id}`, not in the catalog"
            );
        }
    }

    #[test]
    fn every_item_has_a_unique_id_and_sane_size() {
        let mut ids = std::collections::HashSet::new();
        for item in CATALOG {
            assert!(ids.insert(item.id), "duplicate id {}", item.id);
            assert!(item.size.iter().all(|v| *v > 0.0), "{}", item.id);
        }
        assert!(CATALOG.len() >= 50);
    }

    #[test]
    fn every_mesh_fits_its_box_exactly_and_faces_outward() {
        for item in CATALOG {
            for size in [
                item.size,
                [item.size[0] * 1.3, item.size[1] * 0.8, item.size[2] * 1.1],
            ] {
                let mut piece = item.instantiate(FurnitureId(1), Point2::new(0.0, 0.0));
                (piece.width, piece.depth, piece.height) = (size[0], size[1], size[2]);
                let mesh = piece_mesh(&piece);
                assert!(!mesh.indices.is_empty(), "{} has no geometry", item.id);
                assert!(
                    mesh.indices
                        .iter()
                        .all(|i| (*i as usize) < mesh.positions.len())
                );
                let (min, max) = mesh.bounds().unwrap();
                #[allow(clippy::cast_possible_truncation)]
                let (w, h, d) = (size[0] as f32, size[2] as f32, size[1] as f32);
                let tol = 0.05;
                assert!(
                    (max[0] - min[0] - w).abs() < tol,
                    "{} width {} vs {w}",
                    item.id,
                    max[0] - min[0]
                );
                assert!(
                    (max[1] - min[1] - h).abs() < tol,
                    "{} height {} vs {h}",
                    item.id,
                    max[1] - min[1]
                );
                assert!(
                    (max[2] - min[2] - d).abs() < tol,
                    "{} depth {} vs {d}",
                    item.id,
                    max[2] - min[2]
                );
                assert!(min[1].abs() < tol, "{} rests on the floor", item.id);
                mesh::tests::assert_outward(&mesh);
            }
        }
    }

    #[test]
    fn search_is_accent_insensitive_and_bilingual() {
        let ids = |q: &str| search(q).iter().map(|i| i.id).collect::<Vec<_>>();
        assert!(ids("sofa").contains(&"sofa-3"));
        assert!(ids("Sofá 2").contains(&"sofa-2"));
        assert_eq!(ids("geladeira").first(), Some(&"fridge"));
        // Telecom points are found by what an agent or a client calls them.
        assert_eq!(ids("ponto de rede rj45").first(), Some(&"network-outlet"));
        assert_eq!(ids("wifi teto").first(), Some(&"wifi-point"));
        assert!(ids("rack telecom").contains(&"telecom-panel"));
        assert!(ids("antena tv coaxial").contains(&"tv-outlet"));
        // Automation, as the friction log searched for it.
        let automation = ids("rele automacao sonoff dimmer sensor");
        for id in ["smart-relay", "dimmer", "presence-sensor"] {
            assert!(automation.contains(&id), "{id} in {automation:?}");
        }
        assert_eq!(ids("fechadura eletronica").first(), Some(&"smart-lock"));
        assert_eq!(
            ids("interruptor inteligente").first(),
            Some(&"smart-switch")
        );
        for id in ["network-outlet", "tv-outlet", "wifi-point", "telecom-panel"] {
            assert_eq!(
                find(id).unwrap().category.discipline(),
                Some(newera_core::Discipline::Electrical),
                "{id} goes to the electrical project"
            );
        }
        // Several words: any of them, best match first.
        assert_eq!(ids("box vidro chuveiro").first(), Some(&"shower-glass"));
        assert!(ids("cama casal").contains(&"bed-double"));
        assert!(ids("window").contains(&"window"));
        assert_eq!(search("").len(), CATALOG.len());
    }
}
