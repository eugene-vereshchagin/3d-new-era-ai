//! Cut lists for the workshop: equal boards counted together, sizes in
//! millimeters, edge banding, and a DXF with the boards laid out on sheets
//! for a CNC or a panel saw.

use std::fmt::Write as _;

use crate::{Output, Part};
use newera_core::{Furniture, FurnitureId};

/// Thickest part of a drawn group read as a board, mm: a 50 mm stone or a
/// double panel still is, a 30 cm body of a model is not.
const DRAWN_BOARD_MM: f64 = 50.0;

/// What a storey's joinery is cut from: the rows of every build and of every
/// group drawn by hand with board parts, equal boards counted together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gathered {
    pub rows: Vec<CutRow>,
    /// Each build's hardware, with the build it goes into.
    pub hardware: Vec<(FurnitureId, String)>,
    /// How many joinery builds the rows came from.
    pub builds: usize,
    /// Groups drawn by hand whose board parts are in the rows.
    pub drawn: Vec<FurnitureId>,
    /// Parts of those groups that are not boards (a model, a solid too
    /// thick to be one), named instead of guessed into a row.
    pub skipped: Vec<(FurnitureId, String)>,
}

impl Gathered {
    /// Whether there is nothing to cut: no build and no drawn group.
    pub fn is_empty(&self) -> bool {
        self.builds == 0 && self.drawn.is_empty()
    }
}

/// The cut list of `furniture` (a storey's pieces), or of the pieces among
/// them whose id is in `wanted`: every joinery build with the rows its
/// generator gives, merged by board, size, banding and cutouts; and every
/// group drawn by hand, each part thin enough to be a board one, at the size
/// it was drawn, its finish and thickness for a board name.
///
/// # Errors
/// A build whose stored parameters no longer read or generate, named by id.
pub fn gather(furniture: &[Furniture], wanted: Option<&[FurnitureId]>) -> Result<Gathered, String> {
    let chosen = |f: &Furniture| wanted.is_none_or(|ids| ids.contains(&f.id));
    let mut out = Gathered::default();
    for group in furniture
        .iter()
        .filter(|f| f.properties.contains_key(crate::PARAMS_KEY) && chosen(f))
    {
        let build: crate::Build = serde_json::from_str(&group.properties[crate::PARAMS_KEY])
            .map_err(|e| format!("{}: {e}", group.id))?;
        let output = crate::generate(&build).map_err(|e| format!("{}: {e}", group.id))?;
        out.builds += 1;
        out.hardware
            .extend(output.hardware.iter().map(|h| (group.id, h.clone())));
        for row in cut_list(&output) {
            match out.rows.iter_mut().find(|r| {
                r.board == row.board
                    && r.edge == row.edge
                    && r.holes == row.holes
                    && same_size(r.size, row.size)
            }) {
                Some(existing) => existing.qty += row.qty,
                None => out.rows.push(row),
            }
        }
    }
    for group in furniture
        .iter()
        .filter(|f| f.is_group() && !f.properties.contains_key(crate::PARAMS_KEY) && chosen(f))
    {
        let mut boards = 0;
        let mut left = Vec::new();
        for part in group.visible_leaves() {
            let mut size =
                [part.width, part.depth, part.height].map(|cm| (cm * 100.0).round() / 10.0);
            size.sort_by(|a, b| b.total_cmp(a));
            if part.model.is_some() || size[2] > DRAWN_BOARD_MM || size[2] <= 0.0 {
                left.push((part.id, part.name.clone()));
                continue;
            }
            boards += 1;
            let board = part.texture.as_ref().map_or_else(
                || format!("desenhado {} mm", size[2]),
                |mat| format!("{mat} {} mm", size[2]),
            );
            let row = CutRow {
                name: part.name.clone(),
                board,
                qty: 1,
                size,
                edge: [0, 0],
                holes: Vec::new(),
            };
            match out
                .rows
                .iter_mut()
                .find(|r| r.board == row.board && r.name == row.name && same_size(r.size, row.size))
            {
                Some(existing) => existing.qty += 1,
                None => out.rows.push(row),
            }
        }
        if boards > 0 {
            out.drawn.push(group.id);
            out.skipped.extend(left);
        }
    }
    Ok(out)
}

