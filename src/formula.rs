//! Typed variables and Rune formula evaluation for component/machine costing.
//!
//! Numerics are [`Decimal`] end-to-end. Duration is stored as nanoseconds and
//! exposed to Rune as `Decimal` seconds. Quantity is `i64` in Rune.

use std::collections::HashMap;
use std::fmt::{self, Write as _};
use std::str::FromStr;
use std::sync::Arc;

use lariv_rs::duration::parse_duration;
use rune::termcolor::NoColor;
use rune::{Any, Context, Diagnostics, FromValue, Module, Source, Sources, Value, Vm};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

const NANOS_PER_SECOND: i64 = 1_000_000_000;
const MAX_SOURCE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, thiserror::Error)]
pub enum FormulaError {
    #[error("{0}")]
    Message(String),
}

impl FormulaError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    Length,
    Weight,
    Duration,
    Quantity,
}

impl VariableType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Weight => "weight",
            Self::Duration => "duration",
            Self::Quantity => "quantity",
        }
    }

    pub fn parse_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "length" => Some(Self::Length),
            "weight" => Some(Self::Weight),
            "duration" => Some(Self::Duration),
            "quantity" => Some(Self::Quantity),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableValue {
    Length(Decimal),
    Weight(Decimal),
    DurationNanos(i64),
    Quantity(i64),
}

impl VariableValue {
    pub fn ty(&self) -> VariableType {
        match self {
            Self::Length(_) => VariableType::Length,
            Self::Weight(_) => VariableType::Weight,
            Self::DurationNanos(_) => VariableType::Duration,
            Self::Quantity(_) => VariableType::Quantity,
        }
    }

    pub fn standard_sample(ty: VariableType) -> Self {
        match ty {
            VariableType::Length => Self::Length(Decimal::ONE),
            VariableType::Weight => Self::Weight(Decimal::ONE),
            VariableType::Duration => Self::DurationNanos(NANOS_PER_SECOND),
            VariableType::Quantity => Self::Quantity(1),
        }
    }

    fn to_json_value(&self) -> serde_json::Value {
        match self {
            Self::Length(d) | Self::Weight(d) => {
                serde_json::Value::String(d.normalize().to_string())
            }
            Self::DurationNanos(n) => serde_json::Value::Number((*n).into()),
            Self::Quantity(n) => serde_json::Value::Number((*n).into()),
        }
    }
}

pub type VariableSchema = HashMap<String, VariableType>;
pub type VariableValues = HashMap<String, VariableValue>;

const RUNE_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "yield", "select", "is", "not", "and", "or",
];

pub fn is_valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    !RUNE_KEYWORDS.contains(&name)
}

pub fn parse_schema(value: &serde_json::Value) -> Result<VariableSchema, FormulaError> {
    let obj = value
        .as_object()
        .ok_or_else(|| FormulaError::msg("variables schema must be a JSON object"))?;
    let mut out = VariableSchema::new();
    for (name, ty) in obj {
        let name = name.trim();
        if !is_valid_ident(name) {
            return Err(FormulaError::msg(format!(
                "variable name `{name}` is not a valid Rune identifier"
            )));
        }
        let ty_s = ty
            .as_str()
            .ok_or_else(|| FormulaError::msg(format!("variable `{name}` type must be a string")))?;
        let parsed = VariableType::parse_name(ty_s).ok_or_else(|| {
            FormulaError::msg(format!(
                "unknown variable type `{ty_s}` for `{name}` (expected length, weight, duration, quantity)"
            ))
        })?;
        out.insert(name.to_string(), parsed);
    }
    Ok(out)
}

/// Parse `"name:type"` rows from a List widget into a schema.
pub fn parse_schema_list(entries: &[String]) -> Result<VariableSchema, FormulaError> {
    let mut lines = Vec::new();
    for raw in entries {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        lines.push(s.to_string());
    }
    let mut map = serde_json::Map::new();
    for s in lines {
        let Some((name, ty)) = s.split_once(':') else {
            return Err(FormulaError::msg(format!(
                "variable `{s}` must be in name:type form (types: length, weight, duration, quantity)"
            )));
        };
        map.insert(
            name.trim().to_string(),
            serde_json::Value::String(ty.trim().to_string()),
        );
    }
    parse_schema(&serde_json::Value::Object(map))
}

