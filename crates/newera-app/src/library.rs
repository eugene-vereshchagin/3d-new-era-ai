//! The model library in the catalog: the approved versions kept on this
//! machine (`newera_core::library`), listed beside the built-in pieces and
//! placed with one click, so a model published once is taken by hand like
//! any sofa of the catalog instead of imported again from its file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui;
use newera_catalog::Category;
use newera_core::library::{self, Entry};
use newera_core::{Command, ElementId};

use crate::app::NewEraApp;
use crate::view::plan::Tool;

/// Pictures read from disk in one frame: a section of hundreds opens at once
/// and fills its pictures in over the next frames instead of freezing.
const ICONS_PER_FRAME: usize = 8;

/// Side of a row's picture, px, as it is kept in memory.
const ICON_PX: u32 = 64;

/// Where the window reads the library from. Tests read none: what a
/// developer published on their own machine is no part of what they check.
pub(crate) fn root() -> PathBuf {
    if cfg!(test) {
        PathBuf::new()
    } else {
        library::dir()
    }
}

/// One row of the catalog: what it shows of an entry, and the name it is
/// placed by.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Listed {
    pub(crate) name: String,
    pub(crate) title: String,
    /// `[w, d, h]` cm.
    pub(crate) size: [f64; 3],
    icon: Option<PathBuf>,
}

impl Listed {
    /// The row an entry is shown as: its title, or its name when it has none,
    /// its size and its picture.
    fn of(entry: &Entry) -> Self {
        Self {
            name: entry.name.clone(),
            title: entry.title.clone().unwrap_or_else(|| entry.name.clone()),
            size: entry.size,
            icon: entry.icon.as_deref().map(PathBuf::from),
        }
    }
}

/// The library as the catalog lists it: the latest version of each entry.
pub(crate) struct Shelf {
    root: PathBuf,
    /// Each entry with the built-in section its `category` names, resolved
    /// once when the library is read: the panel asks every section for its
    /// entries each frame.
    items: Vec<(Entry, PathBuf, Option<Category>)>,
    /// Read since the window last came back into focus. The library changes
    /// from outside — an agent publishes, a folder is copied in — while the
    /// person is in another window, so coming back is when to look again.
    fresh: bool,
    was_focused: bool,
    icons: HashMap<PathBuf, Option<egui::TextureHandle>>,
    icons_this_frame: usize,
}

