//! "Armários na parede": fills the selected wall with cabinets sized for it
//! (base, wall or tall row), with the sink and cooktop where the connections
//! are. A preview lists the modules before anything changes.

use crate::icon;
use eframe::egui::{self, RichText};
use newera_core::WallId;
use newera_joinery::{CabinetRunParams, RunRow};

use crate::app::NewEraApp;

/// Choices for the wall being filled, and the last plan.
#[derive(Debug)]
pub(crate) struct CabinetsWindow {
    pub(crate) wall: WallId,
    row: RunRow,
    sink: Option<f64>,
    cooktop: Option<f64>,
    front: [u8; 3],
    drawers: Option<u32>,
    top: bool,
    /// Reply of the last preview or build.
    pub(crate) result: Option<Result<serde_json::Value, String>>,
    built: bool,
}

impl CabinetsWindow {
    pub(crate) fn new(wall: WallId) -> Self {
        Self {
            wall,
            row: RunRow::Base,
            sink: None,
            cooktop: None,
            front: [238, 236, 230],
            drawers: None,
            top: true,
            result: None,
            built: false,
        }
    }

    fn params(&self, dry: bool) -> CabinetRunParams {
        let mut p = serde_json::Map::new();
        p.insert(
            "row".into(),
            match self.row {
                RunRow::Base => "base",
                RunRow::Wall => "wall",
                RunRow::Tall => "tall",
            }
            .into(),
        );
        let [r, g, b] = self.front;
        p.insert("front".into(), format!("#{r:02x}{g:02x}{b:02x}").into());
        if self.row == RunRow::Base {
            p.insert("top".into(), self.top.into());
            if let Some(x) = self.sink {
                p.insert("sink".into(), x.into());
            }
            if let Some(x) = self.cooktop {
                p.insert("cooktop".into(), x.into());
            }
        }
        if let Some(n) = self.drawers {
            p.insert("drawers".into(), n.into());
        }
        CabinetRunParams {
            wall: Some(self.wall.to_string()),
            p: Some(p),
            dry,
            ..CabinetRunParams::default()
        }
    }
}

fn role_name(role: &str) -> &'static str {
    match role {
        "doors" => crate::i18n::tr("portas"),
        "drawers" => crate::i18n::tr("gaveteiro"),
        "slim" => crate::i18n::tr("porta-temperos"),
        "corner" => crate::i18n::tr("canto cego"),
        "hanging" => crate::i18n::tr("cabideiro"),
        "over" => crate::i18n::tr("sobre a geladeira"),
        "sink" => crate::i18n::tr("pia"),
        "cooktop" => crate::i18n::tr("cooktop"),
        "filler" => crate::i18n::tr("tamponamento"),
        "countertop" => crate::i18n::tr("bancada"),
        _ => "",
    }
}

fn optional_position(ui: &mut egui::Ui, label: &str, value: &mut Option<f64>, length: f64) {
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *value = on.then_some(length / 2.0);
        }
        if let Some(x) = value {
            ui.add(
                egui::DragValue::new(x)
                    .range(0.0..=length)
                    .suffix(" cm")
                    .speed(1.0),
            );
            ui.weak(crate::i18n::tr("do início da parede"));
        }
    });
}

