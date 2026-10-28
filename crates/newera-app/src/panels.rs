//! Left side panel: the catalog to take pieces from on top, the project to
//! find them in below. One search line, sections titled like a drawing's
//! notes, and a single row shape everywhere — so the eye learns one pattern
//! and then reads the whole panel with it.

use crate::icon;
use eframe::egui::{self, RichText, Sense, Stroke, Vec2};
use newera_catalog::{CatalogItem, Category, SymbolShape};
use newera_core::{ElementId, FurnitureId, Point2};

use crate::app::NewEraApp;
use crate::view::plan::Tool;

fn category_icon(category: Category) -> &'static str {
    match category {
        Category::Living => icon::COUCH,
        Category::Dining => icon::CHAIR,
        Category::Kitchen => icon::OVEN,
        Category::Bedroom => icon::BED,
        Category::Bathroom => icon::BATHTUB,
        Category::Laundry => icon::WASHING_MACHINE,
        Category::Office => icon::DESK,
        Category::DoorsWindows => icon::DOOR,
        Category::Structure => icon::STAIRS,
        Category::Decor => icon::PLANT,
        Category::Lighting => icon::LIGHTBULB,
        Category::Outdoor => icon::TREE,
        Category::Electrical => icon::LIGHTNING,
        Category::Plumbing => icon::DROP,
    }
}

/// Small top-view thumbnail drawn from the item's plan symbol.
fn thumbnail(ui: &mut egui::Ui, item: &CatalogItem, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let t = crate::theme::of(ui.visuals());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, t.inset);
    let piece = item.instantiate(FurnitureId(0), Point2::new(0.0, 0.0));
    // Door swings extend in front of the opening; frame them too.
    let extent_y = if piece.is_opening() {
        piece.depth / 2.0 + piece.width
    } else {
        piece.depth
    };
    #[allow(clippy::cast_possible_truncation)]
    let scale = (f64::from(size - 6.0) / piece.width.max(extent_y)) as f32;
    let offset_y = if piece.is_opening() {
        -(piece.width / 2.0)
    } else {
        0.0
    };
    #[allow(clippy::cast_possible_truncation)]
    let to =
        |(x, y): (f64, f64)| rect.center() + Vec2::new(x as f32, (y + offset_y) as f32) * scale;
    for shape in newera_catalog::plan_symbol(&piece) {
        match shape {
            SymbolShape::Fill { points, detail } => {
                let pts: Vec<Point2> = points.iter().map(|&(x, y)| Point2::new(x, y)).collect();
                let mut mesh = egui::Mesh::default();
                let color = if detail { t.rule_strong } else { t.ink_dim };
                for p in &points {
                    mesh.colored_vertex(to(*p), color);
                }
                for [a, b, c] in newera_core::triangulate(&pts) {
                    #[allow(clippy::cast_possible_truncation)]
                    mesh.add_triangle(a as u32, b as u32, c as u32);
                }
                painter.add(egui::Shape::mesh(mesh));
            }
            SymbolShape::Line {
                points,
                closed,
                strong,
            } => {
                let mut pts: Vec<egui::Pos2> = points.iter().map(|p| to(*p)).collect();
                if closed && let Some(&first) = pts.first() {
                    pts.push(first);
                }
                let width = if strong { 1.2 } else { 0.7 };
                painter.add(egui::Shape::line(pts, Stroke::new(width, t.ink)));
            }
        }
    }
}

/// A name in a row, cut short with an ellipsis rather than drawn in full.
///
/// A panel is as wide as what it draws, and a plan may well carry a note a
/// paragraph long ("AÉREOS: 80 geladeira + 106 cozinha…"). Drawn whole, such a
/// name pins the panel open: the drag handle gives way and the next frame
/// pushes it back out. `beside` is the room to leave for whatever the row puts
/// after the name — its size, its count. The full name is a hover away.
fn name(ui: &mut egui::Ui, text: &str, styled: RichText, beside: f32) {
    let room = (ui.available_width() - beside).max(48.0);
    let response = ui
        .scope(|ui| {
            ui.set_max_width(room);
            ui.add(egui::Label::new(styled).truncate())
        })
        .inner;
    if response.rect.width() >= room - 0.5 {
        response.on_hover_text(text); // Only what did not fit is worth a tooltip.
    }
}

/// A panel's title: the icon in the accent, the name, and the number that
/// says how much is in there set as a note in the margin.
fn header(ui: &mut egui::Ui, glyph: &str, title: &str, note: &str) {
    let t = crate::theme::of(ui.visuals());
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(RichText::new(glyph).size(15.0).color(t.accent));
        name(ui, title, RichText::new(title).heading().color(t.ink), 90.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(crate::theme::fig(ui.visuals(), note));
        });
    });
    ui.add_space(4.0);
}

