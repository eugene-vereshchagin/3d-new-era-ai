//! Plan variants as tabs: several versions of the same project, each with
//! its own history, to try alternatives and compare them side by side.

use crate::icon;
use eframe::egui::{self, RichText};
use newera_core::Home;

use crate::app::NewEraApp;
use crate::dialogs::Dialog;

/// Numbers that make versions comparable at a glance.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VariantStats {
    pub(crate) name: String,
    pub(crate) active: bool,
    pub(crate) walls: usize,
    pub(crate) rooms: usize,
    /// Total room area, cm².
    pub(crate) area: f64,
    pub(crate) furniture: usize,
    pub(crate) issues: usize,
}

pub(crate) fn stats(name: &str, active: bool, home: &Home) -> VariantStats {
    VariantStats {
        name: name.to_owned(),
        active,
        walls: home.walls.len(),
        rooms: home.rooms.len(),
        area: home.rooms.iter().map(newera_core::Room::area).sum(),
        furniture: home.furniture.len(),
        // Built-in pieces, layered storeys and accepted pairs are not counted:
        // the badge only ever means "this many things need fixing".
        issues: newera_core::check_layout(home)
            .iter()
            .filter(|i| i.is_pending(home))
            .count(),
    }
}

pub(crate) fn bar(app: &mut NewEraApp, ui: &mut egui::Ui) {
    let infos = app.document.read().variant_infos();
    let count = infos.len();
    let mut switch_to = None;
    let mut duplicate = false;
    let mut blank = false;
    let mut close = None;

    // On a phone the row has room for the sheets and one button: the rest —
    // discipline, layers, storeys — goes behind that button instead of being
    // drawn on top of the tabs.
    let narrow = ui.ctx().content_rect().width() < crate::app::NARROW;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        egui::ScrollArea::horizontal()
            .id_salt("variant_tabs")
            .max_width((ui.available_width() - if narrow { 92.0 } else { 190.0 }).max(60.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for info in &infos {
                        if let Some((index, name)) = &mut app.renaming_variant
                            && *index == info.index
                        {
                            let edit =
                                ui.add(egui::TextEdit::singleline(name).desired_width(120.0));
                            edit.request_focus();
                            if edit.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                let (index, name) = app.renaming_variant.take().expect("renaming");
                                app.run(|doc| doc.rename_variant(index, name.trim()));
                            }
                            continue;
                        }
                        let text = RichText::new(format!("{} {}", icon::FILE_TEXT, info.name));
                        let tab = ui.add(egui::Button::selectable(
                            info.active,
                            if info.active { text.strong() } else { text },
                        ));
                        // The sheet in front is underlined, the way a tab in
                        // a drawing set is marked on its edge.
                        if info.active {
                            let t = crate::theme::of(ui.visuals());
                            let rect = tab.rect;
                            ui.painter().hline(
                                (rect.left() + 3.0)..=(rect.right() - 3.0),
                                rect.bottom() - 1.0,
                                egui::Stroke::new(2.0, t.accent),
                            );
                        }
                        if tab.clicked() && !info.active {
                            switch_to = Some(info.index);
                        }
                        if tab.double_clicked() {
                            app.renaming_variant = Some((info.index, info.name.clone()));
                        }
                        tab.context_menu(|ui| {
                            if ui
                                .button(format!(
                                    "{} {}",
                                    icon::PENCIL_SIMPLE,
                                    crate::i18n::tr("Renomear")
                                ))
                                .clicked()
                            {
                                app.renaming_variant = Some((info.index, info.name.clone()));
                                ui.close();
                            }
                            if ui
                                .button(format!("{} {}", icon::COPY, crate::i18n::tr("Duplicar")))
                                .clicked()
                            {
                                switch_to = Some(info.index);
                                duplicate = true;
                                ui.close();
                            }
                            if ui
                                .add_enabled(
                                    count > 1,
                                    egui::Button::new(format!(
                                        "{} {}",
                                        icon::X,
                                        crate::i18n::tr("Fechar versão")
                                    )),
                                )
                                .clicked()
                            {
                                close = Some(info.index);
                                ui.close();
                            }
                        });
                    }
                });
            });
        ui.menu_button(RichText::new(icon::PLUS).size(16.0), |ui| {
            if ui
                .add(
                    egui::Button::new(format!(
                        "{} {}",
                        icon::COPY,
                        crate::i18n::tr("Duplicar versão atual")
                    ))
                    .shortcut_text("Ctrl+T"),
                )
                .clicked()
            {
                duplicate = true;
                ui.close();
            }
            if ui
                .button(format!(
                    "{} {}",
                    icon::FILE_PLUS,
                    crate::i18n::tr("Nova versão em branco")
                ))
                .clicked()
            {
                blank = true;
                ui.close();
            }
        })
        .response
        .on_hover_text(crate::i18n::tr("Nova versão da planta"));
        let compare = count > 1
            && !narrow
            && ui
                .button(format!(
                    "{} {}",
                    icon::CHART_BAR,
                    crate::i18n::tr("Comparar")
                ))
                .on_hover_text(crate::i18n::tr("Comparar as versões"))
                .clicked();
        let mut compare_narrow = false;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if narrow {
                ui.menu_button(RichText::new(icon::SLIDERS_HORIZONTAL).size(16.0), |ui| {
                    ui.set_min_width(230.0);
                    disciplines(app, ui);
                    ui.separator();
                    layers(app, ui);
                    ui.separator();
                    levels(app, ui);
                    if count > 1 {
                        ui.separator();
                        if ui
                            .button(format!(
                                "{} {}",
                                icon::CHART_BAR,
                                crate::i18n::tr("Comparar as versões")
                            ))
                            .clicked()
                        {
                            compare_narrow = true;
                            ui.close();
                        }
                    }
                })
                .response
                .on_hover_text(crate::i18n::tr("Disciplina, camadas e andares"));
                return;
            }
            levels(app, ui);
            ui.separator();
            layers(app, ui);
            ui.separator();
            disciplines(app, ui);
        });
        let compare = compare || compare_narrow;
        if compare {
            app.open_compare();
        }
    });

    if let Some(index) = switch_to {
        app.run(|doc| doc.switch_variant(index));
        app.after_variant_change();
    }
    if duplicate || blank {
        app.run(|doc| {
            doc.add_variant(None, duplicate);
            Ok(())
        });
        app.after_variant_change();
    }
    if let Some(index) = close {
        let name = infos[index].name.clone();
        app.set_dialog(Dialog::ConfirmCloseVariant { index, name });
    }
}