impl Shelf {
    /// A shelf over the library at `root`, still to be read: the first
    /// [`Shelf::refresh`] reads it.
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            items: Vec::new(),
            fresh: false,
            was_focused: true,
            icons: HashMap::new(),
            icons_this_frame: 0,
        }
    }

    /// Reads the library again when it has not been read since the window
    /// came back into focus. Called once a frame, before the panel draws.
    pub(crate) fn refresh(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|i| i.focused);
        if focused && !self.was_focused {
            self.fresh = false;
        }
        self.was_focused = focused;
        self.icons_this_frame = 0;
        if !self.fresh {
            self.items = shelved(library::latest(&self.root));
            self.fresh = true;
        }
    }

    /// How many entries the library lists, one for each name: what the
    /// catalog's count adds to the built-in pieces.
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// The entries listed under a built-in section: those whose `category`
    /// names it, by its id or its name in any language the window speaks.
    pub(crate) fn in_category(&self, category: Category) -> Vec<Listed> {
        self.items
            .iter()
            .filter(|(_, _, filed)| *filed == Some(category))
            .map(|(entry, _, _)| Listed::of(entry))
            .collect()
    }

    /// The sections of the library's own, by name, with their entries: every
    /// `category` that is not a built-in one, and the entries with none under
    /// `untitled`.
    pub(crate) fn own_sections(&self, untitled: &str) -> Vec<(String, Vec<Listed>)> {
        let mut sections: Vec<(String, Vec<Listed>)> = Vec::new();
        for (entry, _, filed) in &self.items {
            if filed.is_some() {
                continue;
            }
            let section = match entry.category.as_deref().map(str::trim) {
                Some(name) if !name.is_empty() => name,
                _ => untitled,
            };
            match sections.iter_mut().find(|(name, _)| name == section) {
                Some((_, listed)) => listed.push(Listed::of(entry)),
                None => sections.push((section.to_owned(), vec![Listed::of(entry)])),
            }
        }
        // The sections people named in order, the unnamed pieces last.
        sections.sort_by_key(|(name, _)| (name == untitled, newera_catalog::fold(name)));
        sections
    }

    /// Entries whose name, title or section has one of the words.
    pub(crate) fn find(&self, query: &str) -> Vec<Listed> {
        let words: Vec<String> = newera_catalog::fold(query)
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        self.items
            .iter()
            .map(|(entry, _, _)| entry)
            .filter(|entry| {
                let text = newera_catalog::fold(&format!(
                    "{} {} {}",
                    entry.name,
                    entry.title.as_deref().unwrap_or_default(),
                    entry.category.as_deref().unwrap_or_default()
                ));
                words.iter().any(|word| text.contains(word.as_str()))
            })
            .map(Listed::of)
            .collect()
    }

    /// The entry's picture, read from disk the first time it is asked for —
    /// a few a frame. `None` when it has none, or it is still to be read.
    pub(crate) fn icon(
        &mut self,
        ctx: &egui::Context,
        listed: &Listed,
    ) -> Option<egui::TextureHandle> {
        let path = listed.icon.clone()?;
        if let Some(icon) = self.icons.get(&path) {
            return icon.clone();
        }
        if self.icons_this_frame >= ICONS_PER_FRAME {
            ctx.request_repaint();
            return None;
        }
        self.icons_this_frame += 1;
        let icon = read_icon(&path).map(|image| {
            ctx.load_texture(
                format!("library-icon:{}", path.display()),
                image,
                egui::TextureOptions::LINEAR,
            )
        });
        self.icons.insert(path, icon.clone());
        icon
    }
}

/// The entries as the shelf keeps them, each with its built-in section.
fn shelved(entries: Vec<(Entry, PathBuf)>) -> Vec<(Entry, PathBuf, Option<Category>)> {
    entries
        .into_iter()
        .map(|(entry, path)| {
            let filed = entry.category.as_deref().and_then(builtin);
            (entry, path, filed)
        })
        .collect()
}

/// The built-in section a `category` names, by id (`bedroom`) or by name in
/// any language the window speaks (`Quarto`, `Bedroom`, `Chambre`): a library
/// filled in English files its beds under the bedroom in a Portuguese window.
fn builtin(name: &str) -> Option<Category> {
    let name = newera_catalog::fold(name.trim());
    Category::ALL.into_iter().find(|c| {
        c.id() == name || crate::i18n::every(c.name()).any(|n| newera_catalog::fold(n) == name)
    })
}

/// An entry's picture, shrunk to [`ICON_PX`] on its longer side. `None`
/// when the file is missing or is not an image: the row shows a cube.
fn read_icon(path: &Path) -> Option<egui::ColorImage> {
    let bytes = newera_core::vfs::read(path).ok()?;
    let image = image::load_from_memory(&bytes)
        .ok()?
        .thumbnail(ICON_PX, ICON_PX)
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        image.as_raw(),
    ))
}

impl NewEraApp {
    /// Puts the latest version of a library entry in the middle of the plan
    /// view, at the size, height, in the unit and with the photos and edits it was
    /// published with — what `place model=library:<name>` does.
    pub(crate) fn place_from_library(&mut self, name: &str) {
        if crate::jobs::busy(self) {
            return;
        }
        let (entry, path) = match library::get(&self.library.root, name, None) {
            Ok(found) => found,
            Err(err) => return self.set_status(format!("⚠ {err}")),
        };
        let at = self.plan.view_center();
        let title = entry.title.clone().unwrap_or_else(|| entry.name.clone());
        crate::jobs::start(
            self,
            crate::i18n::tr("Importando modelo"),
            title,
            move || {
                newera_core::progress::step("Lendo o arquivo", 0, 0);
                let unit = entry.unit.as_deref().and_then(newera_catalog::Unit::parse);
                let loaded = newera_catalog::load_model_in(&path, unit);
                Box::new(move |app: &mut Self| app.put_library_piece(entry, &path, at, loaded))
            },
        );
    }

