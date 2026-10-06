//! Quantity on a delivery challan line: length (mm), weight (kg), or a whole number.
//!
//! Kinds match work-order formula variables (`length`, `weight`, `quantity`).

use lariv_core::length::{LengthUnit, format_length_label, parse_length_unit};
use rust_decimal::Decimal;
use std::str::FromStr;

use kds_plugin_formula::{VariableType, parse_quantity, parse_weight};

/// One line's quantity after validation, ready to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedQty {
    pub kind: VariableType,
    pub length_mm: Option<Decimal>,
    pub length_unit: Option<String>,
    pub weight_kg: Option<Decimal>,
    pub number: Option<i64>,
}

/// Form payload for one line's quantity fields.
#[derive(Clone, Debug, Default)]
pub struct QtyInput {
    pub kind: String,
    pub qty_mm: String,
    pub qty_unit: String,
    pub qty_weight: String,
    pub qty_number: String,
}

pub fn parse_qty(input: &QtyInput) -> Result<ParsedQty, String> {
    let kind = VariableType::parse_name(input.kind.trim())
        .filter(|k| !matches!(k, VariableType::Duration))
        .ok_or_else(|| "Quantity kind must be length, weight, or number.".to_string())?;
    match kind {
        VariableType::Length => {
            let raw = input.qty_mm.trim();
            if raw.is_empty() {
                return Err("Enter a length.".into());
            }
            let mm = Decimal::from_str(raw).map_err(|_| format!("Invalid length `{raw}`"))?;
            if mm.is_sign_negative() {
                return Err("Length cannot be negative.".into());
            }
            let unit = length_unit_token(&input.qty_unit);
            Ok(ParsedQty {
                kind,
                length_mm: Some(mm),
                length_unit: Some(unit),
                weight_kg: None,
                number: None,
            })
        }
        VariableType::Weight => {
            let kg = parse_weight(input.qty_weight.trim()).map_err(|e| e.to_string())?;
            if kg.is_sign_negative() {
                return Err("Weight cannot be negative.".into());
            }
            Ok(ParsedQty {
                kind,
                length_mm: None,
                length_unit: None,
                weight_kg: Some(kg),
                number: None,
            })
        }
        VariableType::Quantity => {
            let n = parse_quantity(input.qty_number.trim()).map_err(|e| e.to_string())?;
            if n < 0 {
                return Err("Quantity cannot be negative.".into());
            }
            Ok(ParsedQty {
                kind,
                length_mm: None,
                length_unit: None,
                weight_kg: None,
                number: Some(n),
            })
        }
        VariableType::Duration => unreachable!("duration is rejected above"),
    }
}

fn length_unit_token(raw: &str) -> String {
    parse_length_unit(raw)
        .unwrap_or(LengthUnit::Millimetre)
        .as_str()
        .to_string()
}

pub fn format_stored_qty(
    kind: &str,
    length_mm: Option<Decimal>,
    length_unit: Option<&str>,
    weight_kg: Option<Decimal>,
    number: Option<i64>,
) -> String {
    match VariableType::parse_name(kind) {
        Some(VariableType::Length) => {
            let Some(mm) = length_mm else {
                return "—".into();
            };
            let unit =
                parse_length_unit(length_unit.unwrap_or("mm")).unwrap_or(LengthUnit::Millimetre);
            format_length_label(&mm.normalize().to_string(), unit)
        }
        Some(VariableType::Weight) => weight_kg
            .map(|kg| format!("{} kg", kg.normalize()))
            .unwrap_or_else(|| "—".into()),
        Some(VariableType::Quantity) => number.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
        _ => "—".into(),
    }
}

pub fn kind_label(kind: &str) -> &'static str {
    match VariableType::parse_name(kind) {
        Some(VariableType::Length) => "Length",
        Some(VariableType::Weight) => "Weight",
        Some(VariableType::Quantity) => "Number",
        _ => "Quantity",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_stores_millimetres_and_display_unit() {
        let parsed = parse_qty(&QtyInput {
            kind: "length".into(),
            qty_mm: "1000".into(),
            qty_unit: "cm".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(parsed.kind, VariableType::Length);
        assert_eq!(parsed.length_mm.unwrap().normalize().to_string(), "1000");
        assert_eq!(parsed.length_unit.as_deref(), Some("cm"));
        assert_eq!(
            format_stored_qty(
                "length",
                parsed.length_mm,
                parsed.length_unit.as_deref(),
                None,
                None
            ),
            "100 cm"
        );
    }

    #[test]
    fn weight_and_number() {
        let weight = parse_qty(&QtyInput {
            kind: "weight".into(),
            qty_weight: "1.5".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(weight.weight_kg.unwrap().normalize().to_string(), "1.5");
        assert_eq!(
            format_stored_qty("weight", None, None, weight.weight_kg, None),
            "1.5 kg"
        );

        let number = parse_qty(&QtyInput {
            kind: "quantity".into(),
            qty_number: "12".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(number.number, Some(12));
        assert_eq!(
            format_stored_qty("quantity", None, None, None, Some(12)),
            "12"
        );
    }

    #[test]
    fn rejects_duration_and_blank_length() {
        assert!(
            parse_qty(&QtyInput {
                kind: "duration".into(),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            parse_qty(&QtyInput {
                kind: "length".into(),
                ..Default::default()
            })
            .is_err()
        );
    }
}
