//! Shared display helpers for material-line variable JSON.

use std::collections::HashMap;

use sea_orm::entity::prelude::Json;

use crate::formula::{
    self, VariableSchema, VariableType, VariableValues, parse_schema, parse_values_from_json,
};

const NANOS_PER_SECOND: i64 = 1_000_000_000;

fn json_display(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".into(),
        other => other.to_string(),
    }
}

fn json_as_i64(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        serde_json::Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<i64>().ok()
            }
        }
        _ => None,
    }
}

pub fn extra_data_str(extra: &Json) -> String {
    serde_json::to_string(extra).unwrap_or_else(|_| "{}".into())
}

pub fn dim_units_map(extra: &Json) -> HashMap<String, String> {
    if let serde_json::Value::Object(m) = extra {
        if let Some(serde_json::Value::Object(units)) = m.get("dim_units") {
            let mut map = HashMap::new();
            for (k, v) in units {
                if let Some(s) = v.as_str() {
                    map.insert(k.clone(), s.to_string());
                }
            }
            return map;
        } else if let Some(serde_json::Value::String(u)) = m.get("dim_unit") {
            let mut map = HashMap::new();
            map.insert("default".into(), u.clone());
            return map;
        }
    }
    HashMap::new()
}

pub fn dim_unit(extra: &Json) -> String {
    if let serde_json::Value::Object(m) = extra {
        if let Some(serde_json::Value::String(u)) = m.get("dim_unit") {
            return u.clone();
        }
    }
    "mm".into()
}

fn infer_type_from_name(name: &str) -> Option<VariableType> {
    let lower = name.to_ascii_lowercase();
    if lower.contains("duration") || lower == "time" {
        Some(VariableType::Duration)
    } else if lower.contains("weight") || lower.contains("mass") {
        Some(VariableType::Weight)
    } else if lower == "qty" || lower.contains("quantity") || lower == "count" {
        Some(VariableType::Quantity)
    } else if lower.contains("length")
        || lower.contains("width")
        || lower.contains("height")
        || lower.contains("thick")
    {
        Some(VariableType::Length)
    } else {
        None
    }
}

/// Infer types from stored JSON when a schema was not supplied.
///
/// `values_to_json` stores length/weight as decimal strings and duration as a
/// nanosecond integer. Integers below one second are treated as quantity unless
/// the variable name indicates otherwise.
fn infer_schema_from_stored_values(values_json: &Json, extra: &Json) -> VariableSchema {
    let mut schema = VariableSchema::new();
    let Some(obj) = values_json.as_object() else {
        return schema;
    };
    let units = dim_units_map(extra);
    for (k, v) in obj {
        let ty = if let Some(named) = infer_type_from_name(k) {
            named
        } else if units.contains_key(k) {
            VariableType::Length
        } else {
            match v {
                serde_json::Value::Number(n) => {
                    if n.as_i64().is_some_and(|n| n.abs() >= NANOS_PER_SECOND) {
                        VariableType::Duration
                    } else {
                        VariableType::Quantity
                    }
                }
                serde_json::Value::String(s) => {
                    let t = s.trim();
                    if t.chars().any(|c| c.is_ascii_alphabetic())
                        && formula::parse_duration_nanos(t).is_ok()
                    {
                        VariableType::Duration
                    } else {
                        VariableType::Length
                    }
                }
                _ => VariableType::Quantity,
            }
        };
        schema.insert(k.clone(), ty);
    }
    schema
}

fn resolved_schema(schema_json: &Json, values_json: &Json, extra: &Json) -> VariableSchema {
    let parsed: VariableSchema = parse_schema(schema_json).unwrap_or_default();
    if parsed.is_empty() {
        infer_schema_from_stored_values(values_json, extra)
    } else {
        parsed
    }
}

/// Rewrite stored values so form fields show typed units instead of raw JSON.
pub fn values_json_for_form(schema_json: &Json, values_json: &Json) -> Json {
    let schema = resolved_schema(schema_json, values_json, &serde_json::json!({}));
    let mut out = match values_json {
        serde_json::Value::Object(m) => serde_json::Value::Object(m.clone()),
        _ => return values_json.clone(),
    };
    let Some(obj) = out.as_object_mut() else {
        return out;
    };
    for (name, ty) in &schema {
        if *ty != VariableType::Duration {
            continue;
        }
        let Some(v) = obj.get(name).cloned() else {
            continue;
        };
        if let Some(n) = json_as_i64(&v) {
            obj.insert(
                name.clone(),
                serde_json::Value::String(lariv_rs::duration::format_duration(n)),
            );
        }
    }
    out
}

pub fn format_variables_display(schema_json: &Json, values_json: &Json, extra: &Json) -> String {
    let schema = resolved_schema(schema_json, values_json, extra);
    let units = dim_units_map(extra);
    let values: VariableValues =
        parse_values_from_json(&schema, values_json, &units).unwrap_or_default();
    if values.is_empty() {
        if let serde_json::Value::Object(m) = values_json {
            if m.is_empty() {
                return "-".into();
            }
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|k| format!("{k}: {}", json_display(&m[k])))
                .collect();
            return parts.join(", ");
        }
        return "-".into();
    }
    formula::format_values_display(&schema, &values, &units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_nanos_render_as_hours() {
        let schema = serde_json::json!({"duration": "duration"});
        let values = serde_json::json!({"duration": 7_200_000_000_000i64});
        let out = format_variables_display(&schema, &values, &serde_json::json!({}));
        assert_eq!(out, "duration: 2 hours");
    }

    #[test]
    fn duration_nanos_render_without_schema() {
        let values = serde_json::json!({"duration": 7_200_000_000_000i64});
        let out = format_variables_display(&serde_json::json!({}), &values, &serde_json::json!({}));
        assert_eq!(out, "duration: 2 hours");
    }

    #[test]
    fn duration_form_value_is_human() {
        let schema = serde_json::json!({"duration": "duration"});
        let values = serde_json::json!({"duration": 7_200_000_000_000i64});
        let out = values_json_for_form(&schema, &values);
        assert_eq!(out["duration"], "2 hours");
    }

    #[test]
    fn length_and_quantity_render_with_units() {
        let schema = serde_json::json!({"length": "length", "qty": "quantity"});
        let values = serde_json::json!({"length": "1000", "qty": 2});
        let out = format_variables_display(&schema, &values, &serde_json::json!({}));
        assert!(out.contains("length: 1000 mm"), "{out}");
        assert!(out.contains("qty: 2"), "{out}");
    }
}
