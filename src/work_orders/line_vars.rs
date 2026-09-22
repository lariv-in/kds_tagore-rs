//! Shared display helpers for material-line variable JSON.

use std::collections::HashMap;

use sea_orm::entity::prelude::Json;

use crate::formula::{self, VariableSchema, VariableValues, parse_schema, parse_values_from_json};

fn json_display(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".into(),
        other => other.to_string(),
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

pub fn format_variables_display(schema_json: &Json, values_json: &Json, extra: &Json) -> String {
    let schema: VariableSchema = parse_schema(schema_json).unwrap_or_default();
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