/// Layers of the plan — the electrical and plumbing projects, lighting,
/// appliances, joinery — shown or hidden on the drawing and in the 3D.
fn layers(app: &mut NewEraApp, ui: &mut egui::Ui) {
    use newera_core::{Discipline, PlanLayer};
    let (hidden_disciplines, discipline_counts) = {
        let doc = app.document.read();
        let home = doc.home();
        let count = |d: Discipline| {
            home.furniture
                .iter()
                .flat_map(|top| {
                    top.flatten()
                        .into_iter()
                        .filter(move |leaf| leaf.discipline.or(top.discipline) == Some(d))
                })
                .count()
                + home
                    .polylines
                    .iter()
                    .filter(|l| l.discipline == Some(d))
                    .count()
        };
        (
            home.hidden_disciplines.clone(),
            [count(Discipline::Electrical), count(Discipline::Plumbing)],
        )
    };
    let mut discipline_toggle = None;
    let (hidden, counts) = {
        let doc = app.document.read();
        let home = doc.home();
        let counts: Vec<usize> = PlanLayer::ALL
            .iter()
            .map(|layer| {
                home.furniture
                    .iter()
                    .flat_map(|top| {
                        top.visible_leaves()
                            .into_iter()
                            .map(move |leaf| newera_core::layer_in_group(top, leaf))
                    })
                    .filter(|l| *l == Some(*layer))
                    .count()
            })
            .collect();
        (home.hidden_layers.clone(), counts)
    };
    let mut toggle = None;
    let mut all_3d = None;
    let showing_all = app.document.read().home().show_all_in_3d;
    let title = if hidden.is_empty() {
        format!("{} {}", icon::STACK, crate::i18n::tr("Camadas"))
    } else {
        format!(
            "{} {} ({})",
            icon::STACK,
            crate::i18n::tr("Camadas"),
            PlanLayer::ALL.len() - hidden.len()
        )
    };
    ui.menu_button(title, |ui| {
        // A whole project at once: hiding the electrical one takes its
        // outlets, switches, cables, circuit numbers and every lamp away.
        for (d, count) in [Discipline::Electrical, Discipline::Plumbing]
            .into_iter()
            .zip(discipline_counts)
        {
            let label = match d {
                Discipline::Electrical => format!(
                    "{} {}",
                    icon::LIGHTNING,
                    crate::i18n::tr("Elétrica (tomadas, luz, cabos)")
                ),
                Discipline::Plumbing => {
                    format!("{} {}", icon::DROP, crate::i18n::tr("Hidráulica"))
                }
            };
            let mut visible = !hidden_disciplines.contains(&d);
            if ui
                .checkbox(&mut visible, format!("{label} · {count}"))
                .changed()
            {
                discipline_toggle = Some((d, visible));
            }
        }
        ui.separator();
        for (layer, count) in PlanLayer::ALL.into_iter().zip(counts) {
            let label = match layer {
                PlanLayer::Lighting => {
                    format!("{} {}", icon::LIGHTBULB, crate::i18n::tr("Iluminação"))
                }
                PlanLayer::Appliances => {
                    format!("{} {}", icon::OVEN, crate::i18n::tr("Eletrodomésticos"))
                }
                PlanLayer::Joinery => format!("{} {}", icon::HAMMER, crate::i18n::tr("Marcenaria")),
            };
            let mut visible = !hidden.contains(&layer);
            if ui
                .checkbox(&mut visible, format!("{label} · {count}"))
                .changed()
            {
                toggle = Some((layer, visible));
            }
        }
        ui.separator();
        let mut all = showing_all;
        if ui
            .checkbox(
                &mut all,
                format!("{} {}", icon::CUBE, crate::i18n::tr("Mostrar tudo no 3D")),
            )
            .on_hover_text(crate::i18n::tr(
                "Ligado, o 3D mostra a obra inteira; desligado, esconde o que a planta esconde",
            ))
            .changed()
        {
            all_3d = Some(all);
        }
    })
    .response
    .on_hover_text(crate::i18n::tr("Mostrar ou esconder na planta e no 3D"));
    if let Some((layer, visible)) = toggle {
        app.document.write().set_layer_visible(layer, visible);
        app.plan.invalidate_scene();
    }
    if let Some((d, visible)) = discipline_toggle {
        app.document.write().set_discipline_visible(d, visible);
        app.plan.invalidate_scene();
    }
    if let Some(all) = all_3d {
        app.document.write().set_show_all_in_3d(all);
    }
}