/// Render a schema as sorted `"name:type"` rows for List widgets.
pub fn schema_to_entries(schema: &VariableSchema) -> Vec<String> {
    let mut keys: Vec<_> = schema.keys().cloned().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|k| schema.get(&k).map(|ty| format!("{}:{}", k, ty.as_str())))
        .collect()
}

pub fn schema_to_json(schema: &VariableSchema) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    let mut keys: Vec<_> = schema.keys().cloned().collect();
    keys.sort();
    for k in keys {
        if let Some(ty) = schema.get(&k) {
            map.insert(k, serde_json::Value::String(ty.as_str().to_string()));
        }
    }
    serde_json::Value::Object(map)
}

pub fn standard_sample_values(schema: &VariableSchema) -> VariableValues {
    schema
        .iter()
        .map(|(name, ty)| (name.clone(), VariableValue::standard_sample(*ty)))
        .collect()
}

pub fn values_to_json(values: &VariableValues) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in values {
        map.insert(k.clone(), v.to_json_value());
    }
    serde_json::Value::Object(map)
}

pub fn parse_values_from_json(
    schema: &VariableSchema,
    raw: &serde_json::Value,
    length_units: &HashMap<String, String>,
) -> Result<VariableValues, FormulaError> {
    let obj = match raw {
        serde_json::Value::Object(m) => m,
        serde_json::Value::String(s) => {
            let parsed: serde_json::Value =
                serde_json::from_str(s.trim()).unwrap_or(serde_json::Value::Null);
            return parse_values_from_json(schema, &parsed, length_units);
        }
        _ => {
            return Err(FormulaError::msg("variable values must be a JSON object"));
        }
    };
    let mut out = VariableValues::new();
    let mut required = Vec::new();
    let mut invalid = Vec::new();
    let mut names: Vec<&String> = schema.keys().collect();
    names.sort();
    for name in names {
        let ty = schema[name];
        let parsed = match obj.get(name) {
            None => {
                required.push(name.clone());
                continue;
            }
            Some(v) => parse_typed_value(ty, json_to_str(v).as_str(), name, length_units),
        };
        match parsed {
            Ok(val) => {
                out.insert(name.clone(), val);
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.ends_with(" is required") {
                    required.push(name.clone());
                } else {
                    invalid.push(format!("`{name}`: {msg}"));
                }
            }
        }
    }
    if required.is_empty() && invalid.is_empty() {
        return Ok(out);
    }
    Err(format_variable_errors(&required, &invalid))
}

fn parse_typed_value(
    ty: VariableType,
    raw: &str,
    name: &str,
    length_units: &HashMap<String, String>,
) -> Result<VariableValue, FormulaError> {
    match ty {
        VariableType::Length => {
            let unit = length_units.get(name).map(String::as_str).unwrap_or("mm");
            Ok(VariableValue::Length(parse_length(raw, unit)?))
        }
        VariableType::Weight => Ok(VariableValue::Weight(parse_weight(raw)?)),
        VariableType::Duration => Ok(VariableValue::DurationNanos(parse_duration_nanos(raw)?)),
        VariableType::Quantity => Ok(VariableValue::Quantity(parse_quantity(raw)?)),
    }
}

fn backtick_names(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => format!("`{}`", names[0]),
        2 => format!("`{}` and `{}`", names[0], names[1]),
        n => {
            let head = names[..n - 1]
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{head}, and `{}`", names[n - 1])
        }
    }
}

fn format_variable_errors(required: &[String], invalid: &[String]) -> FormulaError {
    let mut parts = Vec::new();
    if !required.is_empty() {
        let names = backtick_names(required);
        if required.len() == 1 {
            parts.push(format!("{names} is required"));
        } else {
            parts.push(format!("{names} are required"));
        }
    }
    parts.extend(invalid.iter().cloned());
    FormulaError::msg(parts.join("; "))
}

fn json_to_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn length_factor_mm(unit: &str) -> Result<Decimal, FormulaError> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "" | "mm" => Ok(Decimal::ONE),
        "cm" => Ok(Decimal::from(10)),
        "m" => Ok(Decimal::from(1000)),
        "km" => Ok(Decimal::from(1_000_000)),
        "in" | "inch" => Ok(Decimal::new(254, 1)),
        "ft" | "foot" | "feet" => Ok(Decimal::new(3048, 1)),
        other => Err(FormulaError::msg(format!("unknown length unit `{other}`"))),
    }
}

