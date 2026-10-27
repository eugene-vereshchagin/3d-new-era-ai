//! Loading user 3D models (OBJ, glTF/GLB) into catalog-style meshes.
//!
//! Imported meshes are normalized to the local frame (`y` up, centered on the
//! floor) so a piece can scale them to its exact width, depth and height.

use std::fmt::Write as _;
use std::path::Path;

use crate::mesh::{Mesh, MeshMaterial, Rgb};

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("unsupported model format `{0}` (use .obj, .gltf or .glb)")]
    Format(String),
    #[error("could not read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not read OBJ: {0}")]
    Obj(#[from] tobj::LoadError),
    #[error("could not read glTF: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("the model has no triangles")]
    Empty,
}

/// A loaded model with its natural size in centimeters.
#[derive(Debug, Clone)]
pub struct ImportedModel {
    pub mesh: Mesh,
    /// Width, depth, height in cm, after unit detection.
    pub size: [f64; 3],
    /// How the file was read, and what of it was left out.
    pub report: ImportReport,
}

/// What an import made of a file: the unit it took, and what in the file
/// is not drawn — a missing texture, a normal map, an extension — so "it
/// loaded" is not mistaken for "it looks as it should".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportReport {
    /// `obj`, `gltf` or `glb`.
    pub format: String,
    /// Meshes (OBJ objects, glTF primitives) read.
    pub meshes: usize,
    /// Width, depth, height in the file's own units.
    pub raw_size: [f64; 3],
    /// The unit taken for those numbers: `m`, `cm`, `mm` or `in`.
    pub unit: &'static str,
    /// Whether that unit was guessed from the size rather than given.
    pub guessed: bool,
    /// What was left out or could not be found, one sentence each.
    pub warnings: Vec<String>,
}

const DEFAULT_COLOR: Rgb = [0.78, 0.76, 0.72];

/// The unit a model file's numbers are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    M,
    Cm,
    Mm,
    In,
}

impl Unit {
    pub const NAMES: [&str; 4] = ["m", "cm", "mm", "in"];

    /// `m`, `cm`, `mm` or `in`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "m" => Some(Self::M),
            "cm" => Some(Self::Cm),
            "mm" => Some(Self::Mm),
            "in" | "inch" => Some(Self::In),
            _ => None,
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::M => "m",
            Self::Cm => "cm",
            Self::Mm => "mm",
            Self::In => "in",
        }
    }

    #[must_use]
    pub fn to_cm(self) -> f64 {
        match self {
            Self::M => 100.0,
            Self::Cm => 1.0,
            Self::Mm => 0.1,
            Self::In => 2.54,
        }
    }

    /// The guess for a file that does not say: under 20 units across it is
    /// taken as meters, over 2000 as millimeters, otherwise centimeters.
    #[must_use]
    pub fn guess(largest: f64) -> Self {
        if largest < 20.0 {
            Self::M
        } else if largest > 2000.0 {
            Self::Mm
        } else {
            Self::Cm
        }
    }
}

/// Loads a model file, guessing its unit from its size ([`Unit::guess`]).
pub fn load_model(path: &Path) -> Result<ImportedModel, ImportError> {
    load_model_in(path, None)
}

/// The least width, depth or height an imported model gets, cm: a flat
/// side of it (a plane) still has to have some.
const MIN_SIZE_CM: f64 = 0.01;

/// Loads a model file whose numbers are in `unit`, or guessed when `None`.
/// A 5 cm part in a file drawn in centimeters is under 20 units across, and
/// the guess would make it 5 m: saying the unit is how such a file comes in
/// at its size.
pub fn load_model_in(path: &Path, unit: Option<Unit>) -> Result<ImportedModel, ImportError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut report = ImportReport {
        format: ext.clone(),
        ..ImportReport::default()
    };
    let mut mesh = match ext.as_str() {
        "obj" => load_obj(path, &mut report)?,
        "gltf" | "glb" => load_gltf(path, &mut report)?,
        other => return Err(ImportError::Format(other.to_owned())),
    };
    let (min, max) = mesh.bounds().ok_or(ImportError::Empty)?;
    let raw = [
        f64::from(max[0] - min[0]),
        f64::from(max[2] - min[2]),
        f64::from(max[1] - min[1]),
    ];
    let largest = raw.iter().copied().fold(0.0, f64::max);
    report.guessed = unit.is_none();
    let unit = unit.unwrap_or_else(|| Unit::guess(largest));
    report.raw_size = raw;
    report.unit = unit.name();
    // glTF says its numbers are meters; a file that reads otherwise was
    // exported wrong, and which way is a guess worth saying.
    if matches!(ext.as_str(), "gltf" | "glb") && unit != Unit::M {
        report.warnings.push(format!(
            "glTF numbers are meters, but this file is {largest:.1} units across: read as {}{}",
            unit.name(),
            if report.guessed {
                " (pass the unit if that is wrong)"
            } else {
                " as asked"
            }
        ));
    }
    // A shelf 8 mm thick is 0.8 cm, not 1: only a flat side (a plane, no
    // thickness at all) is given the least size a piece can have.
    let size = raw.map(|v| (v * unit.to_cm()).max(MIN_SIZE_CM));
    mesh.fit_to(size[0], size[1], size[2]);
    Ok(ImportedModel { mesh, size, report })
}