/// Architecture / electrical / plumbing switch, visibility and quantities.
fn disciplines(app: &mut NewEraApp, ui: &mut egui::Ui) {
    use newera_core::Discipline;
    let (active, hidden) = {
        let doc = app.document.read();
        let home = doc.home();
        (home.active_discipline, home.hidden_disciplines.clone())
    };
    let label = |d: Option<Discipline>| match d {
        None => format!("{} {}", icon::HOUSE_LINE, crate::i18n::tr("Arquitetura")),
        Some(Discipline::Electrical) => {
            format!("{} {}", icon::LIGHTNING, crate::i18n::tr("Elétrica"))
        }
        Some(Discipline::Plumbing) => format!("{} {}", icon::DROP, crate::i18n::tr("Hidráulica")),
    };
    let mut choice = None;
    let mut toggle = None;
    let mut quantities = false;
    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
    let mut electrical = false;
    egui::ComboBox::from_id_salt("discipline_select")
        .selected_text(label(active))
        .show_ui(ui, |ui| {
            for d in [
                None,
                Some(Discipline::Electrical),
                Some(Discipline::Plumbing),
            ] {
                if ui.selectable_label(active == d, label(d)).clicked() {
                    choice = Some(d);
                }
            }
            ui.separator();
            for d in Discipline::ALL {
                let mut visible = !hidden.contains(&d);
                if ui
                    .checkbox(
                        &mut visible,
                        crate::i18n::fill(
                            "Mostrar {}",
                            &[&crate::i18n::tr(d.name()).to_lowercase()],
                        ),
                    )
                    .changed()
                {
                    toggle = Some((d, visible));
                }
            }
            ui.separator();
            if ui
                .button(format!(
                    "{} {}",
                    icon::LIST_NUMBERS,
                    crate::i18n::tr("Quantitativos")
                ))
                .clicked()
            {
                quantities = true;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if ui
                .button(format!(
                    "{} {}",
                    icon::LIGHTNING,
                    crate::i18n::tr("Quadro de cargas e NBR 5410")
                ))
                .clicked()
            {
                electrical = true;
            }
        })
        .response
        .on_hover_text(crate::i18n::tr(
            "Projeto em edição: novos símbolos e linhas vão para ele",
        ));
    if let Some(d) = choice {
        // Architecture shows architecture; a project shows itself. The plan
        // and the 3D follow.
        app.document.write().choose_view(d);
        app.plan.invalidate_scene();
    }
    if let Some((d, visible)) = toggle {
        app.document.write().set_discipline_visible(d, visible);
    }
    if electrical {
        app.set_dialog(Dialog::Electrical);
    }
    if quantities {
        let rows = quantity_rows(app.document.read().home());
        app.set_dialog(Dialog::Quantities(rows));
    }
}

/// One line of a bill of quantities.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuantityRow {
    pub(crate) discipline: newera_core::Discipline,
    pub(crate) name: String,
    pub(crate) count: usize,
}