pub fn parse_length(raw: &str, unit: &str) -> Result<Decimal, FormulaError> {
    let n = parse_decimal_str(raw)?;
    Ok(n * length_factor_mm(unit)?)
}

pub fn parse_weight(raw: &str) -> Result<Decimal, FormulaError> {
    parse_decimal_str(raw)
}

pub fn parse_quantity(raw: &str) -> Result<i64, FormulaError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(FormulaError::msg("quantity is required"));
    }
    s.parse::<i64>()
        .map_err(|_| FormulaError::msg(format!("invalid quantity `{s}`")))
}

pub fn parse_duration_nanos(raw: &str) -> Result<i64, FormulaError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(FormulaError::msg("duration is required"));
    }
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }
    parse_duration(s).map_err(|e| FormulaError::msg(e))
}

fn parse_decimal_str(raw: &str) -> Result<Decimal, FormulaError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(FormulaError::msg("number is required"));
    }
    Decimal::from_str(s).map_err(|_| FormulaError::msg(format!("invalid decimal `{s}`")))
}

pub fn duration_nanos_to_seconds(nanos: i64) -> Decimal {
    Decimal::from(nanos) / Decimal::from(NANOS_PER_SECOND)
}

/// Compile and evaluate `formula` with the given schema-ordered values.
pub fn eval_formula(
    schema: &VariableSchema,
    formula: &str,
    values: &VariableValues,
) -> Result<Decimal, FormulaError> {
    if formula.trim().is_empty() {
        return Err(FormulaError::msg("formula is empty"));
    }
    if formula.len() > MAX_SOURCE_BYTES {
        return Err(FormulaError::msg("formula exceeds maximum size"));
    }
    for name in schema.keys() {
        if !values.contains_key(name) {
            return Err(FormulaError::msg(format!(
                "missing value for variable `{name}`"
            )));
        }
    }
    let source = wrap_formula(schema, formula, values)?;
    run_compute(&source)
}

/// Evaluate a formula using standard sample values (1 mm, 1 kg, 1 s, qty 1).
pub fn validate_formula(schema: &VariableSchema, formula: &str) -> Result<Decimal, FormulaError> {
    let samples = standard_sample_values(schema);
    eval_formula(schema, formula, &samples)
        .map_err(|e| FormulaError::msg(format!("formula failed with standard sample values: {e}")))
}

fn wrap_formula(
    schema: &VariableSchema,
    formula: &str,
    values: &VariableValues,
) -> Result<String, FormulaError> {
    let trimmed = formula.trim();
    if trimmed.contains("pub fn compute") {
        return Ok(trimmed.to_string());
    }
    let mut keys: Vec<&String> = schema.keys().collect();
    keys.sort();
    let mut body = String::new();
    for name in keys {
        let val = values
            .get(name)
            .ok_or_else(|| FormulaError::msg(format!("missing value for variable `{name}`")))?;
        match val {
            VariableValue::Length(d) | VariableValue::Weight(d) => {
                let _ = writeln!(body, "    let {name} = decimal(\"{}\");", d.normalize());
            }
            VariableValue::DurationNanos(n) => {
                let secs = duration_nanos_to_seconds(*n);
                let _ = writeln!(body, "    let {name} = decimal(\"{}\");", secs.normalize());
            }
            VariableValue::Quantity(n) => {
                let _ = writeln!(body, "    let {name} = decimal(\"{n}\");");
            }
        }
    }
    Ok(format!("pub fn compute() {{\n{body}    {trimmed}\n}}\n"))
}

fn run_compute(source: &str) -> Result<Decimal, FormulaError> {
    let mut context = Context::with_config(false).map_err(|e| FormulaError::msg(e.to_string()))?;
    context
        .install(decimal_module().map_err(|e| FormulaError::msg(e.to_string()))?)
        .map_err(|e| FormulaError::msg(e.to_string()))?;
    let runtime = Arc::new(
        context
            .runtime()
            .map_err(|e| FormulaError::msg(e.to_string()))?,
    );
    let mut sources = Sources::new();
    sources
        .insert(Source::new("formula", source).map_err(|e| FormulaError::msg(e.to_string()))?)
        .map_err(|e| FormulaError::msg(e.to_string()))?;
    let mut diagnostics = Diagnostics::new();
    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();
    if diagnostics.has_error() {
        return Err(FormulaError::msg(format_diagnostics(
            &diagnostics,
            &sources,
        )));
    }
    let unit = result.map_err(|e| FormulaError::msg(e.to_string()))?;
    let mut vm = Vm::new(runtime, Arc::new(unit));
    let output = vm
        .call(["compute"], ())
        .map_err(|e| FormulaError::msg(format_vm_error(&e, &sources)))?;
    value_to_decimal(&output)
}