fn load_obj(path: &Path, report: &mut ImportReport) -> Result<Mesh, ImportError> {
    let text = newera_core::vfs::read(path)?;
    let has_library = text
        .split(|&b| b == b'\n')
        .any(|line| line.trim_ascii_start().starts_with(b"mtllib"));
    // Files without a material library name materials from Wavefront's
    // classic default library; supply it so their colors survive.
    let mut source = Vec::with_capacity(text.len() + 32);
    if !has_library {
        source.extend_from_slice(b"mtllib __defaults__.mtl\n");
    }
    source.extend_from_slice(&text);
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let used: Vec<String> = String::from_utf8_lossy(&text)
        .lines()
        .filter_map(|l| l.trim().strip_prefix("usemtl").map(|n| n.trim().to_owned()))
        .collect();
    let unread = std::cell::RefCell::new(Vec::new());
    let (models, materials) = tobj::load_obj_buf(
        &mut std::io::Cursor::new(source),
        &tobj::GPU_LOAD_OPTIONS,
        |mtl| {
            if mtl == Path::new("__defaults__.mtl") {
                return tobj::load_mtl_buf(&mut std::io::Cursor::new(default_library(&used)));
            }
            let Ok(bytes) = newera_core::vfs::read(&dir.join(mtl)) else {
                unread.borrow_mut().push(format!(
                    "material library {} not found: the model is drawn gray",
                    mtl.display()
                ));
                return Err(tobj::LoadError::OpenFileFailed);
            };
            tobj::load_mtl_buf(&mut std::io::Cursor::new(bytes)).inspect_err(|e| {
                unread.borrow_mut().push(format!(
                    "material library {} could not be read ({e}): the model is drawn gray",
                    mtl.display()
                ));
            })
        },
    )?;
    report.warnings.extend(unread.into_inner());
    if !has_library && !used.is_empty() {
        report.warnings.push(
            "no material library (mtllib): colors are guessed from the material names".to_owned(),
        );
    }
    // Missing or broken material files leave the model gray rather than failing.
    let materials = materials.unwrap_or_default();
    for m in &materials {
        obj_material_warnings(m, &dir, &mut report.warnings);
    }
    report.meshes = models.len();
    let mut mesh = Mesh {
        materials: materials
            .iter()
            .map(|m| MeshMaterial {
                name: m.name.clone(),
                color: m.diffuse.unwrap_or(DEFAULT_COLOR),
                alpha: m.dissolve.unwrap_or(1.0).clamp(0.0, 1.0),
                texture: m
                    .diffuse_texture
                    .as_ref()
                    .filter(|t| !t.trim().is_empty())
                    .map(|t| dir.join(t.trim())),
                shininess: m.shininess.unwrap_or(0.0),
                // An opacity map that is the color image itself cuts holes
                // where its alpha is low: how an MTL says "cane weave".
                cutoff: own_alpha(m).then_some(0.5),
                double_sided: own_alpha(m),
            })
            .collect(),
        ..Mesh::default()
    };
    for model in models {
        let m = &model.mesh;
        let material = m
            .material_id
            .and_then(|i| u16::try_from(i).ok())
            .filter(|&i| usize::from(i) < mesh.materials.len());
        let color = material.map_or(DEFAULT_COLOR, |i| mesh.materials[usize::from(i)].color);
        let positions: Vec<[f32; 3]> = m
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect();
        let normals: Vec<[f32; 3]> = m
            .normals
            .as_chunks::<3>()
            .0
            .iter()
            .map(|n| [n[0], n[1], n[2]])
            .collect();
        let uvs: Vec<[f32; 2]> = m
            .texcoords
            .as_chunks::<2>()
            .0
            .iter()
            .map(|t| [t[0], t[1]])
            .collect();
        let triangles: Vec<[u32; 3]> = m
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| [t[0], t[1], t[2]])
            .collect();
        let start = mesh.positions.len();
        let part = mesh.begin_part();
        append_triangles(&mut mesh, &positions, &normals, &triangles, color);
        mesh.end_part(&model.name, part);
        for tri in &triangles {
            for &k in tri {
                mesh.uvs
                    .push(uvs.get(k as usize).copied().unwrap_or([0.0, 0.0]));
            }
        }
        mesh.uvs.truncate(mesh.positions.len());
        mesh.vertex_materials
            .resize(mesh.positions.len(), material.unwrap_or(u16::MAX));
        debug_assert!(mesh.vertex_materials.len() >= start);
    }
    if mesh.indices.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(mesh)
}

/// Whether an MTL material's opacity map is its color image: its alpha
/// then cuts the surface.
fn own_alpha(m: &tobj::Material) -> bool {
    let trimmed = |t: &Option<String>| {
        t.as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
    };
    trimmed(&m.dissolve_texture).is_some()
        && trimmed(&m.dissolve_texture) == trimmed(&m.diffuse_texture)
}

/// What of an MTL material is not drawn, or not found.
fn obj_material_warnings(m: &tobj::Material, dir: &Path, warnings: &mut Vec<String>) {
    let name = &m.name;
    if let Some(file) = m.diffuse_texture.as_deref().map(str::trim)
        && !file.is_empty()
        && !newera_core::vfs::exists(&dir.join(file))
    {
        warnings.push(format!(
            "texture {file} of material {name} not found: drawn in its plain color"
        ));
    }
    let maps = [
        (&m.normal_texture, "normal/bump map"),
        (&m.specular_texture, "specular map"),
        (&m.shininess_texture, "shininess map"),
        (
            if own_alpha(m) {
                &None
            } else {
                &m.dissolve_texture
            },
            "opacity map (map_d) other than its color image",
        ),
        (&m.ambient_texture, "ambient map"),
    ];
    for (map, what) in maps {
        if map.as_deref().is_some_and(|f| !f.trim().is_empty()) {
            warnings.push(format!("{what} of material {name} is not drawn"));
        }
    }
    let mut unknown: Vec<&String> = m
        .unknown_param
        .keys()
        .filter(|k| {
            k.starts_with("map_") || matches!(k.as_str(), "norm" | "disp" | "bump" | "refl")
        })
        .collect();
    unknown.sort();
    for key in unknown {
        warnings.push(format!("{key} of material {name} is not drawn"));
    }
}