/// Counts technical points per discipline and catalog item, plus the
/// length of each discipline's lines.
pub(crate) fn quantity_rows(home: &Home) -> Vec<QuantityRow> {
    let mut counts: std::collections::BTreeMap<(newera_core::Discipline, String), usize> =
        std::collections::BTreeMap::new();
    for top in &home.furniture {
        for piece in top.flatten() {
            if let Some(d) = piece.discipline.or(top.discipline) {
                *counts
                    .entry((d, newera_catalog::quantity_name(piece)))
                    .or_default() += 1;
            }
        }
    }
    counts
        .into_iter()
        .map(|((discipline, name), count)| QuantityRow {
            discipline,
            name,
            count,
        })
        .collect()
}

/// Storey selector: pick, add, edit and delete levels.
fn levels(app: &mut NewEraApp, ui: &mut egui::Ui) {
    let (levels, current) = {
        let doc = app.document.read();
        let home = doc.home();
        let levels: Vec<newera_core::Level> = home.sorted_levels().into_iter().cloned().collect();
        (levels, home.current_level())
    };
    if ui
        .button(format!("{} {}", icon::PLUS, crate::i18n::tr("Andar")))
        .on_hover_text(crate::i18n::tr("Adicionar um andar acima do mais alto"))
        .clicked()
    {
        app.run(|doc| newera_core::ops::add_level(doc, None, None).map(|_| ()));
        app.after_variant_change();
    }
    if levels.is_empty() {
        ui.weak(format!("{} {}", icon::STACK, crate::i18n::tr("Térreo")));
        return;
    }
    let current_name = levels
        .iter()
        .find(|l| Some(l.id) == current)
        .map_or_else(String::new, |l| l.name.clone());
    let mut select = None;
    egui::ComboBox::from_id_salt("level_select")
        .selected_text(format!("{} {current_name}", icon::STACK))
        .show_ui(ui, |ui| {
            for level in levels.iter().rev() {
                let label = format!(
                    "{} · {}",
                    level.name,
                    app.unit().format_length(level.elevation)
                );
                if ui
                    .selectable_label(Some(level.id) == current, label)
                    .clicked()
                {
                    select = Some(level.id);
                }
            }
            ui.separator();
            if ui
                .button(format!(
                    "{} {}",
                    icon::PENCIL_SIMPLE,
                    crate::i18n::tr("Editar andar…")
                ))
                .clicked()
                && let Some(id) = current
            {
                app.open_modify(&[id.into()]);
            }
            if ui
                .add_enabled(
                    levels.len() > 1,
                    egui::Button::new(format!(
                        "{} {}",
                        icon::TRASH,
                        crate::i18n::tr("Excluir andar")
                    )),
                )
                .clicked()
                && let Some(level) = levels.iter().find(|l| Some(l.id) == current)
            {
                app.set_dialog(Dialog::ConfirmDeleteLevel {
                    id: level.id,
                    name: level.name.clone(),
                });
            }
        });
    if let Some(id) = select {
        app.document.write().select_level(Some(id));
        app.after_variant_change();
    }
}

#[cfg(test)]
mod tests {
    use newera_core::{Command, Document, Point2, Wall};

    use super::*;

    #[test]
    fn stats_summarize_a_home() {
        let mut doc = Document::default();
        let wall = Wall::new(
            doc.new_wall_id(),
            Point2::new(0.0, 0.0),
            Point2::new(300.0, 0.0),
        );
        doc.execute(Command::insert(wall)).unwrap();
        let s = stats("A", true, doc.home());
        assert_eq!((s.walls, s.rooms, s.furniture, s.issues), (1, 0, 0, 0));
    }
}
