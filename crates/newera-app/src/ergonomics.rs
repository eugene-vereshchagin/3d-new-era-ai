//! "Ergonomia": the habitability review of the plan for the people who live
//! there, kept up to date while the plan changes. Findings select what they
//! are about, and those with a checked fix apply it in one click.

use crate::icon;
use eframe::egui::{self, Color32, RichText};
use newera_core::standards::{self, Tier};
use newera_core::{Command, Element, ElementId, FurnitureId, ops};
use newera_ergonomics::{Profile, Report, Severity};

use crate::app::NewEraApp;

/// Who lives there, and the last review.
#[derive(Debug, Default)]
pub(crate) struct ErgonomicsWindow {
    pub(crate) profile: Profile,
    /// `(document revision, profile it was made for, report)`.
    pub(crate) report: Option<(u64, Profile, Report)>,
}

/// Elements named in a finding's place, e.g. `Cama de casal f12, Armário f20`.
fn ids_in(place: &str) -> Vec<ElementId> {
    place
        .split([' ', ','])
        .filter_map(|word| word.parse::<ElementId>().ok())
        .collect()
}

/// Applies a fix from the review: `move` or `update` tool arguments.
fn apply(doc: &mut newera_core::Document, fix: &serde_json::Value) -> Result<(), String> {
    match fix["tool"].as_str() {
        Some("move") => {
            let ids: Vec<ElementId> = fix["ids"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str()?.parse().ok())
                .collect();
            let (dx, dy) = (
                fix["dx"].as_f64().unwrap_or_default(),
                fix["dy"].as_f64().unwrap_or_default(),
            );
            ops::translate(doc, &ids, dx, dy, false).map_err(|e| e.to_string())
        }
        Some("update") => {
            let mut commands = Vec::new();
            for item in fix["items"].as_array().into_iter().flatten() {
                let id: FurnitureId = item["id"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .ok_or("fix without a valid id")?;
                let Some(mut piece) = doc.home().furniture.iter().find(|f| f.id == id).cloned()
                else {
                    return Err(format!("{id} not found"));
                };
                if let (Some(right), Some(opening)) =
                    (item["hinge_right"].as_bool(), piece.opening.as_mut())
                {
                    opening.hinge_right = right;
                }
                commands.push(Command::update(Element::Furniture(piece)));
            }
            doc.execute(Command::Batch { commands })
                .map_err(|e| e.to_string())
        }
        _ => Err("unknown fix".into()),
    }
}

/// The colour of a reliability tier: what obliges reads like an error, what
/// only measures or advises reads quieter.
fn tier_look(tier: Tier) -> Color32 {
    match tier {
        Tier::A => Color32::from_rgb(200, 60, 50),
        Tier::B | Tier::C => Color32::from_rgb(130, 120, 100),
        Tier::D | Tier::E => Color32::from_rgb(70, 130, 180),
    }
}

fn severity_look(severity: Severity) -> (&'static str, Color32) {
    match severity {
        Severity::Erro => (icon::X_CIRCLE, Color32::from_rgb(200, 60, 50)),
        Severity::Alerta => (icon::WARNING, Color32::from_rgb(210, 140, 30)),
        Severity::Dica => (icon::LIGHTBULB, Color32::from_rgb(70, 130, 180)),
    }
}

/// Shows the window while it is open.
#[allow(clippy::too_many_lines)]
pub(crate) fn show(app: &mut NewEraApp, ctx: &egui::Context) {
    let Some(mut window) = app.ergonomics.take() else {
        return;
    };
    let revision = app.document.read().revision();
    if window
        .report
        .as_ref()
        .is_some_and(|(rev, _, _)| *rev != revision)
    {
        // Undo and MCP edits must also refresh an already open review panel.
        let home = app.document.read();
        window.profile = Profile {
            // The building code belongs to the project, on the compass, so it
            // survives saving and agrees with what `set_home(city=…)` wrote.
            city: home.home().compass.city.clone(),
            ..Profile::of(home.home())
        };
    }
    let original_profile = window.profile.clone();
    let stale = window
        .report
        .as_ref()
        .is_none_or(|(rev, profile, _)| *rev != revision || *profile != window.profile);
    if stale {
        let report = newera_ergonomics::review(app.document.read().home(), &window.profile);
        window.report = Some((revision, window.profile.clone(), report));
    }
    let mut open = true;
    let mut select: Option<Vec<ElementId>> = None;
    let mut fix: Option<serde_json::Value> = None;
    egui::Window::new(format!("{} {}", icon::PERSON_ARMS_SPREAD, crate::i18n::tr("Ergonomia")))
        .open(&mut open)
        .default_width(520.0)
        .default_height(560.0)
        .show(ctx, |ui| {
            let p = &mut window.profile;
            egui::Grid::new("ergonomics-profile")
                .num_columns(4)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(crate::i18n::tr("Moradores"));
                    ui.add(egui::DragValue::new(&mut p.occupants).range(1..=20));
                    ui.label(crate::i18n::tr("Crianças"));
                    ui.add(egui::DragValue::new(&mut p.children).range(0..=p.occupants));
                    ui.end_row();
                    ui.label(crate::i18n::tr("Idosos"));
                    ui.add(egui::DragValue::new(&mut p.elderly).range(0..=p.occupants));
                    ui.label(crate::i18n::tr("Altura de quem cozinha"));
                    let mut stature = p.stature.unwrap_or(165.0);
                    if ui
                        .add(egui::DragValue::new(&mut stature).range(130.0..=210.0).suffix(" cm"))
                        .changed()
                    {
                        p.stature = Some(stature);
                    }
                    ui.end_row();
                });
            ui.checkbox(
                &mut p.wheelchair,
                crate::i18n::tr("Alguém usa cadeira de rodas (NBR 9050)"),
            );
            ui.horizontal(|ui| {
                ui.label(crate::i18n::tr("Código de obras"));
                let chosen = p.city.as_deref().and_then(standards::municipal).map_or_else(
                    || crate::i18n::tr("Não informado"),
                    |c| c.label,
                );
                egui::ComboBox::from_id_salt("ergonomics-city")
                    .selected_text(chosen)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut p.city,
                            None,
                            crate::i18n::tr("Não informado"),
                        );
                        for code in standards::cities() {
                            ui.selectable_value(
                                &mut p.city,
                                Some(code.city.to_owned()),
                                code.label,
                            )
                            .on_hover_text(code.source);
                        }
                    })
                    .response
                    .on_hover_text(crate::i18n::tr(
                        "Entre a norma e a lei do município, prevalece o mais restritivo.",
                    ));
            });
            ui.checkbox(&mut p.scope.electrical, crate::i18n::tr("Incluir elétrica na nota"));
            ui.checkbox(&mut p.scope.plumbing, crate::i18n::tr("Incluir hidráulica na nota"));
            ui.label(crate::i18n::tr("Pendências fora do escopo continuam visíveis. A nota não certifica o projeto."));
            ui.separator();
            let Some((_, _, report)) = &window.report else {
                return;
            };
            let color = match report.score {
                80.. => Color32::from_rgb(60, 150, 80),
                50..80 => Color32::from_rgb(210, 140, 30),
                _ => Color32::from_rgb(200, 60, 50),
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(report.score.to_string()).size(34.0).strong().color(color));
                ui.vertical(|ui| {
                    ui.label(RichText::new(crate::i18n::tr("nota de habitabilidade")).weak());
                    let c = &report.capacity;
                    ui.label(format!(
                        "{} {} · {} {} · {} {} · {} {} · {} cm {}",
                        c.beds,
                        crate::i18n::tr("lugares para dormir"),
                        c.bathrooms,
                        crate::i18n::tr("banheiros"),
                        c.dining_seats,
                        crate::i18n::tr("à mesa"),
                        c.living_seats,
                        crate::i18n::tr("na sala"),
                        c.wardrobe_cm,
                        crate::i18n::tr("de guarda-roupa"),
                    ));
                });
            });
            if report.refs.is_empty() {
                ui.weak(crate::i18n::tr(
                    "Referências brasileiras onde existe norma; áreas e janelas variam com o código de obras do município.",
                ));
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.weak(crate::i18n::tr("Fontes desta revisão:"));
                    for (n, source) in report.refs.iter().enumerate() {
                        if n > 0 {
                            ui.weak("·");
                        }
                        let text = RichText::new(source.title).weak().size(11.5);
                        match source.url {
                            Some(url) => {
                                ui.hyperlink_to(text, url).on_hover_text(source.scope);
                            }
                            None => {
                                ui.label(text).on_hover_text(source.scope);
                            }
                        }
                    }
                });
            }
            ui.separator();
            if report.findings.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        icon::CHECK_CIRCLE,
                        crate::i18n::tr("Nada a apontar para esses moradores.")
                    ))
                    .color(color),
                );
            }
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                for (k, finding) in report.findings.iter().enumerate() {
                    let (glyph, tint) = severity_look(finding.severity);
                    ui.push_id(k, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new(glyph).color(tint).size(16.0));
                            ui.vertical(|ui| {
                                let ids = ids_in(&finding.place);
                                let place = RichText::new(&finding.place).strong();
                                if ids.is_empty() {
                                    ui.label(place);
                                } else if ui
                                    .link(place)
                                    .on_hover_text(crate::i18n::tr("Selecionar na planta"))
                                    .clicked()
                                {
                                    select = Some(ids);
                                }
                                if !report.scope.includes(finding) {
                                    ui.weak(crate::i18n::tr("Fora do escopo da nota"));
                                }
                                ui.label(
                                    finding
                                        .message
                                        .in_language(crate::i18n::findings_language()),
                                );
                                if let Some(source) =
                                    finding.reference.and_then(standards::standard)
                                {
                                    // The chip carries the code; the whole
                                    // title and what it governs are a hover away.
                                    let name = source
                                        .title
                                        .split_once(" — ")
                                        .map_or(source.title, |(head, _)| head);
                                    // How much it obliges depends on where the
                                    // project is, so the letter comes from the
                                    // place this review was conducted at.
                                    let tier = source.force(&report.place);
                                    let chip =
                                        RichText::new(format!("{} · {name}", tier.letter()))
                                            .color(tier_look(tier))
                                            .size(11.0);
                                    let hint = format!(
                                        "{} ({}) · {}\n{}",
                                        source.title,
                                        source.edition,
                                        crate::i18n::tr(tier.what()),
                                        source.scope
                                    );
                                    match source.url {
                                        Some(url) => {
                                            ui.hyperlink_to(chip, url).on_hover_text(hint);
                                        }
                                        None => {
                                            ui.label(chip).on_hover_text(hint);
                                        }
                                    }
                                }
                                if let Some(f) = &finding.fix
                                    && ui
                                        .button(format!(
                                            "{} {}",
                                            icon::MAGIC_WAND,
                                            crate::i18n::tr("Aplicar correção")
                                        ))
                                        .clicked()
                                {
                                    fix = Some(f.clone());
                                }
                            });
                        });
                    });
                    ui.add_space(4.0);
                }
            });
        });
    if original_profile != window.profile {
        let mut doc = app.document.write();
        // The city is the project's, not the panel's: it goes on the compass,
        // where saving keeps it and every review reads the same one. Picking a
        // code here used to be forgotten the moment the project was saved.
        if original_profile.city != window.profile.city {
            let compass = newera_core::Compass {
                city: window.profile.city.clone(),
                ..doc.home().compass.clone()
            };
            let _ = doc.execute(Command::SetCompass { compass });
        }
        let mut properties = doc.home().properties.clone();
        let kept = Profile {
            city: None,
            ..window.profile.clone()
        };
        if let Ok(value) = serde_json::to_string(&kept) {
            properties.insert(newera_ergonomics::PEOPLE.to_owned(), value);
            let _ = doc.execute(Command::SetProperties { properties });
        }
    }
    if let Some(ids) = select {
        app.selection = ids.into_iter().collect();
    }
    if let Some(f) = fix {
        let result = apply(&mut app.document.write(), &f);
        match result {
            Ok(()) => {
                app.set_status(crate::i18n::tr("Correção aplicada (Ctrl+Z desfaz).").to_owned());
            }
            Err(err) => app.set_status(format!("⚠ {err}")),
        }
    }
    if open {
        app.ergonomics = Some(window);
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use newera_core::{Document, Furniture, Opening, Point2, Room, RoomId, SharedDocument, Wall};

    use super::*;

    #[test]
    fn reviews_the_plan_and_applies_a_fix() {
        let mut doc = Document::default();
        let corners = [(0.0, 0.0), (300.0, 0.0), (300.0, 240.0), (0.0, 240.0)];
        for k in 0..4 {
            let (a, b) = (corners[k], corners[(k + 1) % 4]);
            let id = doc.new_wall_id();
            doc.execute(Command::insert(Wall::new(
                id,
                Point2::new(a.0, a.1),
                Point2::new(b.0, b.1),
            )))
            .unwrap();
        }
        let room = doc.new_room_id();
        doc.execute(Command::insert(Room::new(
            room,
            "Quarto",
            vec![
                Point2::new(7.5, 7.5),
                Point2::new(292.5, 7.5),
                Point2::new(292.5, 232.5),
                Point2::new(7.5, 232.5),
            ],
        )))
        .unwrap();
        // A door swinging into the room and a dresser right where it opens.
        let door = doc.new_furniture_id();
        doc.execute(Command::insert(Furniture {
            id: door,
            catalog: "door".into(),
            name: "Porta".into(),
            position: Point2::new(250.0, 240.0),
            angle: 180.0,
            width: 80.0,
            depth: 15.0,
            height: 210.0,
            opening: Some(Opening::default()),
            ..Furniture::default()
        }))
        .unwrap();
        let dresser = doc.new_furniture_id();
        doc.execute(Command::insert(Furniture {
            id: dresser,
            catalog: "dresser".into(),
            name: "Cômoda".into(),
            position: Point2::new(250.0, 190.0),
            angle: 180.0,
            width: 60.0,
            depth: 40.0,
            height: 85.0,
            ..Furniture::default()
        }))
        .unwrap();
        let _ = RoomId(0);
        let document = SharedDocument::new(doc);
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 900.0))
            .build_eframe(move |cc| NewEraApp::new(cc, document, None));
        h.run_steps(3);
        h.state_mut().ergonomics = Some(ErgonomicsWindow::default());
        h.run_steps(3);
        let report = &h
            .state()
            .ergonomics
            .as_ref()
            .unwrap()
            .report
            .as_ref()
            .unwrap()
            .2;
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.message.contains("A folha da porta bate")),
            "{report:#?}"
        );
        // The source of a finding is on screen, with its tier, and the
        // window lists the standards this very review leaned on.
        let cited = report
            .findings
            .iter()
            .find_map(|f| f.reference.and_then(standards::standard))
            .expect("some finding stands on a published source");
        let chip = format!(
            "{} · {}",
            cited.force(&report.place).letter(),
            cited
                .title
                .split_once(" — ")
                .map_or(cited.title, |(head, _)| head)
        );
        assert!(
            h.query_all_by_label_contains(&chip).next().is_some(),
            "the chip `{chip}` is on screen"
        );
        h.get_by_label_contains("Fontes desta revisão:");
        let before = report.score;
        h.get_by_label_contains("Aplicar correção").click();
        h.run_steps(3);
        let report = &h
            .state()
            .ergonomics
            .as_ref()
            .unwrap()
            .report
            .as_ref()
            .unwrap()
            .2;
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.message.contains("A folha da porta bate")),
            "{report:#?}"
        );
        assert!(report.score > before);
        h.get_by_label("Incluir elétrica na nota").click();
        h.run_steps(3);
        assert!(
            !Profile::of(h.state().document.read().home())
                .scope
                .electrical
        );
        assert!(
            !h.state()
                .ergonomics
                .as_ref()
                .unwrap()
                .report
                .as_ref()
                .unwrap()
                .2
                .scope
                .electrical
        );
        h.state_mut().document.write().undo().unwrap();
        assert!(
            Profile::of(h.state().document.read().home())
                .scope
                .electrical
        );
        h.run_steps(3);
        assert!(
            h.state()
                .ergonomics
                .as_ref()
                .unwrap()
                .profile
                .scope
                .electrical
        );
    }
}