/// Standard base64 (glTF data URIs).
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    };
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in text
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
    {
        acc = (acc << 6) | u32::from(value(c)?);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((acc >> bits) & 0xFF).ok()?);
        }
    }
    Some(out)
}

/// MTL text for material names from the classic Wavefront default library
/// (`white`, `flgrey`, `silver`…), with colors derived from their names.
fn default_library(names: &[String]) -> String {
    let mut out = String::new();
    let mut seen = std::collections::HashSet::new();
    for name in names.iter().filter(|n| seen.insert(n.as_str())) {
        let [r, g, b, alpha] = default_material_color(name);
        let _ = writeln!(out, "newmtl {name}\nKd {r} {g} {b}\nd {alpha}");
    }
    out
}

fn default_material_color(name: &str) -> [f32; 4] {
    let key = name.to_ascii_lowercase();
    let base = key.strip_prefix("fl").unwrap_or(&key);
    let bright = base.ends_with("brt");
    let base = base
        .trim_end_matches("brt")
        .trim_end_matches(|c: char| c.is_ascii_digit());
    let grey = |v: f32| [v, v, v, 1.0];
    let mut color = match base {
        "white" | "archwhite" => grey(0.95),
        "black" => grey(0.04),
        "grey" | "gray" => grey(0.5),
        "ltgrey" | "lightgrey" => grey(0.7),
        "dkgrey" | "darkgrey" => grey(0.28),
        "dkdkgrey" => grey(0.14),
        "silver" | "chrome" => grey(0.76),
        "gold" | "brass" => [0.8, 0.65, 0.3, 1.0],
        "amber" => [0.85, 0.55, 0.12, 1.0],
        "bone" => [0.9, 0.87, 0.78, 1.0],
        "yellow" => [0.9, 0.82, 0.2, 1.0],
        "tan" => [0.78, 0.66, 0.5, 1.0],
        "lighttan" | "lttan" => [0.88, 0.8, 0.66, 1.0],
        "blonde" => [0.86, 0.76, 0.54, 1.0],
        "brown" => [0.45, 0.3, 0.18, 1.0],
        "red" => [0.75, 0.1, 0.08, 1.0],
        "green" => [0.15, 0.55, 0.2, 1.0],
        "blue" => [0.15, 0.3, 0.75, 1.0],
        "orange" => [0.9, 0.5, 0.1, 1.0],
        "glass" | "clear" => [0.8, 0.88, 0.9, 0.3],
        _ => [0.78, 0.76, 0.72, 1.0],
    };
    if bright {
        for c in &mut color[..3] {
            *c = (*c * 1.15).min(1.0);
        }
    }
    color
}

type Mat4 = [[f32; 4]; 4];

fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [[0.0; 4]; 4];
    for (c, column) in out.iter_mut().enumerate() {
        for (r, cell) in column.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    out
}

fn transform_point(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r])
}

fn transform_vector(m: &Mat4, v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[0][r] * v[0] + m[1][r] * v[1] + m[2][r] * v[2])
}

fn load_gltf(path: &Path, report: &mut ImportReport) -> Result<Mesh, ImportError> {
    let gltf::Gltf { document, blob } = gltf::Gltf::from_slice(&newera_core::vfs::read(path)?)?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    // Buffers: the GLB chunk, embedded data URIs or files next to the model.
    let buffers: Vec<Vec<u8>> = document
        .buffers()
        .map(|buffer| match buffer.source() {
            gltf::buffer::Source::Bin => Ok(blob.clone().unwrap_or_default()),
            gltf::buffer::Source::Uri(uri) => match uri.split_once(";base64,") {
                Some((_, data)) if uri.starts_with("data:") => {
                    decode_base64(data).ok_or_else(|| std::io::Error::other("bad data URI").into())
                }
                _ => {
                    let file = beside(&dir, uri).ok_or_else(|| {
                        std::io::Error::other(format!(
                            "buffer `{uri}` is outside the model's folder"
                        ))
                    })?;
                    newera_core::vfs::read(&file).map_err(ImportError::from)
                }
            },
        })
        .collect::<Result<_, ImportError>>()?;
    let images = gltf_images(&document, &buffers, path);
    gltf_warnings(&document, &images, &mut report.warnings);
    let mut mesh = Mesh {
        materials: document
            .materials()
            .map(|m| gltf_material(&m, &images))
            .collect(),
        ..Mesh::default()
    };
    let identity: Mat4 = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next());
    let mut skipped = 0;
    let mut stack: Vec<(gltf::Node<'_>, Mat4)> = scene
        .map(|s| s.nodes().map(|n| (n, identity)).collect())
        .unwrap_or_default();
    while let Some((node, parent)) = stack.pop() {
        let world = mul(&parent, &node.transform().matrix());
        if let Some(node_mesh) = node.mesh() {
            let part = mesh.begin_part();
            let name = node
                .name()
                .or_else(|| node_mesh.name())
                .map_or_else(|| format!("node{}", node.index()), str::to_owned);
            for primitive in node_mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    skipped += 1;
                    continue;
                }
                report.meshes += 1;
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let Some(positions) = reader.read_positions() else {
                    continue;
                };
                let positions: Vec<[f32; 3]> =
                    positions.map(|p| transform_point(&world, p)).collect();
                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|n| n.map(|v| transform_vector(&world, v)).collect())
                    .unwrap_or_default();
                let indices: Vec<u32> = match reader.read_indices() {
                    Some(indices) => indices.into_u32().collect(),
                    None => (0..u32::try_from(positions.len()).unwrap_or(0)).collect(),
                };
                let triangles: Vec<[u32; 3]> = indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|t| [t[0], t[1], t[2]])
                    .collect();
                let material = primitive.material();
                let [r, g, b, _] = material.pbr_metallic_roughness().base_color_factor();
                // glTF puts the texture origin at the top left; meshes keep
                // OBJ's bottom-up v, which the renderers flip.
                let set = material
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .map_or(0, |t| t.tex_coord());
                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(set)
                    .map(|t| t.into_f32().map(|[u, v]| [u, 1.0 - v]).collect())
                    .unwrap_or_default();
                // Primitives without a material use glTF's default one, kept
                // as a gray material of its own after the file's.
                let index = material.index().unwrap_or_else(|| {
                    let fallback = document.materials().len();
                    if mesh.materials.len() == fallback {
                        mesh.materials.push(MeshMaterial {
                            name: "default".to_owned(),
                            color: [1.0; 3],
                            alpha: 1.0,
                            texture: None,
                            shininess: 0.0,
                            cutoff: None,
                            double_sided: false,
                        });
                    }
                    fallback
                });
                let material = u16::try_from(index).unwrap_or(u16::MAX);
                let start = mesh.positions.len();
                append_triangles(&mut mesh, &positions, &normals, &triangles, [r, g, b]);
                mesh.uvs.resize(start, [0.0, 0.0]);
                for tri in &triangles {
                    if tri.iter().all(|&k| (k as usize) < positions.len()) {
                        mesh.uvs.extend(
                            tri.iter()
                                .map(|&k| uvs.get(k as usize).copied().unwrap_or([0.0, 0.0])),
                        );
                    }
                }
                mesh.vertex_materials.resize(start, u16::MAX);
                mesh.vertex_materials.resize(mesh.positions.len(), material);
            }
            mesh.end_part(&name, part);
        }
        stack.extend(node.children().map(|child| (child, world)));
    }
    if skipped > 0 {
        report.warnings.push(format!(
            "{skipped} primitives of lines or points are not drawn"
        ));
    }
    if mesh.indices.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(mesh)
}