fn format_diagnostics(diagnostics: &Diagnostics, sources: &Sources) -> String {
    match emit_to_string(|out| diagnostics.emit(out, sources)) {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) | Err(_) => "compilation failed".into(),
    }
}

fn format_vm_error(error: &rune::runtime::VmError, sources: &Sources) -> String {
    match emit_to_string(|out| error.emit(out, sources)) {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) | Err(_) => error.to_string(),
    }
}

fn emit_to_string<F>(emit: F) -> Result<String, String>
where
    F: FnOnce(&mut NoColor<Vec<u8>>) -> Result<(), rune::diagnostics::EmitError>,
{
    let mut buf = NoColor::new(Vec::new());
    emit(&mut buf).map_err(|e| e.to_string())?;
    String::from_utf8(buf.into_inner()).map_err(|e| e.to_string())
}

fn value_to_decimal(v: &Value) -> Result<Decimal, FormulaError> {
    if let Ok(d) = RuneDecimal::from_value(v.clone()) {
        return Ok(d.0);
    }
    if let Ok(n) = i64::from_value(v.clone()) {
        return Ok(Decimal::from(n));
    }
    Err(FormulaError::msg(
        "formula must return a Decimal (use decimal(\"...\") for fractional constants; integer results are accepted)",
    ))
}

#[derive(Any, Clone, Debug)]
struct RuneDecimal(Decimal);

fn value_as_decimal(value: Value) -> Result<Decimal, String> {
    if let Ok(d) = RuneDecimal::from_value(value.clone()) {
        return Ok(d.0);
    }
    if let Ok(n) = i64::from_value(value) {
        return Ok(Decimal::from(n));
    }
    Err("expected Decimal or integer (f64 is not allowed)".into())
}

#[rune::function]
fn decimal(s: &str) -> RuneDecimal {
    RuneDecimal(Decimal::from_str(s).expect("valid decimal literal"))
}

#[rune::function(instance, protocol = ADD)]
fn add(a: &RuneDecimal, b: Value) -> RuneDecimal {
    RuneDecimal(a.0 + value_as_decimal(b).expect("add operand"))
}

#[rune::function(instance, protocol = SUB)]
fn sub(a: &RuneDecimal, b: Value) -> RuneDecimal {
    RuneDecimal(a.0 - value_as_decimal(b).expect("sub operand"))
}

#[rune::function(instance, protocol = MUL)]
fn mul(a: &RuneDecimal, b: Value) -> RuneDecimal {
    RuneDecimal(a.0 * value_as_decimal(b).expect("mul operand"))
}

#[rune::function(instance, protocol = DIV)]
fn div(a: &RuneDecimal, b: Value) -> RuneDecimal {
    let rhs = value_as_decimal(b).expect("div operand");
    if rhs.is_zero() {
        panic!("division by zero");
    }
    RuneDecimal(a.0 / rhs)
}

fn decimal_module() -> Result<Module, rune::ContextError> {
    let mut module = Module::new();
    module.ty::<RuneDecimal>()?;
    module.function_meta(decimal)?;
    module.function_meta(add)?;
    module.function_meta(sub)?;
    module.function_meta(mul)?;
    module.function_meta(div)?;
    Ok(module)
}

pub fn format_values_display(
    schema: &VariableSchema,
    values: &VariableValues,
    length_units: &HashMap<String, String>,
) -> String {
    if values.is_empty() {
        return "-".into();
    }
    let mut keys: Vec<&String> = values.keys().collect();
    keys.sort();
    let mut parts = Vec::new();
    for k in keys {
        let Some(v) = values.get(k) else {
            continue;
        };
        let expected = schema.get(k).copied();
        match v {
            VariableValue::Length(mm) => {
                let unit = length_units.get(k).map(String::as_str).unwrap_or("mm");
                if unit == "mm" {
                    parts.push(format!("{k}: {} mm", mm.normalize()));
                } else if let Ok(factor) = length_factor_mm(unit) {
                    if !factor.is_zero() {
                        let user = (*mm / factor).normalize();
                        parts.push(format!("{k}: {user} {unit} ({} mm)", mm.normalize()));
                    } else {
                        parts.push(format!("{k}: {} mm", mm.normalize()));
                    }
                } else {
                    parts.push(format!("{k}: {} mm", mm.normalize()));
                }
            }
            VariableValue::Weight(kg) => parts.push(format!("{k}: {} kg", kg.normalize())),
            VariableValue::DurationNanos(n) => {
                parts.push(format!("{k}: {}", lariv_rs::duration::format_duration(*n)));
            }
            VariableValue::Quantity(n) => parts.push(format!("{k}: {n}")),
        }
        let _ = expected;
    }
    parts.join(", ")
}

