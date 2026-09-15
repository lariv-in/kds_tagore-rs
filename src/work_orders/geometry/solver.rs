//! Solves for the remaining free dimension of a component given target weight, cost, or volume.
//!
//! Provides exact analytical inversion for built-in standard primitives (Box, Cylinder, Pipe, HexBar)
//! and robust monotonic numerical inversion for general OpenSCAD shapes.

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::work_orders::entities::{material, shape};
use super::PrimitiveKind;

#[derive(Debug, thiserror::Error)]
pub enum SolverError {
    #[error("Target volume must be positive, got {0}")]
    InvalidTargetVolume(f64),
    #[error("Target weight must be positive, got {0}")]
    InvalidTargetWeight(f64),
    #[error("Target cost must be positive, got {0}")]
    InvalidTargetCost(f64),
    #[error("Material density must be positive, got {0}")]
    InvalidDensity(f64),
    #[error("Material rate must be positive, got {0}")]
    InvalidRate(f64),
    #[error("Expected exactly 1 free variable, found {0}: {1:?}")]
    NotSingleFreeVariable(usize, Vec<String>),
    #[error("Fixed variable '{0}' is missing or invalid ({1})")]
    InvalidFixedVariable(String, f64),
    #[error("Could not solve dimension for variable '{0}': {1}")]
    CalculationFailed(String, String),
}

/// Solves for the single free variable of a shape given a target volume in m³.
/// Returns the calculated dimension in millimeters (mm).
pub fn solve_dimension_from_volume(
    shape: &shape::Model,
    fixed_vars: &HashMap<String, f64>,
    free_var_name: &str,
    target_volume_m3: f64,
) -> Result<f64, SolverError> {
    if target_volume_m3 <= 0.0 || target_volume_m3.is_nan() || target_volume_m3.is_infinite() {
        return Err(SolverError::InvalidTargetVolume(target_volume_m3));
    }

    let target_volume_mm3 = target_volume_m3 * 1e9;

    // 1. Check if shape is a standard analytical primitive
    if let Some(kind) = shape.standard_kind() {
        if let Ok(val) = solve_primitive_dimension(kind, fixed_vars, free_var_name, target_volume_mm3) {
            return Ok(val);
        }
    }

    // 2. Fallback: numerical monotonic root-finding using shape volume
    solve_numerical_dimension(shape, fixed_vars, free_var_name, target_volume_m3)
}