/// What of a glTF file is not drawn: textures other than the base color,
/// emission, alpha masks, animation and extensions.
fn gltf_warnings(
    document: &gltf::Document,
    images: &[Option<std::path::PathBuf>],
    warnings: &mut Vec<String>,
) {
    for extension in document.extensions_used() {
        warnings.push(format!("extension {extension} is not supported"));
    }
    for (k, image) in images.iter().enumerate() {
        match image {
            Some(file) if !newera_core::vfs::exists(file) => warnings.push(format!(
                "image {} not found: its materials are drawn in their plain color",
                file.display()
            )),
            None => warnings.push(format!("image {k} could not be read")),
            Some(_) => {}
        }
    }
    for m in document.materials() {
        let name = m.name().map_or_else(
            || format!("material{}", m.index().unwrap_or_default()),
            str::to_owned,
        );
        let maps = [
            (m.normal_texture().is_some(), "normal map"),
            (m.occlusion_texture().is_some(), "occlusion map"),
            (m.emissive_texture().is_some(), "emissive map"),
            (
                m.pbr_metallic_roughness()
                    .metallic_roughness_texture()
                    .is_some(),
                "metallic-roughness map",
            ),
            (m.emissive_factor().iter().any(|c| *c > 0.0), "emission"),
            (
                m.alpha_mode() == gltf::material::AlphaMode::Blend
                    && m.pbr_metallic_roughness().base_color_texture().is_some(),
                "image alpha of a BLEND material (blended by its factor only)",
            ),
        ];
        for (present, what) in maps {
            if present {
                warnings.push(format!("{what} of material {name} is not drawn"));
            }
        }
    }
    if document.animations().next().is_some() || document.skins().next().is_some() {
        warnings.push("animations and skins are ignored: the model is drawn at rest".to_owned());
    }
}

/// The file of each glTF image: a file next to the model, or — for images
/// inside the GLB or in data URIs — the bytes mounted beside the model as
/// `<model>#<index>.<ext>`, so every renderer reads them like any texture.
fn gltf_images(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    path: &Path,
) -> Vec<Option<std::path::PathBuf>> {
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let file = path
        .file_name()
        .map_or_else(|| "model".to_owned(), |n| n.to_string_lossy().into_owned());
    let json = document.as_json();
    let mut mounted = Vec::new();
    let images = document
        .images()
        .map(|image| {
            let raw = json.images.get(image.index())?;
            let ext = |mime: Option<&str>| match mime {
                Some("image/jpeg") => "jpg",
                _ => "png",
            };
            let (bytes, ext) = match (&raw.buffer_view, raw.uri.as_deref()) {
                (Some(_), _) => {
                    // A buffer view image must say its type; without it the
                    // decoder still recognizes PNG and JPEG by their bytes.
                    let view = document.views().nth(raw.buffer_view?.value())?;
                    let data = buffers.get(view.buffer().index())?;
                    let bytes =
                        data.get(view.offset()..view.offset().checked_add(view.length())?)?;
                    (
                        bytes.to_vec(),
                        ext(raw.mime_type.as_ref().map(|m| m.0.as_str())),
                    )
                }
                (None, Some(uri)) => match uri.split_once(";base64,") {
                    Some((head, data)) if uri.starts_with("data:") => {
                        let mime = head.strip_prefix("data:");
                        (decode_base64(data)?, ext(mime))
                    }
                    _ => return beside(&dir, uri),
                },
                (None, None) => return None,
            };
            let name = format!("{file}#{}.{ext}", image.index());
            let target = dir.join(&name);
            mounted.push((name, bytes));
            Some(target)
        })
        .collect();
    if !mounted.is_empty() {
        newera_core::vfs::mount(&dir, mounted);
    }
    images
}