/// Whether two boards are the same size, to a twentieth of a millimetre.
fn same_size(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.05)
}

/// One line of a cut list.
#[derive(Debug, Clone, PartialEq)]
pub struct CutRow {
    pub name: String,
    pub board: String,
    pub qty: u32,
    /// Length, width and thickness, mm (length is the longest side).
    pub size: [f64; 3],
    /// Edges to band `[long, short]`.
    pub edge: [u8; 2],
    /// Cutouts `[x, y, w, d]` in mm from the corner, along the length first.
    pub holes: Vec<[f64; 4]>,
}

fn mm(cm: f64) -> f64 {
    (cm * 10.0 * 10.0).round() / 10.0
}

fn row(part: &Part) -> Option<CutRow> {
    let board = part.board.clone()?;
    let mut dims = match part.cut {
        Some([length, width]) => [length, width, part.size[2]],
        None => part.size,
    };
    // The board's own length runs along x when x is its longest side.
    let along_x = part.size[0] >= part.size[1];
    dims.sort_by(|a, b| b.total_cmp(a));
    let holes = part
        .holes
        .iter()
        .map(|h| {
            if along_x {
                *h
            } else {
                [h[1], h[0], h[3], h[2]]
            }
        })
        .map(|h| h.map(mm))
        .collect();
    Some(CutRow {
        name: part.name.clone(),
        board,
        qty: 1,
        size: dims.map(mm),
        edge: part.edge,
        holes,
    })
}

/// Rows for every board of a build, equal boards merged (first name kept,
/// e.g. `Lateral esquerda` × 2).
pub fn cut_list(output: &Output) -> Vec<CutRow> {
    let mut rows: Vec<CutRow> = Vec::new();
    for part in output.parts.iter().chain(&output.extra_cuts) {
        let Some(new) = row(part) else { continue };
        match rows.iter_mut().find(|r| {
            r.board == new.board
                && r.edge == new.edge
                && r.holes == new.holes
                && r.size
                    .iter()
                    .zip(new.size)
                    .all(|(a, b)| (a - b).abs() < 0.05)
        }) {
            Some(existing) => existing.qty += 1,
            None => rows.push(new),
        }
    }
    rows.sort_by(|a, b| a.board.cmp(&b.board).then(b.size[0].total_cmp(&a.size[0])));
    rows
}