impl fmt::Display for VariableType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(pairs: &[(&str, VariableType)]) -> VariableSchema {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn ident_validation() {
        assert!(is_valid_ident("length"));
        assert!(is_valid_ident("_x"));
        assert!(!is_valid_ident(""));
        assert!(!is_valid_ident("1x"));
        assert!(!is_valid_ident("let"));
        assert!(!is_valid_ident("fn"));
    }

    #[test]
    fn eval_length_times_qty() {
        let schema = schema(&[
            ("length", VariableType::Length),
            ("qty", VariableType::Quantity),
        ]);
        let mut values = VariableValues::new();
        values.insert("length".into(), VariableValue::Length(Decimal::from(10)));
        values.insert("qty".into(), VariableValue::Quantity(3));
        let out = eval_formula(&schema, "length * qty", &values).unwrap();
        assert_eq!(out, Decimal::from(30));
    }

    #[test]
    fn eval_decimal_division_no_f64() {
        let schema = schema(&[("duration", VariableType::Duration)]);
        let mut values = VariableValues::new();
        values.insert(
            "duration".into(),
            VariableValue::DurationNanos(3_600_000_000_000),
        );
        let out = eval_formula(&schema, "duration / 3600 * 950", &values).unwrap();
        assert_eq!(out, Decimal::from(950));
    }

    #[test]
    fn validate_formula_uses_samples() {
        let schema = schema(&[
            ("length", VariableType::Length),
            ("qty", VariableType::Quantity),
        ]);
        let out = validate_formula(&schema, "length * qty * 85").unwrap();
        assert_eq!(out, Decimal::from(85));
    }

    #[test]
    fn validate_rejects_bad_formula() {
        let schema = schema(&[("length", VariableType::Length)]);
        assert!(validate_formula(&schema, "not_a_variable").is_err());
    }

    #[test]
    fn length_unit_conversion() {
        let mm = parse_length("2", "cm").unwrap();
        assert_eq!(mm, Decimal::from(20));
        let mm = parse_length("1", "in").unwrap();
        assert_eq!(mm, Decimal::new(254, 1));
    }

    #[test]
    fn parse_values_names_required_variables() {
        let schema = schema(&[
            ("length", VariableType::Length),
            ("qty", VariableType::Quantity),
        ]);
        let err = parse_values_from_json(&schema, &serde_json::json!({}), &HashMap::new())
            .unwrap_err()
            .to_string();
        assert!(err.contains("`length`"), "{err}");
        assert!(err.contains("`qty`"), "{err}");
        assert!(err.contains("are required"), "{err}");
    }

    #[test]
    fn parse_values_names_invalid_variables() {
        let schema = schema(&[("qty", VariableType::Quantity)]);
        let err = parse_values_from_json(
            &schema,
            &serde_json::json!({"qty": "nope"}),
            &HashMap::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("`qty`"), "{err}");
        assert!(err.contains("invalid quantity"), "{err}");
    }

    #[test]
    fn sample_values() {
        let schema = schema(&[
            ("l", VariableType::Length),
            ("w", VariableType::Weight),
            ("d", VariableType::Duration),
            ("q", VariableType::Quantity),
        ]);
        let s = standard_sample_values(&schema);
        assert_eq!(s.get("l"), Some(&VariableValue::Length(Decimal::ONE)));
        assert_eq!(s.get("w"), Some(&VariableValue::Weight(Decimal::ONE)));
        assert_eq!(
            s.get("d"),
            Some(&VariableValue::DurationNanos(NANOS_PER_SECOND))
        );
        assert_eq!(s.get("q"), Some(&VariableValue::Quantity(1)));
    }
}