    /// Ends [`Self::place_from_library`] once the model is read: inserts the
    /// piece at `at` as one step to undo, fitted the way the entry was published,
    /// and selects it so it can be dragged into place. A model that failed to
    /// read leaves the plan as it was and says why in the status bar.
    fn put_library_piece(
        &mut self,
        entry: Entry,
        path: &Path,
        at: newera_core::Point2,
        loaded: Result<newera_catalog::ImportedModel, newera_catalog::ImportError>,
    ) {
        let model = match loaded {
            Ok(model) => model,
            Err(err) => return self.set_status(format!("⚠ {}: {err}", entry.name)),
        };
        let mut placed = None;
        self.run(|doc| {
            let mut piece = newera_core::Furniture {
                id: doc.new_furniture_id(),
                catalog: "imported".to_owned(),
                name: entry.title.clone().unwrap_or_else(|| entry.name.clone()),
                position: at,
                width: entry.size[0],
                depth: entry.size[1],
                height: entry.size[2],
                elevation: entry.elevation.unwrap_or(0.0),
                model: Some(path.display().to_string()),
                visible: true,
                ..Default::default()
            };
            entry.fit(&mut piece);
            placed = Some(piece.id);
            doc.execute(Command::insert(piece))
        });
        if let Some(id) = placed {
            self.selection = std::iter::once(ElementId::from(id)).collect();
            self.set_tool(Tool::Select);
            let mut status =
                crate::i18n::tr("Modelo da biblioteca posicionado. Arraste para o lugar.")
                    .to_owned();
            if !model.report.warnings.is_empty() {
                status.push_str(" ⚠ ");
                status.push_str(&model.report.warnings.join("; "));
            }
            self.set_status(status);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shelf already read, with one entry for each name and `category`,
    /// filed the way [`Shelf::refresh`] files what it reads.
    fn shelf(entries: &[(&str, Option<&str>)]) -> Shelf {
        let mut shelf = Shelf::new(PathBuf::new());
        shelf.items = shelved(
            entries
                .iter()
                .map(|(name, category)| {
                    let entry = Entry {
                        name: (*name).to_owned(),
                        category: category.map(str::to_owned),
                        size: [80.0, 85.0, 90.0],
                        ..Entry::default()
                    };
                    (entry, PathBuf::from(format!("{name}.obj")))
                })
                .collect(),
        );
        shelf.fresh = true;
        shelf
    }

    /// The names of the rows, in the order they are listed.
    fn names(listed: &[Listed]) -> Vec<&str> {
        listed.iter().map(|l| l.name.as_str()).collect()
    }

    /// A `category` that names a built-in section by id or in any language
    /// puts the piece under it; any other name makes a section of the library's
    /// own, and the pieces without one go last, under `untitled`.
    #[test]
    fn a_category_naming_a_built_in_section_files_the_piece_there() {
        let shelf = shelf(&[
            ("bed-oak", Some("bedroom")),
            ("bed-pine", Some(" Quarto ")),
            ("bed-fir", Some("Chambre")),
            ("lamp", Some("Luminárias de mesa")),
            ("stool", None),
            ("vase", Some("Vasos")),
        ]);
        assert_eq!(
            names(&shelf.in_category(Category::Bedroom)),
            ["bed-oak", "bed-pine", "bed-fir"]
        );
        let own = shelf.own_sections("Minha biblioteca");
        let titles: Vec<&str> = own.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(titles, ["Luminárias de mesa", "Vasos", "Minha biblioteca"]);
        assert_eq!(names(&own[2].1), ["stool"]);
    }

    /// The search finds an entry by its name, its title or its section, in any
    /// case.
    #[test]
    fn the_search_reads_name_title_and_section() {
        let mut shelf = shelf(&[("win", Some("Sala")), ("ares", None)]);
        shelf.items[1].0.title = Some("Cadeira Ares".into());
        assert_eq!(names(&shelf.find("cadeira")), ["ares"]);
        assert_eq!(names(&shelf.find("SALA")), ["win"]);
        assert!(shelf.find("mesa").is_empty());
    }
}