/// The cut list as CSV (semicolon separated, decimal comma), for spreadsheets.
pub fn cut_list_csv(rows: &[CutRow]) -> String {
    let mut out = String::from(
        "peca;material;qtd;comprimento_mm;largura_mm;espessura_mm;fita_longa;fita_curta;recortes_mm\n",
    );
    let n = |v: f64| format!("{v}").replace('.', ",");
    for r in rows {
        let _ = writeln!(
            out,
            "{};{};{};{};{};{};{};{};{}",
            r.name.replace(';', ","),
            r.board,
            r.qty,
            n(r.size[0]),
            n(r.size[1]),
            n(r.size[2]),
            r.edge[0],
            r.edge[1],
            r.holes
                .iter()
                .map(|h| format!("{}x{} em {},{}", n(h[2]), n(h[3]), n(h[0]), n(h[1])))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    out
}

/// The sheets boards are cut from: their size by material, the saw kerf
/// between two boards, and the factory edge trimmed off each side first.
///
/// Sheet sizes vary with the market — 2750 × 1830 mm MDF in Brazil,
/// 2800 × 2070 mm chipboard in Russia, 2800 × 2070 or 2440 × 1220 mm in
/// Europe — so the workshop's own stock replaces the defaults where it
/// differs, through one table: a board takes the size whose key is the
/// longest start of its name (`MDF 18` before `MDF`), and `*` any other.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetStock {
    /// `(start of the board name, [length, width] mm)`; `*` matches any board.
    pub sizes: Vec<(String, [f64; 2])>,
    /// Saw kerf between two boards, mm.
    pub kerf: f64,
    /// Factory edge trimmed off each side of a sheet before cutting, mm.
    pub trim: f64,
}

impl Default for SheetStock {
    /// The stock the layouts were drawn with before it could be told:
    /// 2750 × 1830 mm panels, stone slabs, gypsum boards and slat strips,
    /// a 4 mm kerf and no trim.
    fn default() -> Self {
        let sizes = [
            ("*", [2750.0, 1830.0]),
            // Large granite and quartz slabs.
            ("Pedra", [3200.0, 1900.0]),
            ("Gesso", [2400.0, 1200.0]),
            ("Ripa", [3000.0, 300.0]),
        ];
        Self {
            sizes: sizes.map(|(k, v)| (k.to_owned(), v)).to_vec(),
            kerf: 4.0,
            trim: 0.0,
        }
    }
}

impl SheetStock {
    /// The same stock with `sizes` taking the place of the entries of the
    /// same key and joining the rest.
    #[must_use]
    pub fn with_sizes(mut self, sizes: impl IntoIterator<Item = (String, [f64; 2])>) -> Self {
        for (key, size) in sizes {
            match self.sizes.iter_mut().find(|(k, _)| *k == key) {
                Some(entry) => entry.1 = size,
                None => self.sizes.push((key, size)),
            }
        }
        self
    }

    /// Sheet size for a board material, mm: the entry whose key is the
    /// longest start of `board`, else `*`.
    #[must_use]
    pub fn size(&self, board: &str) -> [f64; 2] {
        self.sizes
            .iter()
            .filter(|(k, _)| k != "*" && board.starts_with(k.as_str()))
            .max_by_key(|(k, _)| k.len())
            .or_else(|| self.sizes.iter().find(|(k, _)| k == "*"))
            .map_or([2750.0, 1830.0], |(_, size)| *size)
    }

    /// Whether boards can be laid out on this stock: a kerf and a trim that
    /// are finite and not negative, and every sheet still there once the
    /// trim is off both sides.
    ///
    /// # Errors
    ///
    /// Says which number cannot be cut from.
    pub fn check(&self) -> Result<(), String> {
        if !self.kerf.is_finite() || self.kerf < 0.0 {
            return Err(format!("kerf {} mm: give 0 or more", self.kerf));
        }
        if !self.trim.is_finite() || self.trim < 0.0 {
            return Err(format!("trim {} mm: give 0 or more", self.trim));
        }
        for (key, [length, width]) in &self.sizes {
            if !length.is_finite() || !width.is_finite() || length.min(*width) <= 2.0 * self.trim {
                return Err(format!(
                    "sheets.{key} {length}×{width} mm: nothing is left once {} mm is trimmed off each side",
                    self.trim
                ));
            }
        }
        Ok(())
    }
}

/// What a layout takes from the stock.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SheetUse {
    /// Sheets each material uses, in the order the materials first appear.
    pub sheets: Vec<(String, u32)>,
    /// Boards larger than their trimmed sheet, once each as `name WxH (material)`:
    /// drawn cut to the sheet and counted on it, so they need a splice or
    /// a bigger sheet.
    pub oversize: Vec<String>,
}

/// A board placed on a sheet, mm from the drawing origin.
struct Placed {
    layer: String,
    label: String,
    rect: [f64; 4],
    sheet: bool,
    /// Cutouts inside the board, placed like it.
    holes: Vec<[f64; 4]>,
}