/// Solves for the free variable of a standard primitive shape analytically.
/// `target_volume_mm3` is in mm³, and `fixed_vars` are in mm. Result is in mm.
fn solve_primitive_dimension(
    kind: PrimitiveKind,
    fixed_vars: &HashMap<String, f64>,
    free_var: &str,
    target_volume_mm3: f64,
) -> Result<f64, SolverError> {
    match kind {
        PrimitiveKind::Box => {
            // V_mm3 = length_mm * width_mm * thickness_mm
            let mut other_prod = 1.0;
            for &v_name in kind.required_variables() {
                if v_name != free_var {
                    let val = get_fixed(fixed_vars, v_name)?;
                    other_prod *= val;
                }
            }
            if other_prod <= 0.0 {
                return Err(SolverError::CalculationFailed(
                    free_var.to_string(),
                    "Product of fixed dimensions is zero or negative".into(),
                ));
            }
            Ok(target_volume_mm3 / other_prod)
        }

        PrimitiveKind::Cylinder => {
            // V = pi * (diameter / 2)^2 * length
            if free_var == "length" {
                let d = get_fixed(fixed_vars, "diameter")?;
                let r = d / 2.0;
                let area = PI * r * r;
                Ok(target_volume_mm3 / area)
            } else if free_var == "diameter" {
                let l = get_fixed(fixed_vars, "length")?;
                let r_sq = target_volume_mm3 / (PI * l);
                if r_sq < 0.0 {
                    return Err(SolverError::CalculationFailed(
                        free_var.to_string(),
                        "Negative radius squared".into(),
                    ));
                }
                Ok(2.0 * r_sq.sqrt())
            } else {
                Err(SolverError::CalculationFailed(
                    free_var.to_string(),
                    format!("Unknown variable '{}' for cylinder", free_var),
                ))
            }
        }

        PrimitiveKind::HollowPipe => {
            // V = pi * ((outer_diameter / 2)^2 - (inner_diameter / 2)^2) * length
            if free_var == "length" {
                let od = get_fixed(fixed_vars, "outer_diameter")?;
                let id = get_fixed(fixed_vars, "inner_diameter")?;
                if id >= od {
                    return Err(SolverError::CalculationFailed(
                        free_var.to_string(),
                        "Inner diameter must be less than outer diameter".into(),
                    ));
                }
                let area = PI * ((od / 2.0).powi(2) - (id / 2.0).powi(2));
                Ok(target_volume_mm3 / area)
            } else if free_var == "outer_diameter" {
                let id = get_fixed(fixed_vars, "inner_diameter")?;
                let l = get_fixed(fixed_vars, "length")?;
                let r_out_sq = (target_volume_mm3 / (PI * l)) + (id / 2.0).powi(2);
                Ok(2.0 * r_out_sq.sqrt())
            } else if free_var == "inner_diameter" {
                let od = get_fixed(fixed_vars, "outer_diameter")?;
                let l = get_fixed(fixed_vars, "length")?;
                let r_in_sq = (od / 2.0).powi(2) - (target_volume_mm3 / (PI * l));
                if r_in_sq <= 0.0 {
                    return Err(SolverError::CalculationFailed(
                        free_var.to_string(),
                        "Target volume exceeds solid cylinder volume".into(),
                    ));
                }
                Ok(2.0 * r_in_sq.sqrt())
            } else {
                Err(SolverError::CalculationFailed(
                    free_var.to_string(),
                    format!("Unknown variable '{}' for pipe", free_var),
                ))
            }
        }

        PrimitiveKind::HexBar => {
            // V = (sqrt(3) / 2) * across_flats^2 * length
            let factor = 3.0_f64.sqrt() / 2.0;
            if free_var == "length" {
                let s = get_fixed(fixed_vars, "across_flats")?;
                let area = factor * s * s;
                Ok(target_volume_mm3 / area)
            } else if free_var == "across_flats" {
                let l = get_fixed(fixed_vars, "length")?;
                let s_sq = target_volume_mm3 / (factor * l);
                Ok(s_sq.sqrt())
            } else {
                Err(SolverError::CalculationFailed(
                    free_var.to_string(),
                    format!("Unknown variable '{}' for hex bar", free_var),
                ))
            }
        }

        PrimitiveKind::Flange => {
            if free_var == "thickness" {
                let od = get_fixed(fixed_vars, "outer_diameter")?;
                let id = fixed_vars.get("inner_diameter").copied().unwrap_or(0.0);
                let bolt_d = fixed_vars.get("bolt_diameter").copied().unwrap_or(0.0);
                let num_bolts = fixed_vars.get("num_bolts").copied().unwrap_or(0.0);

                let mut net_area = PI * ((od / 2.0).powi(2) - (id / 2.0).powi(2));
                if bolt_d > 0.0 && num_bolts > 0.0 {
                    let bolt_area = PI * (bolt_d / 2.0).powi(2) * num_bolts;
                    net_area = (net_area - bolt_area).max(1e-12);
                }
                Ok(target_volume_mm3 / net_area)
            } else {
                Err(SolverError::CalculationFailed(
                    free_var.to_string(),
                    "Analytical inversion for flange only supports thickness".into(),
                ))
            }
        }
    }
}

/// Numerical bisection solver for arbitrary parametric shapes.
fn solve_numerical_dimension(
    shape: &shape::Model,
    fixed_vars: &HashMap<String, f64>,
    free_var: &str,
    target_volume: f64,
) -> Result<f64, SolverError> {
    let mut low = 1e-4;
    let mut high = 1000.0;

    // Expand upper bound if needed
    for _ in 0..10 {
        let mut test_vars = fixed_vars.clone();
        test_vars.insert(free_var.to_string(), high);
        let vol = shape.clone().get_volume(test_vars);
        if vol >= target_volume {
            break;
        }
        high *= 10.0;
    }

    // Binary search
    for _ in 0..40 {
        let mid = (low + high) / 2.0;
        let mut test_vars = fixed_vars.clone();
        test_vars.insert(free_var.to_string(), mid);
        let vol = shape.clone().get_volume(test_vars);

        if (vol - target_volume).abs() / target_volume < 1e-6 {
            return Ok(mid);
        }

        if vol < target_volume {
            low = mid;
        } else {
            high = mid;
        }
    }

    Ok((low + high) / 2.0)
}

fn get_fixed(fixed: &HashMap<String, f64>, name: &str) -> Result<f64, SolverError> {
    match fixed.get(name) {
        Some(&val) if val > 0.0 => Ok(val),
        Some(&val) => Err(SolverError::InvalidFixedVariable(name.to_string(), val)),
        None => Err(SolverError::InvalidFixedVariable(name.to_string(), 0.0)),
    }
}

