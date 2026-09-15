//! Analytical parametric volume formulas for standard stock shapes used in manufacturing.
//!
//! Provides instant, exact, zero-dependency volume calculations.

use std::collections::HashMap;
use std::f64::consts::PI;

#[derive(Debug, thiserror::Error)]
pub enum PrimitiveError {
    #[error("Missing required variable '{0}'")]
    MissingVariable(String),
    #[error("Variable '{0}' must be positive, got {1}")]
    InvalidValue(String, f64),
    #[error("Inner diameter ({0}) cannot be greater than or equal to outer diameter ({1})")]
    InnerDiameterTooLarge(f64, f64),
    #[error("Unknown primitive kind '{0}'")]
    UnknownKind(String),
}

/// Supported standard shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveKind {
    Box,
    Cylinder,
    HollowPipe,
    HexBar,
    Flange,
}

impl PrimitiveKind {
    pub fn all() -> &'static [PrimitiveKind] {
        &[
            Self::Box,
            Self::Cylinder,
            Self::HollowPipe,
            Self::HexBar,
            Self::Flange,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Box => "box",
            Self::Cylinder => "cylinder",
            Self::HollowPipe => "hollow_pipe",
            Self::HexBar => "hex_bar",
            Self::Flange => "flange",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Box => "Box / Plate",
            Self::Cylinder => "Cylinder / Round Bar",
            Self::HollowPipe => "Hollow Pipe / Tube",
            Self::HexBar => "Hex Bar",
            Self::Flange => "Flange",
        }
    }

    pub fn formula_description(&self) -> &'static str {
        match self {
            Self::Box => "V = length × width × thickness",
            Self::Cylinder => "V = π × (diameter / 2)² × length",
            Self::HollowPipe => "V = π × ((outer_diameter / 2)² - (inner_diameter / 2)²) × length",
            Self::HexBar => "V = (√3 / 2) × across_flats² × length",
            Self::Flange => "V = π × ((outer_diameter / 2)² - (inner_diameter / 2)²) × thickness - (num_bolts × π × (bolt_diameter / 2)² × thickness)",
        }
    }

    pub fn default_openscad_code(&self) -> &'static str {
        match self {
            Self::Box => "cube([length, width, thickness]);",
            Self::Cylinder => "cylinder(h=length, r=diameter/2, $fn=64);",
            Self::HollowPipe => "difference() {\n    cylinder(h=length, r=outer_diameter/2, $fn=64);\n    cylinder(h=length + 0.001, r=inner_diameter/2, $fn=64);\n}",
            Self::HexBar => "cylinder(h=length, r=across_flats / sqrt(3), $fn=6);",
            Self::Flange => "difference() {\n    cylinder(h=thickness, r=outer_diameter/2, $fn=64);\n    cylinder(h=thickness + 0.001, r=inner_diameter/2, $fn=64);\n}",
        }
    }

    pub fn from_name(name: &str) -> Option<PrimitiveKind> {
        let lower = name.to_lowercase();
        if lower.contains("box") || lower.contains("plate") || lower.contains("cube") || lower.contains("block") {
            Some(Self::Box)
        } else if lower.contains("cylinder") || lower.contains("round") || lower.contains("rod") || (lower.contains("bar") && !lower.contains("hex")) {
            Some(Self::Cylinder)
        } else if lower.contains("pipe") || lower.contains("tube") || lower.contains("hollow") {
            Some(Self::HollowPipe)
        } else if lower.contains("hex") {
            Some(Self::HexBar)
        } else if lower.contains("flange") {
            Some(Self::Flange)
        } else {
            None
        }
    }

    pub fn required_variables(&self) -> &'static [&'static str] {
        match self {
            Self::Box => &["length", "width", "thickness"],
            Self::Cylinder => &["diameter", "length"],
            Self::HollowPipe => &["outer_diameter", "inner_diameter", "length"],
            Self::HexBar => &["across_flats", "length"],
            Self::Flange => &[
                "outer_diameter",
                "inner_diameter",
                "thickness",
                "bolt_diameter",
                "num_bolts",
            ],
        }
    }
}

pub const MM3_TO_M3: f64 = 1e-9;
pub const M3_TO_MM3: f64 = 1e9;