/// The hairline that closes a block, drawn edge to edge instead of inset like
/// `ui.separator`, so the panel reads as stacked bands.
fn rule(ui: &mut egui::Ui) {
    let t = crate::theme::of(ui.visuals());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(
        ui.max_rect().x_range(),
        rect.center().y,
        Stroke::new(1.0, t.rule),
    );
}

/// The search line: one well, the glass inside it, and a cross that appears
/// only when there is something to clear.
fn search(ui: &mut egui::Ui, query: &mut String) {
    let t = crate::theme::of(ui.visuals());
    egui::Frame::new()
        .fill(t.inset)
        .stroke(Stroke::new(1.0, t.rule))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon::MAGNIFYING_GLASS).color(t.ink_faint));
                let clear = !query.is_empty();
                let width = ui.available_width() - if clear { 22.0 } else { 0.0 };
                ui.add_sized(
                    Vec2::new(width, 20.0),
                    egui::TextEdit::singleline(query)
                        .frame(egui::Frame::NONE)
                        .hint_text(crate::i18n::tr("Buscar: cama, janela, sofá…")),
                );
                if clear
                    && ui
                        .add(egui::Button::new(RichText::new(icon::X).size(11.0)).frame(false))
                        .clicked()
                {
                    query.clear();
                }
            });
        });
}

/// One line of a list: full width, a bar of accent down the left when it is
/// the one selected, a quiet lift under the pointer. `name` is what a screen
/// reader — and a test — reads the line as.
fn row<R>(
    ui: &mut egui::Ui,
    selected: bool,
    name: &str,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, R) {
    let t = crate::theme::of(ui.visuals());
    let backdrop = ui.painter().add(egui::Shape::Noop);
    let mut out = None;
    let response = ui
        .scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                out = Some(add(ui));
            });
        })
        .response;
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, name));
    let rect = response.rect.expand2(Vec2::new(0.0, 2.0));
    if selected || response.hovered() {
        let fill = if selected { t.accent_soft } else { t.raised };
        ui.painter()
            .set(backdrop, egui::epaint::RectShape::filled(rect, 5, fill));
    }
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.left_top(), Vec2::new(2.0, rect.height())),
            1,
            t.accent,
        );
    }
    (response, out.expect("the row body ran"))
}

/// The number at the end of a section title, set as a note.
fn count(ui: &mut egui::Ui, n: usize) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(4.0);
        ui.label(crate::theme::fig(ui.visuals(), &n.to_string()));
    });
}

/// A band of the panel: what it is called, how much is in it, and whether it
/// starts open.
#[derive(Clone, Copy)]
struct Band<'a> {
    id: egui::Id,
    glyph: &'a str,
    title: &'a str,
    total: usize,
    /// Open the first time it is drawn.
    open: bool,
    /// Open it now, whatever it was: something inside was just selected.
    reveal: bool,
}

/// A band that opens: the arrow, the icon, the name, the count. Returns what
/// the body drew, when it is open.
fn band<R>(ui: &mut egui::Ui, band: Band<'_>, body: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    let Band {
        id,
        glyph,
        title,
        total,
        open,
        reveal,
    } = band;
    let t = crate::theme::of(ui.visuals());
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id.with("band"),
        open,
    );
    // A selection made out on the plan opens the band that holds it.
    if reveal && !state.is_open() {
        state.set_open(true);
        state.store(ui.ctx());
    }
    let is_open = state.is_open();
    let (response, ()) = row(ui, false, title, |ui| {
        ui.label(
            RichText::new(if is_open {
                icon::CARET_DOWN
            } else {
                icon::CARET_RIGHT
            })
            .size(10.0)
            .color(t.ink_faint),
        );
        ui.label(RichText::new(glyph).color(if is_open { t.accent } else { t.ink_dim }));
        name(ui, title, RichText::new(title).color(t.ink), 40.0);
        count(ui, total);
    });
    if response.clicked() {
        state.toggle(ui);
    }
    let body = state.show_body_indented(&response, ui, body);
    body.map(|inner| inner.inner)
}

