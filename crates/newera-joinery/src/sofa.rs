//! Modular upholstered sofas from standard seat and back blocks.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Output, Part, num};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArmType {
    /// Straight block arms.
    #[default]
    Straight,
    /// Rounded arms.
    Rounded,
    /// No arms.
    None,
}

/// A modular sofa. Sizes in cm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SofaParams {
    /// Total length, cm (default 220).
    pub length: f64,
    /// Depth, cm (default 95).
    pub depth: f64,
    /// Seat height, cm (default 43).
    pub seat: f64,
    /// Back top height, cm (default 85).
    pub back: f64,
    pub arms: ArmType,
    /// Seat modules (default: one per ~75 cm).
    pub modules: Option<u32>,
    /// Fabric color `[r,g,b]` or `#rrggbb`.
    #[serde(deserialize_with = "crate::cabinet::rgb_or_hex")]
    #[schemars(with = "Option<crate::HandleColor>")]
    pub color: Option<[u8; 3]>,
}

impl Default for SofaParams {
    fn default() -> Self {
        Self {
            length: 220.0,
            depth: 95.0,
            seat: 43.0,
            back: 85.0,
            arms: ArmType::Straight,
            modules: None,
            color: None,
        }
    }
}

pub(crate) fn generate(p: &SofaParams) -> Result<Output, String> {
    let fabric = p.color.unwrap_or([150, 140, 125]);
    let mut notes: Vec<String> = Vec::new();
    if p.seat <= 0.0 || p.depth <= 0.0 || p.back <= p.seat {
        return Err(
            "O sofá precisa de assento e profundidade positivos e encosto acima do assento.".into(),
        );
    }
    if !(35.0..=55.0).contains(&p.seat) {
        notes.push(format!(
            "Assento a {} cm do chão: o usual é 43, e fora de 35 a 55 ninguém senta bem.",
            num(p.seat)
        ));
    }
    if p.back < p.seat + 25.0 {
        notes.push(format!(
            "Encosto só {} cm acima do assento: abaixo de 25 cm não apoia as costas.",
            num(p.back - p.seat)
        ));
    }
    if !(70.0..=180.0).contains(&p.depth) {
        notes.push(format!(
            "Profundidade de {} cm fora do usual (70 a 180 cm).",
            num(p.depth)
        ));
    }
    let arm = match p.arms {
        ArmType::None => 0.0,
        ArmType::Straight | ArmType::Rounded => 18.0,
    };
    let inner = p.length - 2.0 * arm;
    let modules = p
        .modules
        .unwrap_or_else(|| ((inner / 75.0).round() as u32).max(1));
    let module = inner / f64::from(modules);
    if module <= 0.0 {
        return Err(format!(
            "{modules} módulos não cabem em {} cm de sofá.",
            num(inner)
        ));
    }
    if !(45.0..=110.0).contains(&module) {
        let fit = ((inner / 75.0).round() as u32).max(1);
        notes.push(format!(
            "Com {modules} módulos cada assento fica com {} cm (usual 55 a 100); modules = {fit} acerta.",
            num(module)
        ));
    }
    let (base, back_t) = (12.0, 20.0);
    let cushion = p.seat - base;
    let mut parts = vec![Part::solid(
        "Base",
        [0.0, 0.0, 0.0],
        [p.length, p.depth, base],
        [60, 50, 45],
    )];
    parts.push(Part::solid(
        "Estrutura do encosto",
        [arm, 0.0, base],
        [inner, back_t, p.back - base - 10.0],
        fabric,
    ));
    for k in 0..modules {
        let x = arm + f64::from(k) * module;
        parts.push(Part::solid(
            &format!("Almofada de assento {}", k + 1),
            [x + 0.5, back_t, base],
            [module - 1.0, p.depth - back_t, cushion],
            fabric,
        ));
        parts.push(Part::solid(
            &format!("Almofada de encosto {}", k + 1),
            [x + 0.5, back_t, p.seat],
            [module - 1.0, 18.0, p.back - p.seat],
            fabric,
        ));
    }
    if arm > 0.0 {
        let arm_h = p.seat + 20.0;
        for (side, x) in [("esquerdo", 0.0), ("direito", p.length - arm)] {
            let mut part = Part::solid(
                &format!("Braço {side}"),
                [x, 0.0, base],
                [arm, p.depth, arm_h - base],
                fabric,
            );
            if p.arms == ArmType::Rounded {
                // A rounded front seen from above.
                let steps = 8;
                let mut outline = vec![[x, 0.0], [x + arm, 0.0], [x + arm, p.depth - arm / 2.0]];
                for i in 0..=steps {
                    let a = std::f64::consts::PI * f64::from(i) / f64::from(steps);
                    outline.push([
                        x + arm / 2.0 + arm / 2.0 * a.cos(),
                        p.depth - arm / 2.0 + arm / 2.0 * a.sin(),
                    ]);
                }
                outline.push([x, p.depth - arm / 2.0]);
                part.outline = Some(outline);
            }
            parts.push(part);
        }
    }
    Ok(Output {
        parts,
        size: [p.length, p.depth, p.back],
        hardware: vec![format!(
            "{} m² de tecido (estimado)",
            num((p.length * (p.depth + p.back) * 1.6) / 10_000.0)
        )],
        notes,
        extra_cuts: Vec::new(),
        name: format!("Sofá {} módulos {} cm", modules, num(p.length)),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    fn modules_split_the_seat_evenly_and_limits_guide() {
        let out = generate(&SofaParams::default()).unwrap();
        // 220 − 36 = 184 cm → 2 modules of 92 cm.
        let seats: Vec<&Part> = out
            .parts
            .iter()
            .filter(|p| p.name.starts_with("Almofada de assento"))
            .collect();
        assert_eq!(seats.len(), 2);
        assert!((seats[0].size[0] - 91.0).abs() < 1e-9);
        assert!(
            (seats[0].at[2] + seats[0].size[2] - 43.0).abs() < 1e-9,
            "seat height"
        );
        let rounded = generate(&SofaParams {
            arms: ArmType::Rounded,
            ..SofaParams::default()
        })
        .unwrap();
        assert!(rounded.parts.iter().any(|p| p.outline.is_some()));
        // Modules narrower than anyone sits on, and a seat nobody reaches:
        // both are built, and both say what a sofa usually is.
        let many = generate(&SofaParams {
            modules: Some(5),
            ..SofaParams::default()
        })
        .unwrap();
        assert!(
            many.notes.iter().any(|n| n.contains("modules = 2")),
            "{:?}",
            many.notes
        );
        let tall = generate(&SofaParams {
            seat: 70.0,
            ..SofaParams::default()
        })
        .unwrap();
        assert!(
            tall.notes.iter().any(|n| n.contains("35 a 55")),
            "{:?}",
            tall.notes
        );
    }
}