/// A glTF URI as a file next to the model: relative, and never leaving
/// its folder. A model that names `../../.ssh/id_rsa` or `/etc/passwd` as
/// a texture would otherwise have it read, drawn and embedded in exports.
fn beside(dir: &Path, uri: &str) -> Option<std::path::PathBuf> {
    let decoded = percent_decode(uri);
    let path = Path::new(&decoded);
    let plain = path.components().all(|c| {
        matches!(
            c,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    });
    // `C:` and `file:` would be schemes, not folders.
    let scheme = decoded
        .split_once(':')
        .is_some_and(|(head, _)| !head.contains('/'));
    (plain && !scheme && !decoded.is_empty()).then(|| dir.join(path))
}

/// `%20` and the like in a glTF URI.
fn percent_decode(uri: &str) -> String {
    let bytes = uri.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn gltf_material(m: &gltf::Material<'_>, images: &[Option<std::path::PathBuf>]) -> MeshMaterial {
    let pbr = m.pbr_metallic_roughness();
    let [r, g, b, a] = pbr.base_color_factor();
    let a = a.clamp(0.0, 1.0);
    let cutoff = m.alpha_cutoff().unwrap_or(0.5).clamp(0.0, 1.0);
    MeshMaterial {
        name: m.name().map_or_else(
            || format!("material{}", m.index().unwrap_or_default()),
            str::to_owned,
        ),
        color: [r, g, b],
        alpha: match m.alpha_mode() {
            gltf::material::AlphaMode::Blend => a.clamp(0.0, 1.0),
            // Under the cutoff whatever the image says: not there at all.
            gltf::material::AlphaMode::Mask if a < cutoff => 0.0,
            _ => 1.0,
        },
        texture: pbr
            .base_color_texture()
            .and_then(|t| images.get(t.texture().source().index())?.clone()),
        shininess: 0.0,
        // The image's alpha times the factor's against the cutoff: the image's
        // alone against the cutoff over the factor.
        cutoff: (m.alpha_mode() == gltf::material::AlphaMode::Mask && a >= cutoff)
            .then(|| (cutoff / a.max(1e-6)).min(1.0)),
        double_sided: m.double_sided(),
    }
}

/// Appends triangles as flat-shaded faces when normals are missing.
fn append_triangles(
    mesh: &mut Mesh,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    triangles: &[[u32; 3]],
    color: Rgb,
) {
    for tri in triangles {
        let Some(corners) = tri
            .iter()
            .map(|i| positions.get(*i as usize).copied())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let face = {
            let (a, b, c) = (corners[0], corners[1], corners[2]);
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
            n.map(|x| x / len)
        };
        let base = u32::try_from(mesh.positions.len()).expect("mesh fits in u32");
        for (k, corner) in corners.iter().enumerate() {
            mesh.positions.push(*corner);
            let normal = normals.get(tri[k] as usize).copied().unwrap_or(face);
            mesh.normals.push(normal);
            mesh.colors.push(color);
        }
        mesh.indices.extend([base, base + 1, base + 2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_an_obj_in_meters_as_centimeters() {
        let dir = std::env::temp_dir().join(format!("newera-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cube.obj");
        // A 2 m × 1 m × 0.5 m box (meters) with CCW faces.
        let v = "v -1 0 -0.25\nv 1 0 -0.25\nv 1 1 -0.25\nv -1 1 -0.25\nv -1 0 0.25\nv 1 0 0.25\nv 1 1 0.25\nv -1 1 0.25\n";
        let f = "f 5 6 7 8\nf 2 1 4 3\nf 6 2 3 7\nf 1 5 8 4\nf 8 7 3 4\nf 1 2 6 5\n";
        std::fs::write(&path, format!("{v}{f}")).unwrap();

        let model = load_model(&path).unwrap();
        assert!(
            model
                .size
                .iter()
                .zip([200.0, 50.0, 100.0])
                .all(|(a, b)| (a - b).abs() < 1e-6),
            "{:?}",
            model.size
        );
        let (min, max) = model.mesh.bounds().unwrap();
        assert!((max[0] - min[0] - 200.0).abs() < 1e-3 && min[1].abs() < 1e-3);
        crate::mesh::tests::assert_outward(&model.mesh);
        assert!(matches!(
            load_model(&dir.join("x.fbx")),
            Err(ImportError::Format(_))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod memory_tests {
    use super::*;

    /// A GLB: one triangle with UVs, a named material whose color is an
    /// image stored in the binary chunk.
    pub(crate) fn textured_glb(image: &[u8]) -> Vec<u8> {
        glb_with(image, "", "")
    }

    /// [`textured_glb`] with more JSON in its material and at its root.
    pub(crate) fn glb_with(image: &[u8], material: &str, root: &str) -> Vec<u8> {
        glb_with_pbr(image, material, "", root)
    }

    /// [`glb_with`], with more of `pbrMetallicRoughness` after its texture.
    pub(crate) fn glb_with_pbr(image: &[u8], material: &str, pbr: &str, root: &str) -> Vec<u8> {
        let mut bin = Vec::new();
        for v in [[0.0f32, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0]] {
            bin.extend(v.iter().flat_map(|c| c.to_le_bytes()));
        }
        for t in [[0.0f32, 0.0], [1.0, 0.0], [0.0, 0.25]] {
            bin.extend(t.iter().flat_map(|c| c.to_le_bytes()));
        }
        let image_at = bin.len();
        bin.extend_from_slice(image);
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let json = format!(
            r#"{{"asset":{{"version":"2.0"}},{root}"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0,"name":"assento"}}],
            "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0,"TEXCOORD_0":1}},"material":0}}]}}],
            "materials":[{{"name":"tecido",{material}"pbrMetallicRoughness":{{"baseColorTexture":{{"index":0}}{pbr}}}}}],
            "textures":[{{"source":0}}],"images":[{{"bufferView":2,"mimeType":"image/png"}}],
            "buffers":[{{"byteLength":{len}}}],
            "bufferViews":[{{"buffer":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":24}},{{"buffer":0,"byteOffset":{image_at},"byteLength":{image_len}}}],
            "accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[100,100,0]}},
                         {{"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"}}]}}"#,
            len = bin.len(),
            image_len = image.len(),
        );
        let mut json = json.into_bytes();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let chunk = |kind: &[u8; 4], data: &[u8]| {
            let mut out = u32::try_from(data.len()).unwrap().to_le_bytes().to_vec();
            out.extend_from_slice(kind);
            out.extend_from_slice(data);
            out
        };
        let body = [chunk(b"JSON", &json), chunk(b"BIN\0", &bin)].concat();
        let mut glb = b"glTF".to_vec();
        glb.extend(2u32.to_le_bytes());
        glb.extend(u32::try_from(12 + body.len()).unwrap().to_le_bytes());
        glb.extend(body);
        glb
    }

    #[test]
    fn a_glb_keeps_its_uvs_and_the_images_inside_it() {
        let dir = Path::new("/virtual/catalog-glb-texture-test");
        let image = b"\x89PNG\r\n\x1a\nnot really decoded here".to_vec();
        newera_core::vfs::mount(dir, [("chair.glb".to_owned(), textured_glb(&image))]);
        let model = load_model(&dir.join("chair.glb")).unwrap();
        let mesh = &model.mesh;
        assert_eq!(mesh.materials.len(), 1);
        assert_eq!(mesh.materials[0].name, "tecido");
        let texture = mesh.materials[0]
            .texture
            .as_ref()
            .expect("the image is kept");
        assert_eq!(newera_core::vfs::read(texture).unwrap(), image);
        assert_eq!(mesh.vertex_materials, vec![0; 3]);
        // v is flipped to OBJ's bottom-up convention.
        assert_eq!(mesh.uvs, vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.75]]);
        newera_core::vfs::unmount(dir);
    }

    #[test]
    fn a_gltf_image_next_to_the_model_is_found_by_its_uri() {
        let dir = Path::new("/virtual/catalog-gltf-uri-test");
        let glb = textured_glb(b"");
        // Same file as JSON with the image in a file named with a space.
        let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let json = String::from_utf8(glb[20..20 + len].to_vec())
            .unwrap()
            .replace(
                r#"{"bufferView":2,"mimeType":"image/png"}"#,
                r#"{"uri":"tecido%20azul.png"}"#,
            );
        let bin = &glb[20 + len + 8..];
        let json = json.replace(
            r#""buffers":[{"byteLength""#,
            r#""buffers":[{"uri":"chair.bin","byteLength""#,
        );
        newera_core::vfs::mount(
            dir,
            [
                ("chair.gltf".to_owned(), json.into_bytes()),
                ("chair.bin".to_owned(), bin.to_vec()),
            ],
        );
        let model = load_model(&dir.join("chair.gltf")).unwrap();
        assert_eq!(
            model.mesh.materials[0].texture.as_deref(),
            Some(dir.join("tecido azul.png").as_path())
        );
        newera_core::vfs::unmount(dir);
    }

    #[test]
    fn a_gltf_cannot_name_a_file_outside_its_folder() {
        let dir = Path::new("/virtual/catalog-escape/models");
        for uri in [
            "../secret.png",
            "%2E%2E/secret.png",
            "/etc/passwd",
            "file:///etc/passwd",
            "C:secret.png",
        ] {
            let glb = textured_glb(b"");
            let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
            let json = String::from_utf8(glb[20..20 + len].to_vec())
                .unwrap()
                .replace(
                    r#"{"bufferView":2,"mimeType":"image/png"}"#,
                    &format!(r#"{{"uri":"{uri}"}}"#),
                );
            let bin = &glb[20 + len + 8..];
            newera_core::vfs::mount(
                dir,
                [
                    (
                        "chair.gltf".to_owned(),
                        json.replace(
                            r#""buffers":[{"byteLength""#,
                            r#""buffers":[{"uri":"chair.bin","byteLength""#,
                        )
                        .into_bytes(),
                    ),
                    ("chair.bin".to_owned(), bin.to_vec()),
                    (
                        "buffer.gltf".to_owned(),
                        json.replace(
                            r#""buffers":[{"byteLength""#,
                            &format!(r#""buffers":[{{"uri":"{uri}","byteLength""#),
                        )
                        .into_bytes(),
                    ),
                ],
            );
            let model = load_model(&dir.join("chair.gltf")).unwrap();
            assert_eq!(model.mesh.materials[0].texture, None, "{uri}");
            assert!(load_model(&dir.join("buffer.gltf")).is_err(), "{uri}");
        }
        newera_core::vfs::unmount(Path::new("/virtual/catalog-escape"));
    }

    #[test]
    fn the_report_says_what_was_not_drawn() {
        let dir = Path::new("/virtual/catalog-report-test");
        newera_core::vfs::mount(
            dir,
            [
                (
                    "lost.obj".to_owned(),
                    b"mtllib gone.mtl\nusemtl wood\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
                ),
                (
                    "maps.obj".to_owned(),
                    b"mtllib maps.mtl\nusemtl wood\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
                ),
                (
                    "maps.mtl".to_owned(),
                    b"newmtl wood\nmap_Kd missing.png\nmap_Bump wood-n.png\nmap_Pr rough.png\n"
                        .to_vec(),
                ),
                (
                    "pbr.glb".to_owned(),
                    glb_with(
                        b"png",
                        r#""normalTexture":{"index":0},"alphaMode":"MASK","#,
                        r#""extensionsUsed":["KHR_materials_transmission"],"#,
                    ),
                ),
            ],
        );
        // A missing library is said, and the geometry still comes in.
        let lost = load_model(&dir.join("lost.obj")).unwrap();
        assert_eq!(lost.mesh.indices.len(), 3);
        assert_eq!(lost.report.format, "obj");
        assert_eq!(lost.report.unit, "m");
        assert!(
            lost.report.warnings[0].contains("gone.mtl not found"),
            "{:?}",
            lost.report.warnings
        );

        let maps = load_model(&dir.join("maps.obj")).unwrap().report.warnings;
        for said in [
            "texture missing.png of material wood not found",
            "normal/bump map of material wood is not drawn",
            "map_Pr of material wood is not drawn",
        ] {
            assert!(maps.iter().any(|w| w.contains(said)), "{said}: {maps:?}");
        }

        let pbr = load_model(&dir.join("pbr.glb")).unwrap();
        // The mask is drawn now: a cutoff, not a warning.
        assert_eq!(pbr.mesh.materials[0].cutoff, Some(0.5));
        let pbr = pbr.report;
        assert_eq!(pbr.meshes, 1);
        for said in [
            "extension KHR_materials_transmission is not supported",
            "normal map of material tecido is not drawn",
        ] {
            assert!(
                pbr.warnings.iter().any(|w| w.contains(said)),
                "{said}: {pbr:?}"
            );
        }
        // A clean file says nothing but the unit it had to guess: its 100
        // units are not meters, as glTF's should be.
        newera_core::vfs::mount(dir, [("clean.glb".to_owned(), textured_glb(b"png"))]);
        let clean = load_model(&dir.join("clean.glb")).unwrap().report;
        assert_eq!(clean.warnings.len(), 1, "{clean:?}");
        assert!(
            clean.warnings[0].starts_with("glTF numbers are meters"),
            "{clean:?}"
        );
        assert_eq!((clean.format.as_str(), clean.unit), ("glb", "cm"));
        newera_core::vfs::unmount(dir);
    }

    #[test]
    fn the_named_pieces_of_a_file_stay_apart() {
        let dir = Path::new("/virtual/catalog-parts-test");
        let tri = |x: f32| format!("v {x} 0 0\nv {} 0 0\nv {x} 1 0\n", x + 1.0);
        let obj = format!(
            "o braco_esquerdo\n{}f 1 2 3\no assento\n{}f 4 5 6\nf 4 6 5\no braco_esquerdo\n{}f 7 8 9\n",
            tri(0.0),
            tri(2.0),
            tri(4.0)
        );
        newera_core::vfs::mount(
            dir,
            [
                ("chair.obj".to_owned(), obj.into_bytes()),
                ("chair.glb".to_owned(), textured_glb(b"png")),
            ],
        );
        let obj = load_model(&dir.join("chair.obj")).unwrap().mesh;
        let parts: Vec<(&str, usize, usize)> = obj
            .parts
            .iter()
            .map(|p| (p.name.as_str(), p.start, p.count))
            .collect();
        assert_eq!(
            parts,
            [
                ("braco_esquerdo", 0, 1),
                ("assento", 1, 2),
                ("braco_esquerdo#2", 3, 1)
            ]
        );
        // In centimeters, after the unit guess (meters): the seat spans 2..3 m.
        let (min, max) = obj.part_bounds(&obj.parts[1]).unwrap();
        assert!((max[0] - min[0] - 100.0).abs() < 1e-3, "{min:?} {max:?}");
        let glb = load_model(&dir.join("chair.glb")).unwrap().mesh;
        assert_eq!(glb.parts.len(), 1);
        assert_eq!(
            (glb.parts[0].name.as_str(), glb.parts[0].count),
            ("assento", 1)
        );
        newera_core::vfs::unmount(dir);
    }

    #[test]
    fn a_part_under_a_centimetre_keeps_its_size() {
        let dir = Path::new("/virtual/catalog-thin-test");
        // A 100 × 100 cm panel 8 mm thick (file z is the depth), and a plane.
        let v = "v 0 0 0\nv 100 0 0\nv 100 100 0\nv 0 100 0\nv 0 0 0.8\nv 100 0 0.8\nv 100 100 0.8\nv 0 100 0.8\n";
        let f = "f 5 6 7 8\nf 2 1 4 3\nf 6 2 3 7\nf 1 5 8 4\nf 8 7 3 4\nf 1 2 6 5\n";
        newera_core::vfs::mount(
            dir,
            [
                ("panel.obj".to_owned(), format!("{v}{f}").into_bytes()),
                (
                    "plane.obj".to_owned(),
                    b"v 0 0 0\nv 50 0 0\nv 50 50 0\nv 0 50 0\nf 1 2 3 4\n".to_vec(),
                ),
            ],
        );
        let panel = load_model_in(&dir.join("panel.obj"), Some(Unit::Cm)).unwrap();
        assert!(
            panel
                .size
                .iter()
                .zip([100.0, 0.8, 100.0])
                .all(|(a, b)| (a - b).abs() < 1e-4),
            "{:?}",
            panel.size
        );
        let plane = load_model_in(&dir.join("plane.obj"), Some(Unit::Cm)).unwrap();
        assert!(plane.size.iter().all(|&v| v > 0.0), "{:?}", plane.size);
        assert!(plane.size[1] < 0.1, "{:?}", plane.size);
        newera_core::vfs::unmount(dir);
    }

    #[test]
    fn a_unit_said_is_taken_over_the_guess() {
        let dir = Path::new("/virtual/catalog-unit-test");
        newera_core::vfs::mount(
            dir,
            [
                (
                    "knob.obj".to_owned(),
                    b"v 0 0 0\nv 5 0 0\nv 5 3 0\nv 0 0 4\nf 1 2 3\nf 1 3 4\n".to_vec(),
                ),
                ("chair.glb".to_owned(), textured_glb(b"png")),
            ],
        );
        let guessed = load_model(&dir.join("knob.obj")).unwrap();
        assert_eq!((guessed.report.unit, guessed.report.guessed), ("m", true));
        assert!((guessed.size[0] - 500.0).abs() < 1e-3);
        let said = load_model_in(&dir.join("knob.obj"), Some(Unit::Cm)).unwrap();
        assert_eq!((said.report.unit, said.report.guessed), ("cm", false));
        assert!((said.size[0] - 5.0).abs() < 1e-3);
        let inches = load_model_in(&dir.join("knob.obj"), Some(Unit::In)).unwrap();
        assert!((inches.size[0] - 12.7).abs() < 1e-3);
        // A glTF 100 units across is not meters, as the standard says it is.
        let glb = load_model(&dir.join("chair.glb")).unwrap().report;
        assert!(
            glb.warnings
                .iter()
                .any(|w| w.contains("glTF numbers are meters")),
            "{glb:?}"
        );
        newera_core::vfs::unmount(dir);
    }

    #[test]
    fn an_image_whose_alpha_cuts_holes_is_a_mask() {
        let dir = Path::new("/virtual/catalog-mask-test");
        newera_core::vfs::mount(
            dir,
            [
                (
                    "cane.obj".to_owned(),
                    b"mtllib cane.mtl\nusemtl palha\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
                ),
                (
                    "cane.mtl".to_owned(),
                    b"newmtl palha\nmap_Kd palha.png\nmap_d palha.png\n".to_vec(),
                ),
                ("palha.png".to_owned(), b"png".to_vec()),
                (
                    "cane.glb".to_owned(),
                    glb_with(
                        b"png",
                        r#""alphaMode":"MASK","alphaCutoff":0.3,"doubleSided":true,"#,
                        "",
                    ),
                ),
            ],
        );
        let obj = load_model(&dir.join("cane.obj")).unwrap();
        let m = &obj.mesh.materials[0];
        assert_eq!((m.cutoff, m.double_sided), (Some(0.5), true));
        assert!(obj.report.warnings.is_empty(), "{:?}", obj.report.warnings);
        let glb = load_model(&dir.join("cane.glb")).unwrap();
        let m = &glb.mesh.materials[0];
        assert_eq!((m.cutoff, m.double_sided), (Some(0.3), true));
        newera_core::vfs::unmount(dir);
    }

    #[test]
    fn a_mask_counts_the_color_factor_alpha_against_its_cutoff() {
        let dir = Path::new("/virtual/catalog-mask-factor-test");
        let with_factor = |a: f32| {
            glb_with_pbr(
                b"png",
                r#""alphaMode":"MASK","alphaCutoff":0.3,"#,
                &format!(r#","baseColorFactor":[1,1,1,{a}]"#),
                "",
            )
        };
        newera_core::vfs::mount(
            dir,
            [
                ("half.glb".to_owned(), with_factor(0.6)),
                ("faint.glb".to_owned(), with_factor(0.2)),
            ],
        );
        // Image alpha × 0.6 under 0.3: the image's own alpha under 0.5.
        let half = &load_model(&dir.join("half.glb")).unwrap().mesh.materials[0];
        assert!(
            half.cutoff.is_some_and(|c| (c - 0.5).abs() < 1e-6),
            "{half:?}"
        );
        assert!((half.alpha - 1.0).abs() < 1e-6, "{half:?}");
        // 0.2 is under 0.3 whatever the image: not drawn.
        let faint = &load_model(&dir.join("faint.glb")).unwrap().mesh.materials[0];
        assert_eq!((faint.cutoff, faint.alpha), (None, 0.0), "{faint:?}");
        newera_core::vfs::unmount(dir);
    }

    #[test]
    fn models_load_from_mounted_files_with_their_materials() {
        let dir = Path::new("/virtual/catalog-memory-test");
        newera_core::vfs::mount(
            dir,
            [
                (
                    "cube/cube.obj".to_owned(),
                    b"mtllib cube.mtl\nusemtl red\nv 0 0 0\nv 100 0 0\nv 100 100 0\nv 0 0 50\nf 1 2 3\nf 1 3 4\n".to_vec(),
                ),
                ("cube/cube.mtl".to_owned(), b"newmtl red\nKd 0.9 0.1 0.1\n".to_vec()),
                // A glTF whose buffer is a base64 data URI: one triangle.
                (
                    "tri.gltf".to_owned(),
                    br#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],
                        "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
                        "buffers":[{"byteLength":36,"uri":"data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAADIQgAAAAAAAAAAAAAAAAAAyEIAAAAA"}],
                        "bufferViews":[{"buffer":0,"byteLength":36}],
                        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[100,100,0]}]}"#
                        .to_vec(),
                ),
            ],
        );
        let obj = load_model(&dir.join("cube/cube.obj")).unwrap();
        assert_eq!(obj.mesh.materials[0].name, "red");
        assert!((obj.mesh.materials[0].color[0] - 0.9).abs() < 1e-6);
        let gltf = load_model(&dir.join("tri.gltf")).unwrap();
        assert_eq!(gltf.mesh.indices.len(), 3);
        newera_core::vfs::unmount(dir);
        assert!(load_model(&dir.join("tri.gltf")).is_err());
    }
}
