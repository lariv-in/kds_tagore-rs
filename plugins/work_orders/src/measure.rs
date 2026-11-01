//! Quantity parsing against a part's measure kind and unit.

use rust_decimal::Decimal;
use std::str::FromStr;

use lariv_formula::{Measure, VariableType, convert_measure};

pub const PART_KINDS: [VariableType; 4] = [
    VariableType::Length,
    VariableType::Weight,
    VariableType::Quantity,
    VariableType::Decimal,
];

pub fn kind_choices() -> Vec<(String, String)> {
    PART_KINDS
        .iter()
        .copied()
        .map(|kind| (kind.as_str().to_string(), kind.label().to_string()))
        .collect()
}

pub fn parse_kind(raw: &str) -> Result<VariableType, String> {
    let kind = VariableType::parse_name(raw).ok_or_else(|| "Choose a unit kind.".to_string())?;
    if PART_KINDS.contains(&kind) {
        Ok(kind)
    } else {
        Err("Choose length, weight, quantity, or decimal.".to_string())
    }
}

pub fn normalize_unit(kind: VariableType, unit: &str) -> Result<String, String> {
    let trimmed = unit.trim();
    if let Some(choices) = kind.unit_choices() {
        if trimmed.is_empty() {
            return Ok(kind.default_accepted_unit().to_string());
        }
        for (id, _) in choices {
            if id.eq_ignore_ascii_case(trimmed) {
                return Ok((*id).to_string());
            }
        }
        return Err(format!("Use a {} unit.", kind.label().to_lowercase()));
    }
    let fixed = kind.default_accepted_unit();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case(fixed) {
        Ok(fixed.to_string())
    } else {
        Err(format!("{} uses {fixed}.", kind.label()))
    }
}

/// Store the entered value and display unit once it converts into `accepted_unit`.
pub fn parse_line(
    kind: VariableType,
    accepted_unit: &str,
    value: &str,
    unit: &str,
) -> Result<(Decimal, String), String> {
    let qty = Decimal::from_str(value.trim()).map_err(|_| "Enter a quantity.".to_string())?;
    if qty.is_sign_negative() {
        return Err("Quantity cannot be negative.".to_string());
    }
    let display_unit = normalize_unit(kind, unit)?;
    let measure = Measure {
        kind,
        value: qty,
        unit: display_unit.clone(),
    };
    convert_measure(&measure, accepted_unit).map_err(|err| err.to_string())?;
    Ok((qty, display_unit))
}

pub fn in_part_unit(
    kind: VariableType,
    accepted_unit: &str,
    value: Decimal,
    unit: &str,
) -> Result<Decimal, String> {
    let measure = Measure {
        kind,
        value,
        unit: unit.to_string(),
    };
    convert_measure(&measure, accepted_unit).map_err(|err| err.to_string())
}

pub fn part_measure(kind_raw: &str, unit_raw: &str) -> Result<(VariableType, String), String> {
    let kind = parse_kind(kind_raw)?;
    let unit = normalize_unit(kind, unit_raw)?;
    Ok((kind, unit))
}