/// The catalog's own search, plus the names as this window writes them: a
/// Spanish window has to find the sofa by the word on its own screen.
fn find(query: &str) -> Vec<&'static CatalogItem> {
    let mut found = newera_catalog::search(query);
    if crate::i18n::lang() == crate::i18n::Lang::Pt {
        return found;
    }
    let words: Vec<String> = newera_catalog::fold(query)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    for item in newera_catalog::CATALOG {
        if found.iter().any(|f| f.id == item.id) {
            continue;
        }
        let name = newera_catalog::fold(crate::i18n::tr(item.name));
        if words.iter().any(|word| name.contains(word.as_str())) {
            found.push(item);
        }
    }
    found
}

fn catalog_row(app: &mut NewEraApp, ui: &mut egui::Ui, item: &'static CatalogItem) {
    let selected = app.tool == Tool::Place(item.id);
    let unit = app.unit();
    let t = crate::theme::of(ui.visuals());
    let name = crate::i18n::tr(item.name);
    let (response, ()) = row(ui, selected, name, |ui| {
        thumbnail(ui, item, 30.0);
        ui.vertical(|ui| {
            ui.add_space(1.0);
            self::name(
                ui,
                name,
                RichText::new(name).color(if selected { t.accent } else { t.ink }),
                0.0,
            );
            ui.label(crate::theme::fig(
                ui.visuals(),
                &unit.format_size(item.size),
            ));
        });
    });
    if response
        .on_hover_text(crate::i18n::tr(
            "Clique e depois clique na planta para posicionar",
        ))
        .clicked()
    {
        app.set_tool(Tool::Place(item.id));
    }
}

pub(crate) fn left(app: &mut NewEraApp, ui: &mut egui::Ui) {
    // Every name here names a row. Left selectable, egui hands the click to
    // the text — it comes out as a text selection and the row never hears it.
    ui.style_mut().interaction.selectable_labels = false;
    egui::Panel::top("catalog")
        .resizable(true)
        .default_size(ui.available_height() * 0.55)
        .show(ui, |ui| {
            header(
                ui,
                icon::ARMCHAIR,
                crate::i18n::tr("Catálogo"),
                &format!(
                    "{} {}",
                    newera_catalog::CATALOG.len(),
                    crate::i18n::tr("peças")
                ),
            );
            let mut query = std::mem::take(&mut app.catalog_query);
            search(ui, &mut query);
            app.catalog_query = query;
            ui.add_space(6.0);
            rule(ui);
            egui::ScrollArea::vertical()
                .id_salt("catalog_scroll")
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    let query = app.catalog_query.trim().to_owned();
                    if query.is_empty() {
                        for category in Category::ALL {
                            let items: Vec<&'static CatalogItem> = newera_catalog::CATALOG
                                .iter()
                                .filter(|i| i.category == category)
                                .collect();
                            band(
                                ui,
                                Band {
                                    id: egui::Id::unique(category.id()),
                                    glyph: category_icon(category),
                                    title: crate::i18n::tr(category.name()),
                                    total: items.len(),
                                    open: false,
                                    reveal: false,
                                },
                                |ui| {
                                    for item in items {
                                        catalog_row(app, ui, item);
                                    }
                                },
                            );
                        }
                    } else {
                        let found = find(&query);
                        if found.is_empty() {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(crate::i18n::tr("Nada encontrado."))
                                    .color(crate::theme::of(ui.visuals()).ink_faint),
                            );
                        }
                        for item in found {
                            catalog_row(app, ui, item);
                        }
                    }
                    ui.add_space(8.0);
                });
        });

    egui::CentralPanel::default().show(ui, |ui| outliner(app, ui));
}

/// What the project is made of, by kind. The chips at the top say which kinds
/// are worth looking at right now.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Furniture,
    Rooms,
    Walls,
    Dimensions,
    Labels,
    Lines,
}

impl Kind {
    const ALL: [Self; 6] = [
        Self::Furniture,
        Self::Rooms,
        Self::Walls,
        Self::Dimensions,
        Self::Labels,
        Self::Lines,
    ];

    const fn glyph(self) -> &'static str {
        match self {
            Self::Furniture => icon::ARMCHAIR,
            Self::Rooms => icon::POLYGON,
            Self::Walls => icon::WALL,
            Self::Dimensions => icon::RULER,
            Self::Labels => icon::TEXT_T,
            Self::Lines => icon::LINE_SEGMENTS,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Furniture => crate::i18n::tr("Móveis"),
            Self::Rooms => crate::i18n::tr("Cômodos"),
            Self::Walls => crate::i18n::tr("Paredes"),
            Self::Dimensions => crate::i18n::tr("Cotas"),
            Self::Labels => crate::i18n::tr("Textos"),
            Self::Lines => crate::i18n::tr("Linhas"),
        }
    }

    const fn bit(self) -> u8 {
        match self {
            Self::Furniture => 1,
            Self::Rooms => 2,
            Self::Walls => 4,
            Self::Dimensions => 8,
            Self::Labels => 16,
            Self::Lines => 32,
        }
    }
}

