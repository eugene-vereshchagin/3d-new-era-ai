//! "Lista de corte": the boards the storey's joinery is cut from — every
//! cabinet, countertop and panel built in the plan and every group drawn by
//! hand with board parts — by material, with the sheets each material takes,
//! the hardware, and the files a workshop cuts from (CSV for the table, DXF
//! or SVG with the boards laid out on sheets). The workshop's own sheet size,
//! saw kerf and trim are set here.

use crate::icon;
use eframe::egui::{self, RichText};
use newera_core::LevelId;
use newera_joinery::{Gathered, SheetStock, SheetUse};

use crate::app::NewEraApp;

/// Sheet size the layouts take when the workshop does not say, mm.
const DEFAULT_SHEET: [f64; 2] = [2750.0, 1830.0];

/// The window's choices, and the list last read.
#[derive(Debug)]
pub(crate) struct CutListWindow {
    /// The workshop's sheets for every panel (MDF, MDP), mm; `None` keeps
    /// the defaults.
    pub(crate) sheet: Option<[f64; 2]>,
    /// Saw kerf, mm.
    pub(crate) kerf: f64,
    /// Factory edge trimmed off each side of a sheet, mm.
    pub(crate) trim: f64,
    /// The list last read, kept until the plan, the storey or the stock
    /// changes.
    read: Option<Cached>,
}

/// A list read, with what it was read from.
#[derive(Debug)]
struct Cached {
    /// The plan's revision.
    revision: u64,
    /// The storey on screen.
    level: Option<LevelId>,
    stock: SheetStock,
    read: Result<Read, String>,
}

/// A storey's cut list with the sheets each board takes.
#[derive(Debug, Clone)]
pub(crate) struct Read {
    pub(crate) found: Gathered,
    /// The sheets each material takes and the boards no sheet holds whole,
    /// or why the stock cannot be cut from.
    pub(crate) used: Result<SheetUse, String>,
}

impl Default for CutListWindow {
    /// The defaults a cut list has without the workshop: 2750 × 1830 mm
    /// panels, a 4 mm kerf and no trim.
    fn default() -> Self {
        let stock = SheetStock::default();
        Self {
            sheet: None,
            kerf: stock.kerf,
            trim: stock.trim,
            read: None,
        }
    }
}

impl CutListWindow {
    /// The stock the window's choices describe.
    pub(crate) fn stock(&self) -> SheetStock {
        let stock = SheetStock {
            kerf: self.kerf,
            trim: self.trim,
            ..SheetStock::default()
        };
        match self.sheet {
            Some(size) => stock.with_sizes([("*".to_owned(), size)]),
            None => stock,
        }
    }

    /// The list of the storey on screen, read again only when the plan,
    /// the storey or the stock changed since the last time.
    pub(crate) fn read(&mut self, app: &NewEraApp) -> Result<Read, String> {
        let doc = app.document.read();
        let revision = doc.revision();
        let level = doc.home().current_level();
        let stock = self.stock();
        if let Some(cached) = &self.read
            && cached.revision == revision
            && cached.level == level
            && cached.stock == stock
        {
            return cached.read.clone();
        }
        let view = doc.home().level_view(level);
        let read = newera_joinery::gather(&view.furniture, None).map(|found| {
            let used = newera_joinery::sheets_used(&found.rows, &stock);
            Read { found, used }
        });
        self.read = Some(Cached {
            revision,
            level,
            stock,
            read: read.clone(),
        });
        read
    }
}

/// A file the list is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Csv,
    Dxf,
    Svg,
}

impl Format {
    /// The file extension, which also names the format on its button.
    const fn ext(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Dxf => "dxf",
            Self::Svg => "svg",
        }
    }

    /// The file's bytes: the table, or the boards laid out on sheets.
    ///
    /// # Errors
    ///
    /// A layout on a stock that cannot be cut from.
    fn bytes(self, read: &Read, stock: &SheetStock) -> Result<Vec<u8>, String> {
        let text = match self {
            Self::Csv => newera_joinery::cut_list_csv(&read.found.rows),
            Self::Dxf => newera_joinery::cut_list_dxf(&read.found.rows, stock)?.0,
            Self::Svg => newera_joinery::cut_list_svg(&read.found.rows, stock)?.0,
        };
        Ok(text.into_bytes())
    }
}