/// Solves for the remaining free dimension from a target weight in kilograms.
pub fn solve_dimension_from_weight(
    shape: &shape::Model,
    material: &material::Model,
    fixed_vars: &HashMap<String, f64>,
    free_var_name: &str,
    target_weight: f64,
) -> Result<f64, SolverError> {
    if target_weight <= 0.0 || target_weight.is_nan() || target_weight.is_infinite() {
        return Err(SolverError::InvalidTargetWeight(target_weight));
    }
    if material.density <= 0.0 || material.density.is_nan() || material.density.is_infinite() {
        return Err(SolverError::InvalidDensity(material.density));
    }

    // Volume (m³) = Weight (kg) / Density (kg/m³)
    let target_volume = target_weight / material.density;
    solve_dimension_from_volume(shape, fixed_vars, free_var_name, target_volume)
}

/// Solves for the remaining free dimension from a target cost in INR using material rate.
pub fn solve_dimension_from_cost(
    shape: &shape::Model,
    material: &material::Model,
    latest_rate_inr_per_kg: f64,
    fixed_vars: &HashMap<String, f64>,
    free_var_name: &str,
    target_cost: f64,
) -> Result<f64, SolverError> {
    if target_cost <= 0.0 || target_cost.is_nan() || target_cost.is_infinite() {
        return Err(SolverError::InvalidTargetCost(target_cost));
    }
    if latest_rate_inr_per_kg <= 0.0
        || latest_rate_inr_per_kg.is_nan()
        || latest_rate_inr_per_kg.is_infinite()
    {
        return Err(SolverError::InvalidRate(latest_rate_inr_per_kg));
    }

    // Weight (kg) = Cost (INR) / Rate (INR/kg)
    let target_weight = target_cost / latest_rate_inr_per_kg;
    solve_dimension_from_weight(shape, material, fixed_vars, free_var_name, target_weight)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_solve_bar_length_from_weight() {
        let box_shape = shape::Model {
            id: 1,
            created_at: None,
            updated_at: None,
            name: "Box / Plate".into(),
            openscad_code: "cube([length, width, thickness]);".into(),
            variable_names: json!(["length", "width", "thickness"]),
        };

        let steel = material::Model {
            id: 1,
            created_at: None,
            updated_at: None,
            name: "Mild Steel".into(),
            density: 7850.0, // kg/m³
        };

        // 2.5mm x 3.5mm bar: width = 2.5mm, thickness = 3.5mm
        let mut fixed = HashMap::new();
        fixed.insert("width".into(), 2.5);
        fixed.insert("thickness".into(), 3.5);

        // Cross-sectional area = 2.5 * 3.5 = 8.75 mm²
        // Mass per 1000mm (1m) = 8.75 * 1000 * 1e-9 * 7850 = 0.0686875 kg
        let target_weight = 0.0686875;
        let solved_len = solve_dimension_from_weight(&box_shape, &steel, &fixed, "length", target_weight).unwrap();
        assert!((solved_len - 1000.0).abs() < 1e-4, "Expected 1000mm, got {}", solved_len);

        // If target cost = ₹10 at ₹100/kg -> target weight = 0.1 kg
        // Expected length in mm = (0.1 / (7850 * 8.75e-6)) * 1000 = 1455.88mm
        let solved_from_cost = solve_dimension_from_cost(&box_shape, &steel, 100.0, &fixed, "length", 10.0).unwrap();
        let expected_cost_len = (0.1 / (7850.0 * 8.75 * 1e-6)) * 1000.0;
        assert!((solved_from_cost - expected_cost_len).abs() < 1e-3);
    }

    #[test]
    fn test_solve_cylinder_length_from_weight() {
        let cyl_shape = shape::Model {
            id: 2,
            created_at: None,
            updated_at: None,
            name: "Cylinder / Round Bar".into(),
            openscad_code: "cylinder(h=length, r=diameter/2);".into(),
            variable_names: json!(["diameter", "length"]),
        };

        let steel = material::Model {
            id: 1,
            created_at: None,
            updated_at: None,
            name: "Mild Steel".into(),
            density: 7850.0,
        };

        // 50mm diameter round bar
        let mut fixed = HashMap::new();
        fixed.insert("diameter".into(), 50.0);

        // Area = pi * (25)^2 = 1963.4954 mm²
        // Mass per 1000mm (1m) = 1963.4954 * 1000 * 1e-9 * 7850 = 15.4134396 kg
        let target_weight = 15.4134396;
        let solved_len = solve_dimension_from_weight(&cyl_shape, &steel, &fixed, "length", target_weight).unwrap();
        assert!((solved_len - 1000.0).abs() < 1e-3, "Expected 1000mm, got {}", solved_len);
    }
}