/// Shows the window while it is open.
#[allow(clippy::too_many_lines)]
pub(crate) fn show(app: &mut NewEraApp, ctx: &egui::Context) {
    let Some(mut window) = app.cabinets.take() else {
        return;
    };
    let length = app
        .document
        .read()
        .home()
        .wall(window.wall)
        .map(|w| w.start.distance(w.end));
    let Some(length) = length else {
        // The wall is gone (undo, delete).
        return;
    };
    let mut open = true;
    let mut run: Option<bool> = None;
    egui::Window::new(format!(
        "{} {} {}",
        icon::SQUARES_FOUR,
        crate::i18n::tr("Armários na parede"),
        window.wall
    ))
    .open(&mut open)
    .default_width(460.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (row, name) in [
                (RunRow::Base, crate::i18n::tr("Balcões")),
                (RunRow::Wall, crate::i18n::tr("Aéreos")),
                (RunRow::Tall, crate::i18n::tr("Torres / guarda-roupa")),
            ] {
                ui.selectable_value(&mut window.row, row, name);
            }
        });
        if window.row == RunRow::Base {
            optional_position(ui, crate::i18n::tr("Pia centrada em"), &mut window.sink, length);
            optional_position(
                ui,
                crate::i18n::tr("Cooktop centrado em"),
                &mut window.cooktop,
                length,
            );
            ui.checkbox(&mut window.top, crate::i18n::tr("Bancada de pedra por cima"));
        }
        ui.horizontal(|ui| {
            ui.label(crate::i18n::tr("Frentes"));
            ui.color_edit_button_srgb(&mut window.front);
            let mut auto = window.drawers.is_none();
            ui.label(crate::i18n::tr("Gaveteiros"));
            if ui.checkbox(&mut auto, crate::i18n::tr("automático")).changed() {
                window.drawers = if auto { None } else { Some(1) };
            }
            if let Some(n) = &mut window.drawers {
                ui.add(egui::DragValue::new(n).range(0..=6));
            }
        });
        ui.weak(crate::i18n::tr(
            "Mede o trecho livre entre cantos, portas, janelas e eletrodomésticos e divide em módulos sem sobras; substitui os armários que já estão nessa parede.",
        ));
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .button(format!("{} {}", icon::EYE, crate::i18n::tr("Pré-visualizar")))
                .clicked()
            {
                run = Some(true);
            }
            if ui
                .button(
                    RichText::new(format!("{} {}", icon::HAMMER, crate::i18n::tr("Criar armários")))
                        .strong(),
                )
                .clicked()
            {
                run = Some(false);
            }
        });
        match &window.result {
            Some(Ok(reply)) => {
                ui.label(
                    RichText::new(if window.built {
                        crate::i18n::tr("Criado (Ctrl+Z desfaz):")
                    } else {
                        crate::i18n::tr("Prévia:")
                    })
                    .strong(),
                );
                egui::Grid::new("cabinet-modules").striped(true).show(ui, |ui| {
                    for m in reply["modules"].as_array().into_iter().flatten() {
                        ui.label(role_name(m[1].as_str().unwrap_or_default()));
                        let cm = |v: &serde_json::Value| {
                            let text = format!("{v} cm");
                            if crate::i18n::decimal_comma() {
                                text.replace('.', ",")
                            } else {
                                text
                            }
                        };
                        ui.label(cm(&m[2]));
                        ui.label(cm(&m[3]));
                        ui.end_row();
                    }
                });
                for note in reply["notes"].as_array().into_iter().flatten() {
                    ui.label(
                        RichText::new(format!("{} {}", icon::INFO, note.as_str().unwrap_or_default()))
                            .weak(),
                    );
                }
            }
            Some(Err(err)) => {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 50), format!("⚠ {err}"));
            }
            None => {}
        }
    });
    if let Some(dry) = run {
        let params = window.params(dry);
        let reply = newera_joinery::cabinet_run(&mut app.document.write(), &params);
        window.built = !dry && reply.is_ok();
        window.result = Some(reply);
    }
    if open {
        app.cabinets = Some(window);
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use newera_core::{Command, Document, Point2, SharedDocument, Wall};

    use super::*;

    #[test]
    fn embeds_the_selected_piece_in_the_selected_countertop() {
        let mut doc = Document::default();
        let build = newera_joinery::Build::Countertop(newera_joinery::CountertopParams::default());
        let output = newera_joinery::generate(&build).unwrap();
        let top = doc.new_furniture_id();
        let group = {
            let mut next = || doc.new_furniture_id();
            newera_joinery::assemble(
                &build,
                &output,
                top,
                Point2::new(200.0, 100.0),
                0.0,
                0.0,
                &mut next,
            )
        };
        doc.execute(Command::insert(group)).unwrap();
        let cooktop = doc.new_furniture_id();
        doc.execute(Command::insert(newera_core::Furniture {
            id: cooktop,
            catalog: "cooktop".into(),
            name: "Cooktop".into(),
            position: Point2::new(230.0, 100.0),
            width: 60.0,
            depth: 50.0,
            height: 6.0,
            ..newera_core::Furniture::default()
        }))
        .unwrap();
        let document = SharedDocument::new(doc);
        let shared = document.clone();
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 900.0))
            .build_eframe(move |cc| NewEraApp::new(cc, document, None));
        h.run_steps(3);
        h.state_mut().selection = [
            newera_core::ElementId::from(top),
            newera_core::ElementId::from(cooktop),
        ]
        .into_iter()
        .collect();
        h.run_steps(2);
        h.get_by_label("Planta").click();
        h.run_steps(2);
        h.get_by_label_contains("Embutir peça").click();
        h.run_steps(3);
        let home = shared.read();
        let host = home.home().furniture.iter().find(|f| f.id == top).unwrap();
        assert!(host.children.iter().any(|c| c.id == cooktop), "embedded");
        assert!(!home.home().furniture.iter().any(|f| f.id == cooktop));
    }

    #[test]
    fn previews_then_builds_the_cabinets_of_a_wall() {
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
        let wall = first.unwrap();
        let document = SharedDocument::new(doc);
        let shared = document.clone();
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 900.0))
            .build_eframe(move |cc| NewEraApp::new(cc, document, None));
        h.run_steps(3);
        h.state_mut().cabinets = Some(CabinetsWindow::new(wall));
        h.run_steps(3);
        h.get_by_label_contains("Pré-visualizar").click();
        h.run_steps(3);
        let before = shared.read().home().furniture.len();
        assert_eq!(before, 0, "a preview changes nothing");
        assert!(matches!(
            &h.state().cabinets.as_ref().unwrap().result,
            Some(Ok(reply)) if !reply["modules"].as_array().unwrap().is_empty()
        ));
        h.get_by_label_contains("Criar armários").click();
        h.run_steps(3);
        assert!(shared.read().home().furniture.len() > 3);
        assert!(h.state().cabinets.as_ref().unwrap().built);
    }
}