impl PrimitiveKind {
    /// Calculate volume in cubic millimeters (mm³).
    /// All input dimensions in `vars` are in millimeters (mm).
    pub fn calculate_volume_mm3(&self, vars: &HashMap<String, f64>) -> Result<f64, PrimitiveError> {
        match self {
            Self::Box => {
                let l = get_pos_var(vars, "length")?;
                let w = get_pos_var(vars, "width")?;
                let t = get_pos_var(vars, "thickness")?;
                Ok(l * w * t)
            }
            Self::Cylinder => {
                let d = get_pos_var(vars, "diameter")?;
                let l = get_pos_var(vars, "length")?;
                let r = d / 2.0;
                Ok(PI * r * r * l)
            }
            Self::HollowPipe => {
                let od = get_pos_var(vars, "outer_diameter")?;
                let id = get_pos_var(vars, "inner_diameter")?;
                let l = get_pos_var(vars, "length")?;
                if id >= od {
                    return Err(PrimitiveError::InnerDiameterTooLarge(id, od));
                }
                let r_out = od / 2.0;
                let r_in = id / 2.0;
                Ok(PI * (r_out * r_out - r_in * r_in) * l)
            }
            Self::HexBar => {
                let s = get_pos_var(vars, "across_flats")?;
                let l = get_pos_var(vars, "length")?;
                // Cross-sectional area of a regular hexagon with flat-to-flat distance s:
                // Area = (sqrt(3) / 2) * s^2
                let area = (3.0_f64.sqrt() / 2.0) * s * s;
                Ok(area * l)
            }
            Self::Flange => {
                let od = get_pos_var(vars, "outer_diameter")?;
                let id = vars.get("inner_diameter").copied().unwrap_or(0.0);
                if id >= od {
                    return Err(PrimitiveError::InnerDiameterTooLarge(id, od));
                }
                let t = get_pos_var(vars, "thickness")?;
                let bolt_d = vars.get("bolt_diameter").copied().unwrap_or(0.0);
                let num_bolts = vars.get("num_bolts").copied().unwrap_or(0.0);

                let r_out = od / 2.0;
                let r_in = id / 2.0;
                let mut ring_volume = PI * (r_out * r_out - r_in * r_in) * t;

                if bolt_d > 0.0 && num_bolts > 0.0 {
                    let bolt_r = bolt_d / 2.0;
                    let bolt_vol = PI * bolt_r * bolt_r * t * num_bolts;
                    ring_volume = (ring_volume - bolt_vol).max(0.0);
                }

                Ok(ring_volume)
            }
        }
    }

    /// Calculate volume in cubic meters (m³), scaling from mm dimensions.
    pub fn calculate_volume(&self, vars: &HashMap<String, f64>) -> Result<f64, PrimitiveError> {
        Ok(self.calculate_volume_mm3(vars)? * MM3_TO_M3)
    }
}

fn get_pos_var(vars: &HashMap<String, f64>, name: &str) -> Result<f64, PrimitiveError> {
    match vars.get(name) {
        Some(&val) => {
            if val <= 0.0 {
                Err(PrimitiveError::InvalidValue(name.to_string(), val))
            } else {
                Ok(val)
            }
        }
        None => Err(PrimitiveError::MissingVariable(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_volume() {
        let mut vars = HashMap::new();
        vars.insert("length".into(), 1000.0); // 1000mm = 1m
        vars.insert("width".into(), 1000.0);  // 1000mm = 1m
        vars.insert("thickness".into(), 10.0); // 10mm = 0.01m
        let vol_m3 = PrimitiveKind::Box.calculate_volume(&vars).unwrap();
        assert!((vol_m3 - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_cylinder_volume() {
        let mut vars = HashMap::new();
        vars.insert("diameter".into(), 100.0); // 100mm = 0.1m
        vars.insert("length".into(), 1000.0);   // 1000mm = 1m
        let vol_m3 = PrimitiveKind::Cylinder.calculate_volume(&vars).unwrap();
        let expected_m3 = PI * 0.05 * 0.05 * 1.0;
        assert!((vol_m3 - expected_m3).abs() < 1e-6);
    }
}