/// Millimetres as the window's language writes them, without a needless
/// decimal.
fn mm(v: f64) -> String {
    let text = if (v - v.round()).abs() < 0.05 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    };
    if crate::i18n::decimal_comma() {
        text.replace('.', ",")
    } else {
        text
    }
}

/// A field of millimetres written as [`mm`] writes them, read back with
/// either a comma or a dot for the decimals.
pub(crate) fn mm_field(value: &mut f64) -> egui::DragValue<'_> {
    egui::DragValue::new(value)
        .suffix(" mm")
        .custom_formatter(|v, _| mm(v))
        .custom_parser(parse_mm)
}

/// Millimetres typed in a field, with a comma or a dot for the decimals.
fn parse_mm(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse().ok()
}

/// Draws the window while it is open, and writes the file a button asked for.
pub(crate) fn show(app: &mut NewEraApp, ctx: &egui::Context) {
    let Some(mut window) = app.cut_list.take() else {
        return;
    };
    let read = window.read(app);
    let names: std::collections::HashMap<newera_core::FurnitureId, String> = app
        .document
        .read()
        .home()
        .furniture
        .iter()
        .map(|f| (f.id, f.name.clone()))
        .collect();
    let mut open = true;
    let mut export: Option<Format> = None;
    egui::Window::new(format!(
        "{} {}",
        icon::LIST_NUMBERS,
        crate::i18n::tr("Lista de corte")
    ))
    .open(&mut open)
    .default_width(560.0)
    .show(ctx, |ui| {
        stock_choices(ui, &mut window);
        ui.separator();
        match &read {
            Err(err) => {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 50), format!("⚠ {err}"));
            }
            Ok(read) if read.found.is_empty() => {
                ui.weak(crate::i18n::tr(
                    "Nenhuma marcenaria neste pavimento: crie armários (Planta → Armários na parede…) ou desenhe um grupo com chapas.",
                ));
            }
            Ok(read) => {
                ui.label(crate::i18n::fill(
                    "{} e {}.",
                    &[
                        &crate::i18n::count(
                            "{} peça de marcenaria|{} peças de marcenaria",
                            read.found.builds as u64,
                        ),
                        &crate::i18n::count(
                            "{} grupo desenhado|{} grupos desenhados",
                            read.found.drawn.len() as u64,
                        ),
                    ],
                ));
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| boards(ui, read, &names));
                sheet_warnings(ui, &read.used);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(crate::i18n::tr("Salvar como"));
                    for format in [Format::Csv, Format::Dxf, Format::Svg] {
                        if ui
                            .button(format!(
                                "{} {}",
                                icon::EXPORT,
                                format.ext().to_uppercase()
                            ))
                            .clicked()
                        {
                            export = Some(format);
                        }
                    }
                });
                ui.weak(crate::i18n::tr(
                    "CSV: a tabela para a planilha ou o serviço de corte. DXF e SVG: as peças dispostas nas chapas, para a seccionadora ou o CNC.",
                ));
            }
        }
    });
    if let (Some(format), Ok(read)) = (export, &read) {
        match format.bytes(read, &window.stock()) {
            Ok(bytes) => save(app, format, bytes),
            Err(err) => app.set_status(crate::i18n::fill("⚠ Não foi possível salvar: {}", &[&err])),
        }
    }
    if open {
        app.cut_list = Some(window);
    }
}