/// Boards laid out on the sheets of `stock` (shelf packing, the kerf between
/// boards, inside the trimmed edges): sheet outlines, one rectangle per
/// board, and what the layout takes from the stock.
fn pack(rows: &[CutRow], stock: &SheetStock) -> Result<(Vec<Placed>, SheetUse), String> {
    stock.check()?;
    let kerf = stock.kerf;
    let trim = stock.trim;
    let mut placed = Vec::new();
    let mut used = SheetUse::default();
    let mut origin_y = 0.0;
    // Each material once, in the order it first appears.
    let mut boards: Vec<&str> = Vec::new();
    for r in rows {
        if !boards.contains(&r.board.as_str()) {
            boards.push(r.board.as_str());
        }
    }
    for board in boards {
        let layer = board.replace([' ', ','], "_");
        let [sw, sh] = stock.size(board);
        // What is left to cut once the factory edges are off.
        let (uw, uh) = (sw - 2.0 * trim, sh - 2.0 * trim);
        let outline = |x: f64| Placed {
            layer: layer.clone(),
            label: String::new(),
            rect: [x, origin_y, sw, sh],
            sheet: true,
            holes: Vec::new(),
        };
        let mut pieces: Vec<(String, f64, f64, Vec<[f64; 4]>)> = rows
            .iter()
            .filter(|r| r.board == board)
            .flat_map(|r| {
                (0..r.qty).map(move |_| (r.name.clone(), r.size[0], r.size[1], r.holes.clone()))
            })
            .collect();
        pieces.sort_by(|a, b| b.2.total_cmp(&a.2).then(b.1.total_cmp(&a.1)));
        let mut sheet_index = 0u32;
        let (mut x, mut y, mut shelf) = (0.0, 0.0, 0.0);
        let mut sheet_x = 0.0;
        placed.push(outline(sheet_x));
        for (name, mut w, mut h, mut holes) in pieces {
            // Turn pieces that only fit sideways.
            if w > uw && h <= uw && w <= uh {
                std::mem::swap(&mut w, &mut h);
                for hole in &mut holes {
                    *hole = [hole[1], hole[0], hole[3], hole[2]];
                }
            }
            // An oversize board is laid out cut to the sheet, so it fits by
            // that size and starts a shelf or a sheet only when one is in use.
            let (fw, fh) = (w.min(uw), h.min(uh));
            if x > 0.0 && x + fw > uw {
                x = 0.0;
                y += shelf + kerf;
                shelf = 0.0;
            }
            if y > 0.0 && y + fh > uh {
                sheet_index += 1;
                sheet_x += sw + 200.0;
                x = 0.0;
                y = 0.0;
                shelf = 0.0;
                placed.push(outline(sheet_x));
            }
            let oversize = w > uw || h > uh;
            let named = format!("{name} {w}x{h} ({board})");
            if oversize && !used.oversize.contains(&named) {
                used.oversize.push(named);
            }
            placed.push(Placed {
                layer: layer.clone(),
                label: if oversize {
                    format!("{name} {w}x{h} MAIOR QUE A CHAPA: prever emenda")
                } else {
                    format!("{name} {w}x{h}")
                },
                rect: [
                    sheet_x + trim + x,
                    origin_y + trim + y,
                    w.min(uw),
                    h.min(uh),
                ],
                sheet: false,
                holes: holes
                    .iter()
                    .map(|hole| {
                        [
                            sheet_x + trim + x + hole[0],
                            origin_y + trim + y + hole[1],
                            hole[2],
                            hole[3],
                        ]
                    })
                    .collect(),
            });
            x += fw + kerf;
            shelf = f64::max(shelf, fh);
        }
        used.sheets.push((board.to_owned(), sheet_index + 1));
        origin_y += sh + 400.0;
    }
    Ok((placed, used))
}

/// How many sheets of `stock` each material of `rows` takes, and the
/// boards no sheet holds whole.
///
/// # Errors
///
/// The stock cannot be cut from (see [`SheetStock::check`]).
pub fn sheets_used(rows: &[CutRow], stock: &SheetStock) -> Result<SheetUse, String> {
    Ok(pack(rows, stock)?.1)
}

/// The sheet layout as a DXF (LINE and TEXT entities, one layer per
/// material) for a CNC or a panel saw, and what it takes from the stock.
///
/// # Errors
///
/// The stock cannot be cut from (see [`SheetStock::check`]).
pub fn cut_list_dxf(rows: &[CutRow], stock: &SheetStock) -> Result<(String, SheetUse), String> {
    let (placed, sheets) = pack(rows, stock)?;
    let mut entities = String::new();
    for p in &placed {
        let [x, y, w, h] = p.rect;
        let corners = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        for i in 0..4 {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            let _ = write!(
                entities,
                "0\nLINE\n8\n{}\n10\n{}\n20\n{}\n11\n{}\n21\n{}\n",
                p.layer, a[0], a[1], b[0], b[1]
            );
        }
        for hole in &p.holes {
            let [hx, hy, hw, hh] = *hole;
            let corners = [[hx, hy], [hx + hw, hy], [hx + hw, hy + hh], [hx, hy + hh]];
            for i in 0..4 {
                let (a, b) = (corners[i], corners[(i + 1) % 4]);
                let _ = write!(
                    entities,
                    "0\nLINE\n8\nRECORTES\n10\n{}\n20\n{}\n11\n{}\n21\n{}\n",
                    a[0], a[1], b[0], b[1]
                );
            }
        }
        if !p.sheet {
            let _ = write!(
                entities,
                "0\nTEXT\n8\n{}\n10\n{}\n20\n{}\n40\n25\n1\n{}\n",
                p.layer,
                x + 10.0,
                y + 10.0,
                p.label
            );
        }
    }
    Ok((
        format!("0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n"),
        sheets,
    ))
}

