//! The icons of the window: the regular weight of Phosphor
//! (<https://phosphoricons.com>, MIT — see `assets/Phosphor-LICENSE.txt`),
//! kept here rather than through `egui-phosphor` so the icon set never holds
//! egui back a version or puts a second egui in the build.
//!
//! Only the icons the window uses are named; any other is a codepoint of the
//! same font (the names and codepoints are Phosphor's own).

// The desktop and the browser each leave a few of these unused.
#![allow(dead_code)]

use eframe::egui;

/// The font, Phosphor Regular.
const FONT: &[u8] = include_bytes!("../assets/Phosphor.ttf");

/// Adds the icon font to `fonts`: second among the proportional fonts, so it
/// does not shadow the text font but icons can sit inside any label, and as a
/// family of its own named `phosphor`.
pub(crate) fn add_to_fonts(fonts: &mut egui::FontDefinitions) {
    const NAME: &str = "phosphor";
    fonts
        .font_data
        .insert(NAME.into(), egui::FontData::from_static(FONT).into());
    let proportional = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut family = proportional;
    family.insert(family.len().min(1), NAME.into());
    fonts
        .families
        .insert(egui::FontFamily::Name(NAME.into()), family);
    if let Some(keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        // Second, so it does not shadow the text font — and `Proportional`
        // can be empty when egui is built without its default fonts.
        let at = keys.len().min(1);
        keys.insert(at, NAME.into());
    }
}

pub(crate) const APERTURE: &str = "\u{E00A}";
pub(crate) const ARMCHAIR: &str = "\u{E012}";
pub(crate) const ARROWS_CLOCKWISE: &str = "\u{E094}";
pub(crate) const ARROWS_IN_LINE_HORIZONTAL: &str = "\u{E530}";
pub(crate) const ARROW_CLOCKWISE: &str = "\u{E036}";
pub(crate) const ARROW_COUNTER_CLOCKWISE: &str = "\u{E038}";
pub(crate) const ARROW_SQUARE_IN: &str = "\u{E5DC}";
pub(crate) const BATHTUB: &str = "\u{E81E}";
pub(crate) const BED: &str = "\u{E0CC}";
pub(crate) const CAMERA: &str = "\u{E10E}";
pub(crate) const CARET_DOWN: &str = "\u{E136}";
pub(crate) const CARET_RIGHT: &str = "\u{E13A}";
pub(crate) const CHAIR: &str = "\u{E950}";
pub(crate) const CHART_BAR: &str = "\u{E150}";
pub(crate) const CHECK_CIRCLE: &str = "\u{E184}";
pub(crate) const CLIPBOARD: &str = "\u{E196}";
pub(crate) const CLOCK_COUNTER_CLOCKWISE: &str = "\u{E1A0}";
pub(crate) const COFFEE: &str = "\u{E1C2}";
pub(crate) const COMPASS: &str = "\u{E1C8}";
pub(crate) const COPY: &str = "\u{E1CA}";
pub(crate) const COPY_SIMPLE: &str = "\u{E1CC}";
pub(crate) const CORNERS_OUT: &str = "\u{E1D0}";
pub(crate) const COUCH: &str = "\u{E7F6}";
pub(crate) const CUBE: &str = "\u{E1DA}";
pub(crate) const CURSOR: &str = "\u{E1DC}";
pub(crate) const DESK: &str = "\u{ED16}";
pub(crate) const DOOR: &str = "\u{E61C}";
pub(crate) const DROP: &str = "\u{E210}";
pub(crate) const EXPORT: &str = "\u{EAF0}";
pub(crate) const EYE: &str = "\u{E220}";
pub(crate) const FILE_PLUS: &str = "\u{E236}";
pub(crate) const FILE_TEXT: &str = "\u{E23A}";
pub(crate) const FILM_SLATE: &str = "\u{E8C2}";
pub(crate) const FILM_STRIP: &str = "\u{E792}";
pub(crate) const FLOPPY_DISK: &str = "\u{E248}";
pub(crate) const FLOPPY_DISK_BACK: &str = "\u{EAF4}";
pub(crate) const FOLDER_OPEN: &str = "\u{E256}";
pub(crate) const GLOBE: &str = "\u{E288}";
pub(crate) const HAMMER: &str = "\u{E80E}";
pub(crate) const HAND: &str = "\u{E298}";
pub(crate) const HEARTBEAT: &str = "\u{E2AC}";
pub(crate) const HOUSE_LINE: &str = "\u{E2C4}";
pub(crate) const IMAGE: &str = "\u{E2CA}";
pub(crate) const INFO: &str = "\u{E2CE}";
pub(crate) const KEYBOARD: &str = "\u{E2D8}";
pub(crate) const LIGHTBULB: &str = "\u{E2DC}";
pub(crate) const LIGHTNING: &str = "\u{E2DE}";
pub(crate) const LINE_SEGMENTS: &str = "\u{E6D4}";
pub(crate) const LIST_BULLETS: &str = "\u{E2F2}";
pub(crate) const LIST_DASHES: &str = "\u{E2F4}";
pub(crate) const LIST_NUMBERS: &str = "\u{E2F6}";
pub(crate) const MAGIC_WAND: &str = "\u{E6B6}";
pub(crate) const MAGNIFYING_GLASS: &str = "\u{E30C}";
pub(crate) const MAGNIFYING_GLASS_MINUS: &str = "\u{E30E}";
pub(crate) const MAGNIFYING_GLASS_PLUS: &str = "\u{E310}";
pub(crate) const OVEN: &str = "\u{ED8C}";
pub(crate) const PENCIL_SIMPLE: &str = "\u{E3B4}";
pub(crate) const PERSON_ARMS_SPREAD: &str = "\u{ECFE}";
pub(crate) const PERSON_SIMPLE_WALK: &str = "\u{E73A}";
pub(crate) const PLANT: &str = "\u{EBAE}";
pub(crate) const PLUS: &str = "\u{E3D4}";
pub(crate) const POLYGON: &str = "\u{E6D0}";
pub(crate) const PUZZLE_PIECE: &str = "\u{E596}";
pub(crate) const ROBOT: &str = "\u{E762}";
pub(crate) const RULER: &str = "\u{E6B8}";
pub(crate) const SCISSORS: &str = "\u{EAE0}";
pub(crate) const SELECTION_ALL: &str = "\u{E746}";
pub(crate) const SIGN_OUT: &str = "\u{E42A}";
pub(crate) const SLIDERS_HORIZONTAL: &str = "\u{E434}";
pub(crate) const SQUARES_FOUR: &str = "\u{E464}";
pub(crate) const STACK: &str = "\u{E466}";
pub(crate) const STAIRS: &str = "\u{E8EC}";
pub(crate) const SUN: &str = "\u{E472}";
pub(crate) const TEXT_T: &str = "\u{E48A}";
pub(crate) const TRANSLATE: &str = "\u{E4A2}";
pub(crate) const TRASH: &str = "\u{E4A6}";
pub(crate) const TREE: &str = "\u{E6DA}";
pub(crate) const TRIANGLE: &str = "\u{E4B0}";
pub(crate) const USERS: &str = "\u{E4D6}";
pub(crate) const WALL: &str = "\u{E688}";
pub(crate) const WARNING: &str = "\u{E4E0}";
pub(crate) const WASHING_MACHINE: &str = "\u{EDE8}";
pub(crate) const X: &str = "\u{E4F6}";
pub(crate) const X_CIRCLE: &str = "\u{E4F8}";