/// One filter key: pressed means "show me this kind".
fn chip(ui: &mut egui::Ui, on: bool, kind: Kind, total: usize) -> egui::Response {
    let t = crate::theme::of(ui.visuals());
    let button = egui::Button::new(RichText::new(kind.glyph()).size(14.0).color(if on {
        t.accent
    } else {
        t.ink_faint
    }))
    .min_size(Vec2::new(26.0, 22.0))
    .corner_radius(egui::CornerRadius::same(6))
    .fill(if on { t.accent_soft } else { t.inset })
    .stroke(Stroke::NONE);
    ui.add_enabled(total > 0, button)
        .on_hover_text(format!("{} ({total})", kind.title()))
}

fn outliner(app: &mut NewEraApp, ui: &mut egui::Ui) {
    let home = {
        let doc = app.document.read();
        doc.home().level_view(doc.home().current_level())
    };
    let unit = app.unit();
    let total: f64 = home.rooms.iter().map(newera_core::Room::area).sum();
    header(
        ui,
        icon::HOUSE_LINE,
        &crate::i18n::shown(&home.name),
        &unit.format_area(total),
    );
    ui.label(crate::theme::fig(
        ui.visuals(),
        &format!(
            "{} {} · {} {} · {} {}",
            home.walls.len(),
            crate::i18n::tr("paredes"),
            home.rooms.len(),
            crate::i18n::tr("cômodos"),
            home.furniture.len(),
            crate::i18n::tr("móveis"),
        ),
    ));
    ui.add_space(6.0);

    let counts = |kind: Kind| match kind {
        Kind::Furniture => home.furniture.len(),
        Kind::Rooms => home.rooms.len(),
        Kind::Walls => home.walls.len(),
        Kind::Dimensions => home.dimensions.len(),
        Kind::Labels => home.labels.len(),
        Kind::Lines => home.polylines.len(),
    };
    let filter_id = egui::Id::unique("outliner_filter");
    let mut hidden = ui.data(|d| d.get_temp::<u8>(filter_id).unwrap_or(0));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for kind in Kind::ALL {
            if chip(ui, hidden & kind.bit() == 0, kind, counts(kind)).clicked() {
                hidden ^= kind.bit();
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(filter_id, hidden));
    ui.add_space(6.0);
    rule(ui);

    // What is picked out on the plan has to light up here too, and the panel
    // scrolls to it: with a hundred pieces the highlight is no use if it is
    // three screens down. Only when the selection actually changed, so the
    // list stays where it was put while nobody is selecting anything.
    let mark = format!("{:?}", app.selection);
    let focus_id = egui::Id::unique("outliner_focus");
    let moved = ui.data(|d| d.get_temp::<String>(focus_id)) != Some(mark.clone());
    ui.data_mut(|d| d.insert_temp(focus_id, mark));

    let mut clicked: Option<(ElementId, bool)> = None;
    let mut modify: Option<ElementId> = None;
    egui::ScrollArea::vertical()
        .id_salt("home_scroll")
        .show(ui, |ui| {
            ui.add_space(4.0);
            let mut section = |ui: &mut egui::Ui, kind: Kind, rows: Vec<(ElementId, String, String)>| {
                if rows.is_empty() || hidden & kind.bit() != 0 {
                    return;
                }
                let n = rows.len();
                let holds_selection = rows.iter().any(|(id, ..)| app.selection.contains(id));
                band(
                    ui,
                    Band {
                        id: egui::Id::unique(("outliner", kind.bit())),
                        glyph: kind.glyph(),
                        title: kind.title(),
                        total: n,
                        open: true,
                        reveal: moved && holds_selection,
                    },
                    |ui: &mut egui::Ui| {
                        let t = crate::theme::of(ui.visuals());
                        for (id, name, note) in rows {
                            let selected = app.selection.contains(&id);
                            let (response, ()) = row(ui, selected, &name, |ui| {
                                self::name(
                                    ui,
                                    &name,
                                    RichText::new(name.clone())
                                        .color(if selected { t.ink } else { t.ink_dim }),
                                    if note.is_empty() { 0.0 } else { 130.0 },
                                );
                                if !note.is_empty() {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_space(4.0);
                                            ui.label(crate::theme::fig(ui.visuals(), &note));
                                        },
                                    );
                                }
                            });
                            if selected && moved {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                            if response.clicked() {
                                clicked = Some((id, ui.input(|i| i.modifiers.command)));
                            }
                            if response.double_clicked() {
                                modify = Some(id);
                            }
                        }
                    },
                );
            };
            section(
                ui,
                Kind::Furniture,
                home.furniture
                    .iter()
                    .map(|f| {
                        let kind = if f.is_group() {
                            format!("{} {}", crate::i18n::tr("grupo de"), f.flatten().len() - 1)
                        } else if f.light.is_some() {
                            crate::i18n::tr("luz").to_owned()
                        } else {
                            unit.format_size([f.width, f.depth, f.height])
                        };
                        (f.id.into(), crate::i18n::shown(&f.name), kind)
                    })
                    .collect(),
            );
            section(
                ui,
                Kind::Rooms,
                home.rooms
                    .iter()
                    .map(|r| {
                        let name = if r.name.is_empty() {
                            crate::i18n::tr("Sem nome").to_owned()
                        } else {
                            r.name.clone()
                        };
                        (r.id.into(), name, unit.format_area(r.area()))
                    })
                    .collect(),
            );
            section(
                ui,
                Kind::Walls,
                home.walls
                    .iter()
                    .map(|w| {
                        let arc = if w.is_arc() {
                            crate::i18n::tr(" · arco")
                        } else {
                            ""
                        };
                        (
                            w.id.into(),
                            format!("{}{arc}", w.id),
                            unit.format_length(w.length()),
                        )
                    })
                    .collect(),
            );
            section(
                ui,
                Kind::Dimensions,
                home.dimensions
                    .iter()
                    .map(|d| {
                        (
                            d.id.into(),
                            format!("{}", d.id),
                            unit.format_length(d.length()),
                        )
                    })
                    .collect(),
            );
            section(
                ui,
                Kind::Labels,
                home.labels
                    .iter()
                    .map(|l| {
                        (
                            l.id.into(),
                            l.text.lines().next().unwrap_or_default().to_owned(),
                            String::new(),
                        )
                    })
                    .collect(),
            );
            section(
                ui,
                Kind::Lines,
                home.polylines
                    .iter()
                    .map(|l| {
                        let length: f64 = l.points.windows(2).map(|p| p[0].distance(p[1])).sum();
                        (
                            l.id.into(),
                            format!("{}", l.id),
                            unit.format_length(length),
                        )
                    })
                    .collect(),
            );
            if home.walls.is_empty() && home.rooms.is_empty() && home.furniture.is_empty() {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(crate::i18n::tr(
                        "Comece desenhando paredes (W), importe uma planta como imagem de fundo, ou peça para a IA via MCP.",
                    ))
                    .color(crate::theme::of(ui.visuals()).ink_faint),
                );
            }
            ui.add_space(8.0);
        });

    if let Some((id, additive)) = clicked {
        if !additive {
            app.selection.clear();
        }
        if !app.selection.remove(&id) || !additive {
            app.selection.insert(id);
        }
    }
    if let Some(id) = modify {
        app.open_modify(&[id]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The note that started it: a label long enough to make the panel as wide
    /// as itself, after which the drag handle could not bring it back.
    const LONG: &str = "AÉREOS: 80 geladeira + 106 cozinha + 94 coifa; lavanderia 70,75 \
        + 70,75. Envoltória e fixação da coifa: estudo sujeito ao manual específico.";

    /// How wide a 260 px panel ends up after drawing one row with this name in
    /// it. A panel is as wide as what it draws, which is the whole of the bug:
    /// a name drawn in full pushed it open and held it there. The frame is
    /// drawn twice because the first one loads the fonts.
    fn panel_width(text: &str) -> f32 {
        let ctx = egui::Context::default();
        let mut width = 0.0;
        for _ in 0..2 {
            let mut frame = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 800.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    let panel = egui::Panel::left("test")
                        .resizable(true)
                        .default_size(260.0)
                        .show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                name(ui, text, RichText::new(text), 40.0);
                            });
                        });
                    width = panel.response.rect.width();
                },
            );
            // Nobody is drawing, so nobody applies them; unapplied deltas
            // panic on their way out.
            frame.textures_delta.clear();
        }
        width
    }

    /// Drawn in full this name takes about 810 px, and the panel went with it.
    #[test]
    fn a_long_name_never_pushes_the_panel_wider_than_it_is() {
        let width = panel_width(LONG);
        assert!(width <= 261.0, "260 px of panel went to {width}");
    }

    #[test]
    fn a_short_name_asks_for_no_room_at_all() {
        let width = panel_width("Sofá");
        assert!(width <= 261.0, "260 px of panel went to {width}");
    }
}