/// The workshop's sheet size, kerf and trim.
fn stock_choices(ui: &mut egui::Ui, window: &mut CutListWindow) {
    ui.horizontal(|ui| {
        let mut own = window.sheet.is_some();
        if ui
            .checkbox(&mut own, crate::i18n::tr("Chapa da marcenaria"))
            .changed()
        {
            window.sheet = own.then_some(DEFAULT_SHEET);
        }
        match &mut window.sheet {
            Some([length, width]) => {
                ui.add(
                    egui::DragValue::new(length)
                        .range(100.0..=6000.0)
                        .speed(10.0),
                );
                ui.label("×");
                ui.add(
                    egui::DragValue::new(width)
                        .range(100.0..=6000.0)
                        .speed(10.0),
                );
                ui.label("mm");
            }
            None => {
                ui.weak(format!(
                    "{} × {} mm",
                    mm(DEFAULT_SHEET[0]),
                    mm(DEFAULT_SHEET[1])
                ));
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(crate::i18n::tr("Serra"));
        ui.add(mm_field(&mut window.kerf).range(0.0..=10.0).speed(0.1));
        ui.label(crate::i18n::tr("Refilo por lado"));
        // A sheet keeps at least 20 mm once trimmed on both sides.
        ui.add(mm_field(&mut window.trim).range(0.0..=40.0).speed(0.5));
    });
}

/// The boards by material, each with the sheets it takes, then the hardware
/// and the parts that are not boards.
fn boards(
    ui: &mut egui::Ui,
    read: &Read,
    names: &std::collections::HashMap<newera_core::FurnitureId, String>,
) {
    let mut materials: Vec<&str> = Vec::new();
    for row in &read.found.rows {
        if !materials.contains(&row.board.as_str()) {
            materials.push(&row.board);
        }
    }
    for material in materials {
        let sheets = read.used.as_ref().map_or(0, |used| {
            used.sheets
                .iter()
                .find(|(board, _)| board == material)
                .map_or(0, |(_, n)| *n)
        });
        let pieces: u32 = read
            .found
            .rows
            .iter()
            .filter(|r| r.board == material)
            .map(|r| r.qty)
            .sum();
        let mut count = crate::i18n::count("{} peça|{} peças", u64::from(pieces));
        if sheets > 0 {
            count = format!(
                "{count}, {}",
                crate::i18n::count("{} chapa|{} chapas", u64::from(sheets))
            );
        }
        let title = crate::i18n::fill("{}: {}", &[&material, &count]);
        egui::CollapsingHeader::new(RichText::new(title).strong())
            .id_salt(("cut-list", material))
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new(("cut-list-rows", material))
                    .striped(true)
                    .show(ui, |ui| {
                        ui.weak(crate::i18n::tr("Nome da peça"));
                        ui.weak(crate::i18n::tr("Qtd"));
                        ui.weak(crate::i18n::tr("Comprimento × largura"));
                        ui.weak(crate::i18n::tr("Fita"));
                        ui.end_row();
                        for row in read.found.rows.iter().filter(|r| r.board == material) {
                            ui.label(&row.name);
                            ui.label(row.qty.to_string());
                            ui.label(format!("{} × {} mm", mm(row.size[0]), mm(row.size[1])));
                            ui.label(format!("{}+{}", row.edge[0], row.edge[1]));
                            ui.end_row();
                        }
                    });
            });
    }
    if !read.found.hardware.is_empty() {
        egui::CollapsingHeader::new(RichText::new(crate::i18n::tr("Ferragens")).strong())
            .id_salt("cut-list-hardware")
            .show(ui, |ui| {
                for (id, item) in &read.found.hardware {
                    let name = names.get(id).map_or_else(|| id.to_string(), Clone::clone);
                    ui.label(format!("{name}: {item}"));
                }
            });
    }
    for (_, name) in &read.found.skipped {
        ui.label(
            RichText::new(format!(
                "{} {}",
                icon::INFO,
                crate::i18n::fill("Fora da lista, não é chapa: {}", &[name])
            ))
            .weak(),
        );
    }
}

/// The boards no sheet holds whole, or why the stock cannot be cut from:
/// both shown before a file is written, not found in it at the saw.
fn sheet_warnings(ui: &mut egui::Ui, used: &Result<SheetUse, String>) {
    let warn = egui::Color32::from_rgb(200, 60, 50);
    match used {
        Err(err) => {
            ui.colored_label(warn, format!("⚠ {err}"));
        }
        Ok(used) if !used.oversize.is_empty() => {
            ui.colored_label(
                warn,
                format!(
                    "⚠ {}",
                    crate::i18n::fill(
                        "Maior que a chapa, prever emenda ou chapa maior: {}",
                        &[&used.oversize.join("; ")],
                    )
                ),
            );
        }
        Ok(_) => {}
    }
}