/// The same sheet layout as an SVG (1 unit = 1 mm) to view or print.
///
/// # Errors
///
/// The stock cannot be cut from (see [`SheetStock::check`]).
pub fn cut_list_svg(rows: &[CutRow], stock: &SheetStock) -> Result<(String, SheetUse), String> {
    let (placed, sheets) = pack(rows, stock)?;
    let (w, h) = placed.iter().fold((0.0_f64, 0.0_f64), |(w, h), p| {
        (w.max(p.rect[0] + p.rect[2]), h.max(p.rect[1] + p.rect[3]))
    });
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-20 -20 {} {}\" font-family=\"sans-serif\">\n",
        w + 40.0,
        h + 40.0
    );
    for p in &placed {
        let [x, y, rw, rh] = p.rect;
        if p.sheet {
            let _ = writeln!(
                out,
                "<rect x=\"{x}\" y=\"{y}\" width=\"{rw}\" height=\"{rh}\" fill=\"#f4efe6\" stroke=\"#555\" stroke-width=\"4\"><title>{}</title></rect>",
                p.layer
            );
        } else {
            let label = p.label.replace('&', "&amp;").replace('<', "&lt;");
            let _ = writeln!(
                out,
                "<rect x=\"{x}\" y=\"{y}\" width=\"{rw}\" height=\"{rh}\" fill=\"#d9c3a0\" stroke=\"#333\" stroke-width=\"2\"/><text x=\"{}\" y=\"{}\" font-size=\"{}\">{label}</text>",
                x + 10.0,
                y + 35.0,
                (rh * 0.4).clamp(8.0, 30.0)
            );
            // Cutouts over the board, dashed.
            for hole in &p.holes {
                let [hx, hy, hw, hh] = *hole;
                let _ = writeln!(
                    out,
                    "<rect x=\"{hx}\" y=\"{hy}\" width=\"{hw}\" height=\"{hh}\" fill=\"#ffffff\" stroke=\"#c0392b\" stroke-width=\"3\" stroke-dasharray=\"12 6\"/>"
                );
            }
        }
    }
    out.push_str("</svg>\n");
    Ok((out, sheets))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::{Build, CabinetParams, generate};

    /// A storey's cut list counts two equal cabinets' boards together, reads
    /// a group drawn by hand as its boards and names the part that is not
    /// one, leaves out what is not wanted, and says when there is nothing.
    #[test]
    fn a_storey_gathers_its_builds_and_its_drawn_boards() {
        let cabinet = |id: u64| {
            let mut f = Furniture {
                id: FurnitureId(id),
                children: vec![Furniture::default()],
                ..Furniture::default()
            };
            f.properties.insert(
                crate::PARAMS_KEY.into(),
                serde_json::to_string(&Build::Cabinet(CabinetParams::default())).unwrap(),
            );
            f
        };
        let board = |id: u64, name: &str, size: (f64, f64, f64)| Furniture {
            id: FurnitureId(id),
            name: name.into(),
            width: size.0,
            depth: size.1,
            height: size.2,
            ..Furniture::default()
        };
        let shelf = Furniture {
            id: FurnitureId(30),
            children: vec![
                board(31, "Prateleira", (80.0, 25.0, 1.8)),
                board(32, "Prateleira", (80.0, 25.0, 1.8)),
                board(33, "Vaso", (20.0, 20.0, 30.0)),
            ],
            ..Furniture::default()
        };
        let one = gather(&[cabinet(10)], None).unwrap();
        let home = [cabinet(10), cabinet(20), shelf];
        let all = gather(&home, None).unwrap();
        assert_eq!(all.builds, 2);
        let qty = |g: &Gathered| g.rows.iter().map(|r| r.qty).sum::<u32>();
        let shelves = all
            .rows
            .iter()
            .find(|r| r.name == "Prateleira" && r.board.starts_with("desenhado"))
            .unwrap();
        assert_eq!(shelves.qty, 2);
        assert_eq!(qty(&all), 2 * qty(&one) + 2, "{:?}", all.rows);
        assert_eq!(all.drawn, vec![FurnitureId(30)]);
        assert_eq!(all.skipped, vec![(FurnitureId(33), "Vaso".to_owned())]);
        assert!(all.hardware.iter().any(|(id, _)| *id == FurnitureId(20)));
        let wanted = gather(&home, Some(&[FurnitureId(20)])).unwrap();
        assert_eq!((wanted.builds, wanted.drawn.len()), (1, 0));
        assert_eq!(wanted.rows, one.rows);
        assert!(gather(&[], None).unwrap().is_empty());
    }

    #[test]
    fn equal_boards_are_counted_and_laid_out_on_sheets() {
        let out = generate(&Build::Cabinet(CabinetParams {
            w: 80.0,
            h: 200.0,
            d: 55.0,
            ..CabinetParams::default()
        }))
        .unwrap();
        let rows = cut_list(&out);
        // Both sides are the same board: 1900 × 532 × 18 mm, banded on one long edge.
        let sides = rows.iter().find(|r| r.name == "Lateral esquerda").unwrap();
        assert_eq!(
            (sides.qty, sides.size, sides.edge),
            (2, [1900.0, 532.0, 18.0], [1, 0])
        );
        assert!(rows.iter().any(|r| r.board == "MDF 6"), "back panel listed");
        let csv = cut_list_csv(&rows);
        assert!(csv.starts_with("peca;material;qtd"));
        assert!(
            csv.contains("Lateral esquerda;MDF 18;2;1900;532;18;1;0"),
            "{csv}"
        );
        let stock = SheetStock::default();
        let (dxf, sheets) = cut_list_dxf(&rows, &stock).unwrap();
        assert!(dxf.starts_with("0\nSECTION") && dxf.ends_with("EOF\n"));
        assert!(dxf.contains("Lateral esquerda 1900x532"));
        // Four boards near 1,9 m long don't share one 2750 × 1830 mm sheet.
        assert_eq!(
            sheets.sheets.iter().find(|(b, _)| b == "MDF 18").unwrap().1,
            2
        );
        let (svg, same) = cut_list_svg(&rows, &stock).unwrap();
        assert!(svg.starts_with("<svg") && svg.contains("Lateral esquerda 1900x532"));
        assert_eq!(same, sheets);
        // A stone top carries its cutouts to the workshop.
        let top = generate(&Build::Countertop(crate::CountertopParams {
            length: 200.0,
            cutouts: vec![crate::Cutout {
                kind: crate::CutoutKind::Cooktop,
                x: 120.0,
                w: None,
                d: None,
                drawn: true,
            }],
            ..crate::CountertopParams::default()
        }))
        .unwrap();
        let rows = cut_list(&top);
        let stone = rows.iter().find(|r| r.board.starts_with("Pedra")).unwrap();
        assert_eq!(stone.holes, vec![[920.0, 60.0, 560.0, 480.0]]);
        assert!(cut_list_csv(&rows).contains("560x480 em 920,60"));
        assert!(cut_list_dxf(&rows, &stock).unwrap().0.contains("RECORTES"));
        assert!(
            cut_list_svg(&rows, &stock)
                .unwrap()
                .0
                .contains("stroke-dasharray")
        );
        // Rows gathered from several builds, materials interleaved: one layout each.
        let mut mixed = cut_list(&out);
        mixed.extend(cut_list(&top));
        mixed.extend(cut_list(&out));
        let (_, used) = cut_list_dxf(&mixed, &stock).unwrap();
        let mut names: Vec<&str> = used.sheets.iter().map(|(b, _)| b.as_str()).collect();
        let count = names.len();
        names.dedup();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "{used:?}");
    }

    /// The workshop's own sheets, kerf and trim change the layout, and only
    /// the materials they name.
    #[test]
    fn the_layout_follows_the_stock_of_the_workshop() {
        let out = generate(&Build::Cabinet(CabinetParams {
            w: 80.0,
            h: 200.0,
            d: 55.0,
            ..CabinetParams::default()
        }))
        .unwrap();
        let rows = cut_list(&out);
        let count = |stock: &SheetStock, board: &str| {
            sheets_used(&rows, stock)
                .unwrap()
                .sheets
                .into_iter()
                .find(|(b, _)| b == board)
                .unwrap()
                .1
        };
        let default = SheetStock::default();
        assert_eq!(count(&default, "MDF 18"), 2);
        // Every board without a closer key goes on the 2800 × 2070 mm sheet.
        let chipboard = SheetStock::default().with_sizes([("*".to_owned(), [2800.0, 2070.0])]);
        assert_eq!(chipboard.size("MDF 18"), [2800.0, 2070.0]);
        assert!(count(&chipboard, "MDF 18") <= count(&default, "MDF 18"));
        assert_eq!(
            chipboard.size("Pedra 2"),
            [3200.0, 1900.0],
            "stone keeps its slab"
        );
        // The longest key wins: only the 18 mm board changes.
        let only_18 = SheetStock::default().with_sizes([("MDF 18".to_owned(), [2800.0, 2070.0])]);
        assert_eq!(only_18.size("MDF 18"), [2800.0, 2070.0]);
        assert_eq!(only_18.size("MDF 6"), [2750.0, 1830.0]);
        // Trimmed edges move every board in from the corner of its sheet.
        let trimmed = SheetStock {
            trim: 10.0,
            ..chipboard.clone()
        };
        let (svg, _) = cut_list_svg(&rows, &trimmed).unwrap();
        let first_board = svg
            .lines()
            .find(|l| l.contains("#d9c3a0"))
            .expect("a board on the sheet");
        assert!(
            first_board.starts_with("<rect x=\"10\" y=\"10\""),
            "{first_board}"
        );
        // Boards that fill a sheet without kerf need a second one with it.
        let tight = vec![CutRow {
            name: "Faixa".into(),
            board: "MDF 18".into(),
            qty: 3,
            size: [1000.0, 610.0, 18.0],
            edge: [0, 0],
            holes: Vec::new(),
        }];
        let exact = SheetStock {
            kerf: 0.0,
            ..SheetStock::default().with_sizes([("*".to_owned(), [1000.0, 1830.0])])
        };
        assert_eq!(
            sheets_used(&tight, &exact).unwrap().sheets,
            vec![("MDF 18".to_owned(), 1)]
        );
        let sawn = SheetStock { kerf: 4.0, ..exact };
        assert_eq!(
            sheets_used(&tight, &sawn).unwrap().sheets,
            vec![("MDF 18".to_owned(), 2)]
        );
    }

    /// A board no trimmed sheet holds is named with the count, and stock
    /// that cannot be cut from is refused before any layout.
    #[test]
    fn oversize_boards_and_impossible_stock_are_reported() {
        let rows = vec![CutRow {
            name: "Tampo".into(),
            board: "MDF 18".into(),
            qty: 1,
            size: [1000.0, 980.0, 18.0],
            edge: [0, 0],
            holes: Vec::new(),
        }];
        let small = SheetStock {
            trim: 10.0,
            ..SheetStock::default().with_sizes([("*".to_owned(), [1000.0, 1000.0])])
        };
        let used = sheets_used(&rows, &small).unwrap();
        assert_eq!(used.sheets, vec![("MDF 18".to_owned(), 1)]);
        assert_eq!(used.oversize, vec!["Tampo 1000x980 (MDF 18)".to_owned()]);
        // Without the trim it fits, and nothing is reported.
        let whole = SheetStock {
            trim: 0.0,
            ..small.clone()
        };
        assert!(sheets_used(&rows, &whole).unwrap().oversize.is_empty());
        for bad in [
            SheetStock {
                kerf: -1.0,
                ..SheetStock::default()
            },
            SheetStock {
                trim: f64::NAN,
                ..SheetStock::default()
            },
            SheetStock {
                trim: 200.0,
                ..SheetStock::default()
            }, // the 300 mm slat strip
            SheetStock::default().with_sizes([("*".to_owned(), [f64::INFINITY, 1830.0])]),
        ] {
            assert!(cut_list_dxf(&rows, &bad).is_err(), "{bad:?}");
            assert!(cut_list_svg(&rows, &bad).is_err(), "{bad:?}");
            assert!(sheets_used(&rows, &bad).is_err(), "{bad:?}");
        }
    }
}