/// Writes `bytes` where the user says: a save dialog on the desktop, a
/// download in the browser.
fn save(app: &mut NewEraApp, format: Format, bytes: Vec<u8>) {
    let ext = format.ext();
    let name = format!("{}-corte.{ext}", app.document.read().home().name);
    #[cfg(target_arch = "wasm32")]
    match crate::files::save_bytes(&ext.to_uppercase(), ext, &name, move || Ok(bytes)) {
        Ok(Some(path)) => app.set_status(crate::i18n::fill("Lista de corte salva em {}", &[&path])),
        Ok(None) => {}
        Err(err) => app.set_status(crate::i18n::fill("⚠ Não foi possível salvar: {}", &[&err])),
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dialog = crate::files::save_dialog(&ext.to_uppercase(), ext, &name);
        app.ask_file(dialog, move |app, path| {
            let Some(path) = path else {
                return;
            };
            match std::fs::write(&path, bytes) {
                Ok(()) => app.set_status(crate::i18n::fill(
                    "Lista de corte salva em {}",
                    &[&path.display()],
                )),
                Err(err) => {
                    app.set_status(crate::i18n::fill("⚠ Não foi possível salvar: {}", &[&err]));
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use newera_core::{Command, Document, Point2, SharedDocument, Wall};

    use super::*;

    /// The kitchen made in the app is cut in the app: cabinets built on a
    /// wall show up in the list by material with their sheets, and a sheet of
    /// the workshop's own size reads the list again with it.
    #[test]
    fn the_cabinets_built_on_a_wall_are_listed_by_board() {
        let mut doc = Document::default();
        let corners = [(0.0, 0.0), (300.0, 0.0), (300.0, 240.0), (0.0, 240.0)];
        let mut first = None;
        for k in 0..4 {
            let (a, b) = (corners[k], corners[(k + 1) % 4]);
            let id = doc.new_wall_id();
            first.get_or_insert(id);
            doc.execute(Command::insert(Wall::new(
                id,
                Point2::new(a.0, a.1),
                Point2::new(b.0, b.1),
            )))
            .unwrap();
        }
        let params: newera_joinery::CabinetRunParams = serde_json::from_value(
            serde_json::json!({"wall": first.unwrap().to_string(), "p": {"row": "base"}}),
        )
        .unwrap();
        newera_joinery::cabinet_run(&mut doc, &params).unwrap();
        let document = SharedDocument::new(doc);
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 900.0))
            .build_eframe(move |cc| NewEraApp::new(cc, document, None));
        h.run_steps(3);
        h.get_by_label("Planta").click();
        h.run_steps(2);
        h.get_by_label_contains("Lista de corte").click();
        h.run_steps(3);
        assert!(h.state().cut_list.is_some(), "the window is open");
        h.get_by_label_contains("MDF 18");
        h.get_by_label(&format!("{} CSV", icon::EXPORT));
        let before = {
            let mut window = h.state_mut().cut_list.take().unwrap();
            let read = window.read(h.state()).unwrap();
            h.state_mut().cut_list = Some(window);
            read
        };
        assert!(before.found.builds > 0);
        let used = |r: &Read| r.used.clone().unwrap();
        assert!(
            used(&before).sheets.iter().any(|(_, n)| *n > 0),
            "{:?}",
            before.used
        );
        assert!(used(&before).oversize.is_empty(), "{:?}", before.used);
        // A sheet four times smaller takes more of them.
        h.state_mut().cut_list.as_mut().unwrap().sheet = Some([1375.0, 915.0]);
        let mut window = h.state_mut().cut_list.take().unwrap();
        let after = window.read(h.state()).unwrap();
        let total = |r: &Read| used(r).sheets.iter().map(|(_, n)| n).sum::<u32>();
        assert!(
            total(&after) > total(&before),
            "{:?} {:?}",
            before.used,
            after.used
        );
        let csv = String::from_utf8(Format::Csv.bytes(&after, &window.stock()).unwrap()).unwrap();
        assert!(csv.contains("MDF 18"), "{csv}");
        // A sheet smaller than a cabinet's side says so on screen, before the
        // file reaches the saw.
        window.sheet = Some([500.0, 500.0]);
        h.state_mut().cut_list = Some(window);
        h.run_steps(3);
        h.get_by_label_contains("Maior que a chapa");
    }

    /// Millimetres read the way the window writes them: no ".0" on a whole
    /// number, and a comma typed for the decimals is understood.
    #[test]
    fn millimetres_drop_the_needless_decimal() {
        assert_eq!(mm(4.0), "4");
        assert_eq!(mm(18.0), "18");
        assert_eq!(parse_mm("3,2"), Some(3.2));
        assert_eq!(parse_mm(" 3.2 "), Some(3.2));
        assert_eq!(parse_mm("três"), None);
    }
}
