//! PDF rendering for draft work orders, finalized work orders, and quotations.
//!
//! Pipeline: Minijinja template (Jinja2-style) → Typst source → PDF via the
//! `typst` crate, mirroring the finance invoices renderer (`electronics style`).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use lariv_rs::plugins::customer::entities::customer::Entity as CustomerEntity;
use lariv_rs::plugins::filesystem::state::FilesystemState;
use lariv_rs::plugins::finance_common::typst::{typst_compile, typst_compile_in, typst_work_dir};
use lariv_rs::plugins::finance_invoices::VnodeImageContext;
use lariv_rs::plugins::finance_taxes::entities::tax;
use minijinja::{Environment, ErrorKind as MiniJinjaErrorKind};
use num2words::{Lang, Num2Words};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;

use super::entities::{
    component, draft_work_order, draft_work_order_machine_line, draft_work_order_material_line,
    quotation, quotation_machine_line, quotation_material_line, work_order, work_order_line,
    work_order_machine_line,
};
use super::preferences::{
    draft_work_order_pdf_template, load_preferences, quotation_pdf_template,
    work_order_pdf_template,
};
use super::tax_assoc;
use crate::machinery_schedule::entities::machine;
use crate::machinery_schedule::logic::format_job_duration;

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("{0}")]
    Message(String),
    #[error("not found")]
    NotFound,
}

impl PdfError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

pub struct PdfResult {
    pub bytes: Vec<u8>,
    pub filename_base: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfCustomer {
    #[serde(rename = "ID")]
    id: i64,
    name: String,
    address: Option<String>,
    #[serde(rename = "GSTIN")]
    gstin: Option<String>,
    #[serde(rename = "PAN")]
    pan: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    website: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfTax {
    #[serde(rename = "ID")]
    id: i64,
    name: String,
    percentage: String,
    #[serde(rename = "TaxType")]
    tax_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfMaterialLine {
    #[serde(rename = "ID")]
    id: i64,
    component_id: i64,
    component: String,
    variables_display: String,
    final_cost: String,
    taxes: String,
    tax_items: Vec<PdfTax>,
    pre_tax: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfMachineLine {
    #[serde(rename = "ID")]
    id: i64,
    machine_id: i64,
    machine: String,
    variables_display: String,
    line_total: String,
    taxes: String,
    tax_items: Vec<PdfTax>,
    pre_tax: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct WorkOrderRoot {
    #[serde(rename = "ID")]
    id: i64,
    order_number: String,
    customer_id: i64,
    created_at: String,
    duration: String,
    customer: PdfCustomer,
    material_lines: Vec<PdfMaterialLine>,
    machine_lines: Vec<PdfMachineLine>,
    total_amount: String,
    #[serde(rename = "company_name")]
    company_name: String,
    #[serde(rename = "company_address")]
    company_address: String,
    #[serde(rename = "company_phone")]
    company_phone: String,
    #[serde(rename = "company_gstin")]
    company_gstin: String,
    #[serde(rename = "place_of_supply")]
    place_of_supply: String,
    #[serde(rename = "company_logo_vnode_id")]
    company_logo_vnode_id: Option<i64>,
    #[serde(rename = "company_signature_vnode_id")]
    company_signature_vnode_id: Option<i64>,
    /// True only for the preferences sample PDF (placeholder logo/signature boxes).
    #[serde(rename = "preview")]
    preview: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct InvoiceRoot {
    #[serde(rename = "ID")]
    id: i64,
    invoice_number: String,
    customer_id: i64,
    date: String,
    customer: PdfCustomer,
    material_lines: Vec<PdfMaterialLine>,
    machine_lines: Vec<PdfMachineLine>,
    grand_total: String,
    #[serde(rename = "company_name")]
    company_name: String,
    #[serde(rename = "company_address")]
    company_address: String,
    #[serde(rename = "company_phone")]
    company_phone: String,
    #[serde(rename = "company_gstin")]
    company_gstin: String,
    #[serde(rename = "place_of_supply")]
    place_of_supply: String,
    #[serde(rename = "company_logo_vnode_id")]
    company_logo_vnode_id: Option<i64>,
    #[serde(rename = "company_signature_vnode_id")]
    company_signature_vnode_id: Option<i64>,
    /// True only for the preferences sample PDF (placeholder logo/signature boxes).
    #[serde(rename = "preview")]
    preview: bool,
}

fn dec_str(d: Decimal) -> String {
    d.normalize().to_string()
}

fn empty_schema() -> serde_json::Value {
    serde_json::json!({})
}

fn machine_schema(machines: &HashMap<i64, machine::Model>, machine_id: i64) -> serde_json::Value {
    machines
        .get(&machine_id)
        .map(|m| m.variables.clone())
        .unwrap_or_else(empty_schema)
}

fn machine_label(
    machines: &HashMap<i64, machine::Model>,
    machine_id: i64,
    fallback: &str,
) -> String {
    machines
        .get(&machine_id)
        .map(|m| m.name.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

/// Serialize PDF context and expose company fields under both snake_case
/// (invoice-style templates) and PascalCase (the rest of this context).
fn pdf_context(root: impl Serialize, what: &str) -> Result<serde_json::Value, PdfError> {
    let mut value = serde_json::to_value(root)
        .map_err(|e| PdfError::msg(format!("serialize {what} PDF context: {e}")))?;
    if let Some(obj) = value.as_object_mut() {
        for (snake, pascal) in [
            ("company_name", "CompanyName"),
            ("company_address", "CompanyAddress"),
            ("company_phone", "CompanyPhone"),
            ("company_gstin", "CompanyGstin"),
            ("place_of_supply", "PlaceOfSupply"),
            ("company_logo_vnode_id", "CompanyLogoVnodeId"),
            ("company_signature_vnode_id", "CompanySignatureVnodeId"),
            ("preview", "Preview"),
        ] {
            if let Some(v) = obj.get(snake).cloned() {
                obj.entry(pascal.to_string()).or_insert(v);
            }
        }
    }
    Ok(value)
}

fn work_order_pdf_context(root: &WorkOrderRoot) -> Result<serde_json::Value, PdfError> {
    pdf_context(root, "work order")
}

/// Serialize quotation PDF context and expose company fields under both snake_case
/// (invoice-style templates) and PascalCase (the rest of this context).
fn quotation_pdf_context(root: &InvoiceRoot) -> Result<serde_json::Value, PdfError> {
    pdf_context(root, "quotation")
}

fn taxes_to_pdf(taxes: &[tax::Model]) -> Vec<PdfTax> {
    taxes
        .iter()
        .map(|t| PdfTax {
            id: t.id,
            name: t.name.clone(),
            percentage: dec_str(t.percentage),
            tax_type: t.tax_type.as_str().to_string(),
        })
        .collect()
}

#[derive(Clone, Debug, Default)]
pub struct CompanyPresentation {
    pub name: String,
    pub address: String,
    pub phone: String,
    pub gstin: String,
    pub place_of_supply: String,
    pub logo_vnode_id: Option<i64>,
    pub signature_vnode_id: Option<i64>,
}

impl CompanyPresentation {
    fn sample() -> Self {
        Self {
            name: "KDS Tagore".into(),
            address: "Industrial Area, \\ \
                 Pune 411019 \\ \
                 Maharashtra \\ \
                 India"
                .into(),
            phone: "+91 20 0000 0000".into(),
            gstin: "27AAAAA0000A1Z5".into(),
            place_of_supply: "Maharashtra".into(),
            logo_vnode_id: None,
            signature_vnode_id: None,
        }
    }

    fn from_work_order_prefs(prefs: &super::entities::WorkOrdersPreferences) -> Self {
        Self {
            name: prefs.company_name.clone().unwrap_or_default(),
            address: prefs.company_address.clone().unwrap_or_default(),
            phone: prefs.company_phone.clone().unwrap_or_default(),
            gstin: prefs.company_gstin.clone().unwrap_or_default(),
            place_of_supply: prefs.place_of_supply.clone().unwrap_or_default(),
            logo_vnode_id: prefs.company_logo_vnode_id.filter(|&id| id > 0),
            signature_vnode_id: prefs.company_signature_vnode_id.filter(|&id| id > 0),
        }
    }

    fn overlay_nonempty(&self, onto: &mut Self) {
        if !self.name.trim().is_empty() {
            onto.name = self.name.clone();
        }
        if !self.address.trim().is_empty() {
            onto.address = self.address.clone();
        }
        if !self.phone.trim().is_empty() {
            onto.phone = self.phone.clone();
        }
        if !self.gstin.trim().is_empty() {
            onto.gstin = self.gstin.clone();
        }
        if !self.place_of_supply.trim().is_empty() {
            onto.place_of_supply = self.place_of_supply.clone();
        }
        if self.logo_vnode_id.is_some() {
            onto.logo_vnode_id = self.logo_vnode_id;
        }
        if self.signature_vnode_id.is_some() {
            onto.signature_vnode_id = self.signature_vnode_id;
        }
    }

    fn apply_to(&self, root: &mut InvoiceRoot) {
        root.company_name = self.name.clone();
        root.company_address = self.address.clone();
        root.company_phone = self.phone.clone();
        root.company_gstin = self.gstin.clone();
        root.place_of_supply = self.place_of_supply.clone();
        root.company_logo_vnode_id = self.logo_vnode_id;
        root.company_signature_vnode_id = self.signature_vnode_id;
    }

    fn apply_to_work_order(&self, root: &mut WorkOrderRoot) {
        root.company_name = self.name.clone();
        root.company_address = self.address.clone();
        root.company_phone = self.phone.clone();
        root.company_gstin = self.gstin.clone();
        root.place_of_supply = self.place_of_supply.clone();
        root.company_logo_vnode_id = self.logo_vnode_id;
        root.company_signature_vnode_id = self.signature_vnode_id;
    }
}

fn sample_gst_tax() -> PdfTax {
    PdfTax {
        id: 1,
        name: "GST".into(),
        percentage: "18".into(),
        tax_type: "levied".into(),
    }
}

pub fn pdf_filename_base(number: Option<&str>, fallback: &str) -> String {
    if let Some(n) = number.map(str::trim).filter(|s| !s.is_empty()) {
        sanitize_pdf_filename_base(n)
    } else {
        fallback.to_string()
    }
}

pub fn sanitize_pdf_filename_base(s: &str) -> String {
    let mut s = s.trim().to_string();
    for ch in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
        s = s.replace(ch, "-");
    }
    if s.is_empty() {
        "document".to_string()
    } else {
        s
    }
}

fn num2words_cardinal(n: i64) -> String {
    Num2Words::new(n)
        .lang(Lang::English)
        .to_words()
        .unwrap_or_default()
}

fn num2words_and(n: i64) -> String {
    let words = num2words_cardinal(n);
    if words.contains(" and ") {
        return words;
    }
    let parts: Vec<&str> = words.split_whitespace().collect();
    if parts.len() >= 3 {
        let mut out = parts[..parts.len() - 2].join(" ");
        out.push_str(" and ");
        out.push_str(parts[parts.len() - 2]);
        out.push(' ');
        out.push_str(parts[parts.len() - 1]);
        return out;
    }
    words
}

fn title_word(w: &str) -> String {
    w.split('-')
        .map(|seg| {
            let mut c = seg.chars();
            match c.next() {
                None => String::new(),
                Some(f) => {
                    f.to_uppercase().collect::<String>() + c.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

fn title_words(s: &str) -> String {
    s.split_whitespace()
        .map(|p| {
            if p.eq_ignore_ascii_case("and") {
                "And".to_string()
            } else {
                title_word(p)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn amount_words(n: i64) -> String {
    format!("{} Rupees", title_words(&num2words_and(n)))
}

fn amount_words_from_decimal(d: Decimal) -> String {
    let rounded = d.round().to_string().parse::<i64>().unwrap_or(0);
    amount_words(rounded)
}

async fn load_customer(db: &DatabaseConnection, id: i64) -> Result<PdfCustomer, PdfError> {
    let c = CustomerEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    Ok(match c {
        Some(c) => {
            let address = c.formatted_address_for_typst();
            PdfCustomer {
                id: c.id,
                name: c.name,
                address,
                gstin: c.gstin,
                pan: c.pan,
                phone: c.phone,
                email: c.email,
                website: c.website,
            }
        }
        None => PdfCustomer {
            id,
            name: format!("Customer #{id}"),
            address: None,
            gstin: None,
            pan: None,
            phone: None,
            email: None,
            website: None,
        },
    })
}

fn num2words_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(num2words_cardinal(n))
}

fn num2words_and_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(num2words_and(n))
}

fn num2words_rupees_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(amount_words(n))
}

fn source_snippet(src: &str, line_no: usize, radius: usize) -> String {
    let lines: Vec<&str> = src.lines().collect();
    if line_no == 0 || lines.is_empty() {
        return String::new();
    }
    let idx = line_no.saturating_sub(1);
    let start = idx.saturating_sub(radius);
    let end = (idx + radius + 1).min(lines.len());
    let mut out = String::new();
    for (i, line) in lines[start..end].iter().enumerate() {
        let n = start + i + 1;
        let mark = if n == line_no { '>' } else { ' ' };
        out.push_str(&format!("\n{mark} {n:>4} | {line}"));
    }
    out
}

fn format_minijinja_error(err: &minijinja::Error, tmpl_src: &str) -> String {
    let mut out = format!("Template render failed: {err}");
    if let Some(line) = err.line() {
        let snippet = source_snippet(tmpl_src, line, 2);
        if !snippet.is_empty() {
            out.push_str("\n\nNear template line ");
            out.push_str(&line.to_string());
            out.push(':');
            out.push_str(&snippet);
        }
    }
    match (err.kind(), err.detail()) {
        (MiniJinjaErrorKind::MissingArgument, Some("value")) => {
            out.push_str(
                "\n\nHint: a filter/function expected its input `value`. \
                 Company name, address, GSTIN, phone, logo, and signature may be left blank. \
                 If this is `|replace`, write `{{ text|replace(\"old\", \"new\") }}`.",
            );
        }
        (MiniJinjaErrorKind::MissingArgument, Some(name)) => {
            out.push_str("\n\nThe missing argument is `");
            out.push_str(name);
            out.push_str("`.");
        }
        (MiniJinjaErrorKind::UndefinedError, Some(name)) => {
            out.push_str("\n\nUndefined template variable `");
            out.push_str(name);
            out.push_str("`. Fill it from quotation data or KDS Quotations → Preferences.");
        }
        _ => {}
    }
    out
}

fn format_typst_error(err: &str, typst_src: &str) -> String {
    let mut out = format!("Typst compile failed: {err}");
    if err.contains("missing argument: value") {
        out.push_str(
            "\n\nTypst called `str()` or `dictionary.insert()` without a value. \
             That happens when a template placeholder renders empty, e.g. \
             `float(str({{ line.PreTax }}))` becomes `float(str())`. \
             Company name, address, GSTIN, phone, logo, and signature can stay blank; \
             check numeric placeholders (amounts, tax percentages) in the Typst template.",
        );
    } else if err.contains("missing argument: source") {
        out.push_str(
            "\n\n`#image(...)` had no file path. Select a Quotation logo or signature \
             under KDS Quotations → Preferences, or remove the `#image` / `vnodeImage(...)` call.",
        );
    }
    let mut hits = Vec::new();
    for (i, line) in typst_src.lines().enumerate() {
        let t = line.trim();
        if t.contains("str()")
            || t.contains("float()")
            || t.contains("#image(\"\")")
            || t.contains("#image('')")
        {
            hits.push(format!("  line {}: {t}", i + 1));
        }
    }
    if !hits.is_empty() {
        out.push_str("\n\nSuspicious generated Typst:");
        for hit in hits.into_iter().take(12) {
            out.push('\n');
            out.push_str(&hit);
        }
    }
    out
}

fn vnode_id_i64(v: &minijinja::Value) -> i64 {
    if let Some(i) = v.as_i64() {
        return i;
    }
    if let Some(s) = v.as_str() {
        return s.parse().ok().filter(|&id| id > 0).unwrap_or(0);
    }
    0
}

fn render_template(
    tmpl_src: &str,
    ctx: serde_json::Value,
    grand_words: &str,
    vnode_ctx: Option<&VnodeImageContext>,
) -> Result<String, PdfError> {
    let grand_words = grand_words.to_string();
    let mut env = Environment::new();
    env.add_function("num2words", num2words_fn);
    env.add_function("num2wordsAnd", num2words_and_fn);
    env.add_function("num2wordsRupees", num2words_rupees_fn);
    let gw1 = grand_words.clone();
    let gw2 = grand_words;
    env.add_function(
        "totalAmountWords",
        move || -> Result<String, minijinja::Error> { Ok(gw1.clone()) },
    );
    env.add_function(
        "grandTotalWords",
        move || -> Result<String, minijinja::Error> { Ok(gw2.clone()) },
    );
    if let Some(ctx) = vnode_ctx {
        let ctx = ctx.clone();
        env.add_function(
            "vnodeImage",
            move |vnode_id: minijinja::Value| -> Result<String, minijinja::Error> {
                let vnode_id = vnode_id_i64(&vnode_id);
                if vnode_id <= 0 {
                    return Err(minijinja::Error::new(
                        MiniJinjaErrorKind::InvalidOperation,
                        "vnodeImage: no file selected. Choose a Quotation logo or signature under KDS Quotations → Preferences.",
                    ));
                }
                ctx.resolve_sync(vnode_id).map_err(|e| {
                    minijinja::Error::new(
                        MiniJinjaErrorKind::InvalidOperation,
                        format!(
                            "{e}. Check Quotation logo / signature under KDS Quotations → Preferences."
                        ),
                    )
                })
            },
        );
    }
    env.add_template("quotation.typ.tmpl", tmpl_src)
        .map_err(|e| PdfError::msg(format_minijinja_error(&e, tmpl_src)))?;
    let tmpl = env
        .get_template("quotation.typ.tmpl")
        .map_err(|e| PdfError::msg(format_minijinja_error(&e, tmpl_src)))?;
    tmpl.render(ctx)
        .map_err(|e| PdfError::msg(format_minijinja_error(&e, tmpl_src)))
}

async fn compile_pdf(
    tmpl_src: &str,
    ctx: serde_json::Value,
    grand_words: String,
    fs: Option<&FilesystemState>,
) -> Result<Vec<u8>, PdfError> {
    if let Some(fs) = fs {
        let work_dir = typst_work_dir();
        let vnode_ctx =
            VnodeImageContext::new(fs.db.clone(), Arc::clone(&fs.store), work_dir.clone());
        let typst_src = render_template(tmpl_src, ctx, &grand_words, Some(&vnode_ctx))?;
        let result = typst_compile_in(&work_dir, &typst_src)
            .await
            .map_err(|e| PdfError::msg(format_typst_error(&e, &typst_src)));
        if let Err(e) = std::fs::remove_dir_all(&work_dir) {
            tracing::warn!(error = %e, path = %work_dir.display(), "failed to remove typst work dir");
        }
        result
    } else {
        let typst_src = render_template(tmpl_src, ctx, &grand_words, None)?;
        typst_compile(&typst_src)
            .await
            .map_err(|e| PdfError::msg(format_typst_error(&e, &typst_src)))
    }
}

/// Render a draft work order to PDF bytes using the configured template.
pub async fn render_work_order_pdf(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    id: i64,
    tz: &str,
) -> Result<PdfResult, PdfError> {
    let order = draft_work_order::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?
        .ok_or(PdfError::NotFound)?;

    let lines = draft_work_order_material_line::Entity::find()
        .filter(draft_work_order_material_line::Column::DraftWorkOrderId.eq(order.id))
        .order_by_asc(draft_work_order_material_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;

    let machine_lines = draft_work_order_machine_line::Entity::find()
        .filter(draft_work_order_machine_line::Column::DraftWorkOrderId.eq(order.id))
        .order_by_asc(draft_work_order_machine_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;

    let comp_ids: Vec<i64> = lines.iter().map(|l| l.component_id).collect();
    let comps = component::Entity::find()
        .filter(component::Column::Id.is_in(comp_ids))
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let comp_map: HashMap<i64, component::Model> = comps.into_iter().map(|c| (c.id, c)).collect();

    let machine_ids: Vec<i64> = machine_lines.iter().map(|l| l.machine_id).collect();
    let machines = machine::Entity::find()
        .filter(machine::Column::Id.is_in(machine_ids))
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let machine_map: HashMap<i64, machine::Model> =
        machines.into_iter().map(|m| (m.id, m)).collect();

    let mat_ids: Vec<i64> = lines.iter().map(|l| l.id).collect();
    let mach_ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let mat_tax_ids = tax_assoc::load_draft_material_line_tax_ids_map(db, &mat_ids)
        .await
        .unwrap_or_default();
    let mach_tax_ids = tax_assoc::load_draft_machine_line_tax_ids_map(db, &mach_ids)
        .await
        .unwrap_or_default();
    let material_taxes = tax_assoc::resolve_taxes_by_line_id(db, &mat_tax_ids).await;
    let machine_taxes = tax_assoc::resolve_taxes_by_line_id(db, &mach_tax_ids).await;

    let total_amount = lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(&material_taxes, l.id)))
        .sum::<Decimal>()
        + machine_lines
            .iter()
            .map(|l| l.taxed_total(tax_assoc::taxes_for_line(&machine_taxes, l.id)))
            .sum::<Decimal>();

    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = draft_work_order_pdf_template(&prefs).to_string();

    let customer = load_customer(db, order.customer_id).await?;
    let material_lines = lines
        .iter()
        .map(|l| {
            let taxes = tax_assoc::taxes_for_line(&material_taxes, l.id);
            PdfMaterialLine {
                id: l.id,
                component_id: l.component_id,
                component: comp_map
                    .get(&l.component_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Component #{}", l.component_id)),
                variables_display: {
                    let schema = comp_map
                        .get(&l.component_id)
                        .map(|c| c.variables.clone())
                        .unwrap_or_else(|| serde_json::json!({}));
                    l.format_variables_display_with_schema(&schema)
                },
                pre_tax: dec_str(l.final_cost),
                taxes: tax_assoc::tax_labels_display(taxes),
                tax_items: taxes_to_pdf(taxes),
                final_cost: dec_str(l.taxed_total(taxes)),
            }
        })
        .collect();

    let pdf_machine_lines = machine_lines
        .iter()
        .map(|l| {
            let taxes = tax_assoc::taxes_for_line(&machine_taxes, l.id);
            PdfMachineLine {
                id: l.id,
                machine_id: l.machine_id,
                machine: machine_label(
                    &machine_map,
                    l.machine_id,
                    &format!("Machine #{}", l.machine_id),
                ),
                variables_display: l.format_variables_display_with_schema(&machine_schema(
                    &machine_map,
                    l.machine_id,
                )),
                pre_tax: dec_str(l.line_total()),
                taxes: tax_assoc::tax_labels_display(taxes),
                tax_items: taxes_to_pdf(taxes),
                line_total: dec_str(l.taxed_total(taxes)),
            }
        })
        .collect();

    let created_at =
        lariv_rs::datetime::format_date_in_tz(order.created_at.unwrap_or(Utc::now()), tz);
    let company = CompanyPresentation::from_work_order_prefs(&prefs);
    let root = WorkOrderRoot {
        id: order.id,
        order_number: order.order_number.clone(),
        customer_id: order.customer_id,
        created_at,
        duration: format_job_duration(order.duration),
        customer,
        material_lines,
        machine_lines: pdf_machine_lines,
        total_amount: dec_str(total_amount),
        company_name: company.name,
        company_address: company.address,
        company_phone: company.phone,
        company_gstin: company.gstin,
        place_of_supply: company.place_of_supply,
        company_logo_vnode_id: company.logo_vnode_id,
        company_signature_vnode_id: company.signature_vnode_id,
        preview: false,
    };
    let ctx = work_order_pdf_context(&root)?;
    let bytes = compile_pdf(&tmpl_src, ctx, amount_words_from_decimal(total_amount), fs).await?;
    let base = pdf_filename_base(
        Some(&order.order_number),
        &format!("draft-work-order-{}", order.id),
    );
    Ok(PdfResult {
        bytes,
        filename_base: base,
    })
}

/// Render a finalized (issued) work order to PDF bytes using the configured template.
pub async fn render_issued_work_order_pdf(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    id: i64,
    tz: &str,
) -> Result<PdfResult, PdfError> {
    let order = work_order::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?
        .ok_or(PdfError::NotFound)?;

    let lines = work_order_line::Entity::find()
        .filter(work_order_line::Column::WorkOrderId.eq(order.id))
        .order_by_asc(work_order_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;

    let machine_lines = work_order_machine_line::Entity::find()
        .filter(work_order_machine_line::Column::WorkOrderId.eq(order.id))
        .order_by_asc(work_order_machine_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;

    let comp_ids: Vec<i64> = lines.iter().map(|l| l.component_id).collect();
    let comps = component::Entity::find()
        .filter(component::Column::Id.is_in(comp_ids))
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let comp_map: HashMap<i64, component::Model> = comps.into_iter().map(|c| (c.id, c)).collect();

    let machine_ids: Vec<i64> = machine_lines.iter().filter_map(|l| l.machine_id).collect();
    let machines = machine::Entity::find()
        .filter(machine::Column::Id.is_in(machine_ids))
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let machine_map: HashMap<i64, machine::Model> =
        machines.into_iter().map(|m| (m.id, m)).collect();

    let mat_ids: Vec<i64> = lines.iter().map(|l| l.id).collect();
    let mach_ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let mat_tax_ids = tax_assoc::load_work_order_material_line_tax_ids_map(db, &mat_ids)
        .await
        .unwrap_or_default();
    let mach_tax_ids = tax_assoc::load_work_order_machine_line_tax_ids_map(db, &mach_ids)
        .await
        .unwrap_or_default();
    let material_taxes = tax_assoc::resolve_taxes_by_line_id(db, &mat_tax_ids).await;
    let machine_taxes = tax_assoc::resolve_taxes_by_line_id(db, &mach_tax_ids).await;

    let total_amount = lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(&material_taxes, l.id)))
        .sum::<Decimal>()
        + machine_lines
            .iter()
            .map(|l| l.taxed_total(tax_assoc::taxes_for_line(&machine_taxes, l.id)))
            .sum::<Decimal>();

    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = work_order_pdf_template(&prefs).to_string();

    let customer = load_customer(db, order.customer_id).await?;
    let material_lines = lines
        .iter()
        .map(|l| {
            let taxes = tax_assoc::taxes_for_line(&material_taxes, l.id);
            PdfMaterialLine {
                id: l.id,
                component_id: l.component_id,
                component: comp_map
                    .get(&l.component_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Component #{}", l.component_id)),
                variables_display: {
                    let schema = comp_map
                        .get(&l.component_id)
                        .map(|c| c.variables.clone())
                        .unwrap_or_else(|| serde_json::json!({}));
                    l.format_variables_display_with_schema(&schema)
                },
                pre_tax: dec_str(l.final_cost),
                taxes: tax_assoc::tax_labels_display(taxes),
                tax_items: taxes_to_pdf(taxes),
                final_cost: dec_str(l.taxed_total(taxes)),
            }
        })
        .collect();

    let pdf_machine_lines = machine_lines
        .iter()
        .map(|l| {
            let taxes = tax_assoc::taxes_for_line(&machine_taxes, l.id);
            PdfMachineLine {
                id: l.id,
                machine_id: l.machine_id.unwrap_or(0),
                machine: l
                    .machine_id
                    .and_then(|id| machine_map.get(&id).map(|m| m.name.clone()))
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| l.name.clone()),
                variables_display: {
                    let schema = l
                        .machine_id
                        .map(|id| machine_schema(&machine_map, id))
                        .unwrap_or_else(empty_schema);
                    l.format_variables_display_with_schema(&schema)
                },
                pre_tax: dec_str(l.line_total()),
                taxes: tax_assoc::tax_labels_display(taxes),
                tax_items: taxes_to_pdf(taxes),
                line_total: dec_str(l.taxed_total(taxes)),
            }
        })
        .collect();

    let created_at =
        lariv_rs::datetime::format_date_in_tz(order.created_at.unwrap_or(Utc::now()), tz);
    let company = CompanyPresentation::from_work_order_prefs(&prefs);
    let root = WorkOrderRoot {
        id: order.id,
        order_number: order.order_number.clone(),
        customer_id: order.customer_id,
        created_at,
        duration: format_job_duration(order.duration),
        customer,
        material_lines,
        machine_lines: pdf_machine_lines,
        total_amount: dec_str(total_amount),
        company_name: company.name,
        company_address: company.address,
        company_phone: company.phone,
        company_gstin: company.gstin,
        place_of_supply: company.place_of_supply,
        company_logo_vnode_id: company.logo_vnode_id,
        company_signature_vnode_id: company.signature_vnode_id,
        preview: false,
    };
    let ctx = work_order_pdf_context(&root)?;
    let bytes = compile_pdf(&tmpl_src, ctx, amount_words_from_decimal(total_amount), fs).await?;
    let base = pdf_filename_base(
        Some(&order.order_number),
        &format!("work-order-{}", order.id),
    );
    Ok(PdfResult {
        bytes,
        filename_base: base,
    })
}

/// Render a quotation to PDF bytes using the configured template.
pub async fn render_quotation_pdf(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    id: i64,
    _tz: &str,
) -> Result<PdfResult, PdfError> {
    let inv = quotation::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?
        .ok_or(PdfError::NotFound)?;

    let material_lines = quotation_material_line::Entity::find()
        .filter(quotation_material_line::Column::InvoiceId.eq(inv.id))
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;

    let machine_lines = quotation_machine_line::Entity::find()
        .filter(quotation_machine_line::Column::InvoiceId.eq(inv.id))
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;

    let comp_ids: Vec<i64> = material_lines.iter().map(|l| l.component_id).collect();
    let comps = component::Entity::find()
        .filter(component::Column::Id.is_in(comp_ids))
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let comp_map: HashMap<i64, component::Model> = comps.into_iter().map(|c| (c.id, c)).collect();

    let machine_ids: Vec<i64> = machine_lines.iter().filter_map(|l| l.machine_id).collect();
    let machines = machine::Entity::find()
        .filter(machine::Column::Id.is_in(machine_ids))
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let machine_map: HashMap<i64, machine::Model> =
        machines.into_iter().map(|m| (m.id, m)).collect();

    let mat_ids: Vec<i64> = material_lines.iter().map(|l| l.id).collect();
    let mach_ids: Vec<i64> = machine_lines.iter().map(|l| l.id).collect();
    let mat_tax_ids = tax_assoc::load_quotation_material_line_tax_ids_map(db, &mat_ids)
        .await
        .unwrap_or_default();
    let mach_tax_ids = tax_assoc::load_quotation_machine_line_tax_ids_map(db, &mach_ids)
        .await
        .unwrap_or_default();
    let material_taxes = tax_assoc::resolve_taxes_by_line_id(db, &mat_tax_ids).await;
    let machine_taxes = tax_assoc::resolve_taxes_by_line_id(db, &mach_tax_ids).await;

    let grand_total = material_lines
        .iter()
        .map(|l| l.taxed_total(tax_assoc::taxes_for_line(&material_taxes, l.id)))
        .sum::<Decimal>()
        + machine_lines
            .iter()
            .map(|l| l.taxed_total(tax_assoc::taxes_for_line(&machine_taxes, l.id)))
            .sum::<Decimal>();

    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = quotation_pdf_template(&prefs).to_string();

    let customer = load_customer(db, inv.customer_id).await?;
    let pdf_material_lines = material_lines
        .iter()
        .map(|l| {
            let taxes = tax_assoc::taxes_for_line(&material_taxes, l.id);
            PdfMaterialLine {
                id: l.id,
                component_id: l.component_id,
                component: comp_map
                    .get(&l.component_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Component #{}", l.component_id)),
                variables_display: {
                    let schema = comp_map
                        .get(&l.component_id)
                        .map(|c| c.variables.clone())
                        .unwrap_or_else(|| serde_json::json!({}));
                    l.format_variables_display_with_schema(&schema)
                },
                pre_tax: dec_str(l.final_cost),
                taxes: tax_assoc::tax_labels_display(taxes),
                tax_items: taxes_to_pdf(taxes),
                final_cost: dec_str(l.taxed_total(taxes)),
            }
        })
        .collect();

    let pdf_machine_lines = machine_lines
        .iter()
        .map(|l| {
            let taxes = tax_assoc::taxes_for_line(&machine_taxes, l.id);
            PdfMachineLine {
                id: l.id,
                machine_id: l.machine_id.unwrap_or(0),
                machine: l
                    .machine_id
                    .and_then(|id| machine_map.get(&id).map(|m| m.name.clone()))
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| l.name.clone()),
                variables_display: {
                    let schema = l
                        .machine_id
                        .map(|id| machine_schema(&machine_map, id))
                        .unwrap_or_else(empty_schema);
                    l.format_variables_display_with_schema(&schema)
                },
                pre_tax: dec_str(l.line_total()),
                taxes: tax_assoc::tax_labels_display(taxes),
                tax_items: taxes_to_pdf(taxes),
                line_total: dec_str(l.taxed_total(taxes)),
            }
        })
        .collect();

    let date = lariv_rs::datetime::format_date(inv.date);
    let company = CompanyPresentation::from_work_order_prefs(&prefs);
    let root = InvoiceRoot {
        id: inv.id,
        invoice_number: inv.invoice_number.clone(),
        customer_id: inv.customer_id,
        date,
        customer,
        material_lines: pdf_material_lines,
        machine_lines: pdf_machine_lines,
        grand_total: dec_str(grand_total),
        company_name: company.name,
        company_address: company.address,
        company_phone: company.phone,
        company_gstin: company.gstin,
        place_of_supply: company.place_of_supply,
        company_logo_vnode_id: company.logo_vnode_id,
        company_signature_vnode_id: company.signature_vnode_id,
        preview: false,
    };
    let ctx = quotation_pdf_context(&root)?;
    let bytes = compile_pdf(&tmpl_src, ctx, amount_words_from_decimal(grand_total), fs).await?;
    let base = pdf_filename_base(Some(&inv.invoice_number), &format!("quotation-{}", inv.id));
    Ok(PdfResult {
        bytes,
        filename_base: base,
    })
}

fn sample_customer() -> PdfCustomer {
    PdfCustomer {
        id: 1,
        name: "Acme Industries Pvt. Ltd.".into(),
        address: Some(
            "123 Example Street, \\ \
             Business Park, \\ \
             Mumbai 400001 \\ \
             Maharashtra \\ \
             India"
                .into(),
        ),
        gstin: Some("27AAAAA0000A1Z5".into()),
        pan: Some("AAAAA0000A".into()),
        phone: Some("+91 98765 43210".into()),
        email: Some("billing@example.com".into()),
        website: Some("www.acme.example".into()),
    }
}

fn sample_work_order_root(dt: DateTime<Utc>, tz: &str) -> WorkOrderRoot {
    let company = CompanyPresentation::sample();
    WorkOrderRoot {
        id: 1,
        order_number: "WO/2026/0001".into(),
        customer_id: 1,
        created_at: lariv_rs::datetime::format_date_in_tz(dt, tz),
        duration: "2h 30m".into(),
        customer: sample_customer(),
        material_lines: vec![PdfMaterialLine {
            id: 1,
            component_id: 1,
            component: "MS Flat Bar 2.5x3.5mm — 1000mm".into(),
            variables_display: "length: 1000 mm, qty: 2".into(),
            pre_tax: "297.5".into(),
            taxes: "GST 18%".into(),
            tax_items: vec![sample_gst_tax()],
            final_cost: "351.05".into(),
        }],
        machine_lines: vec![PdfMachineLine {
            id: 1,
            machine_id: 1,
            machine: "CNC Lathe".into(),
            variables_display: "duration: 2h 30m".into(),
            pre_tax: "2375".into(),
            taxes: "GST 18%".into(),
            tax_items: vec![sample_gst_tax()],
            line_total: "2802.5".into(),
        }],
        total_amount: "3153.55".into(),
        company_name: company.name,
        company_address: company.address,
        company_phone: company.phone,
        company_gstin: company.gstin,
        place_of_supply: company.place_of_supply,
        company_logo_vnode_id: company.logo_vnode_id,
        company_signature_vnode_id: company.signature_vnode_id,
        preview: false,
    }
}

fn sample_invoice_root() -> InvoiceRoot {
    let company = CompanyPresentation::sample();
    InvoiceRoot {
        id: 1,
        invoice_number: "PF/2026/0042".into(),
        customer_id: 1,
        date: lariv_rs::datetime::format_date(
            NaiveDate::from_ymd_opt(2026, 2, 8).expect("valid date"),
        ),
        customer: sample_customer(),
        material_lines: vec![PdfMaterialLine {
            id: 1,
            component_id: 1,
            component: "MS Flat Bar 2.5x3.5mm — 1000mm".into(),
            variables_display: "length: 1000 mm, qty: 50".into(),
            pre_tax: "4250".into(),
            taxes: "GST 18%".into(),
            tax_items: vec![sample_gst_tax()],
            final_cost: "5015".into(),
        }],
        machine_lines: vec![PdfMachineLine {
            id: 1,
            machine_id: 1,
            machine: "CNC Lathe".into(),
            variables_display: "duration: 3h".into(),
            pre_tax: "2850".into(),
            taxes: "GST 18%".into(),
            tax_items: vec![sample_gst_tax()],
            line_total: "3363".into(),
        }],
        grand_total: "8378".into(),
        company_name: company.name,
        company_address: company.address,
        company_phone: company.phone,
        company_gstin: company.gstin,
        place_of_supply: company.place_of_supply,
        company_logo_vnode_id: company.logo_vnode_id,
        company_signature_vnode_id: company.signature_vnode_id,
        preview: false,
    }
}

/// Render a sample draft work order PDF using an optional template override.
pub async fn render_work_order_pdf_preview(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    template_src: Option<&str>,
    presentation: Option<CompanyPresentation>,
    tz: &str,
) -> Result<PdfResult, PdfError> {
    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = match template_src.map(str::trim).filter(|s| !s.is_empty()) {
        None => draft_work_order_pdf_template(&prefs),
        Some(s) => super::pdf_templates::resolved_draft_work_order_pdf_template(Some(s)),
    };
    let mut root = sample_work_order_root(Utc::now(), tz);
    root.preview = true;
    let mut preview_company = CompanyPresentation::sample();
    CompanyPresentation::from_work_order_prefs(&prefs).overlay_nonempty(&mut preview_company);
    if let Some(form_company) = presentation {
        form_company.overlay_nonempty(&mut preview_company);
    }
    preview_company.apply_to_work_order(&mut root);
    let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
    let ctx = work_order_pdf_context(&root)?;
    let bytes = compile_pdf(tmpl_src, ctx, amount_words_from_decimal(grand), fs).await?;
    Ok(PdfResult {
        bytes,
        filename_base: "draft-work-order-preview".to_string(),
    })
}

/// Render a sample finalized work order PDF using an optional template override.
pub async fn render_issued_work_order_pdf_preview(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    template_src: Option<&str>,
    presentation: Option<CompanyPresentation>,
    tz: &str,
) -> Result<PdfResult, PdfError> {
    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = match template_src.map(str::trim).filter(|s| !s.is_empty()) {
        None => work_order_pdf_template(&prefs),
        Some(s) => super::pdf_templates::resolved_work_order_pdf_template(Some(s)),
    };
    let mut root = sample_work_order_root(Utc::now(), tz);
    root.preview = true;
    let mut preview_company = CompanyPresentation::sample();
    CompanyPresentation::from_work_order_prefs(&prefs).overlay_nonempty(&mut preview_company);
    if let Some(form_company) = presentation {
        form_company.overlay_nonempty(&mut preview_company);
    }
    preview_company.apply_to_work_order(&mut root);
    let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
    let ctx = work_order_pdf_context(&root)?;
    let bytes = compile_pdf(tmpl_src, ctx, amount_words_from_decimal(grand), fs).await?;
    Ok(PdfResult {
        bytes,
        filename_base: "work-order-preview".to_string(),
    })
}

/// Render a sample quotation PDF using an optional template override.
pub async fn render_quotation_pdf_preview(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    template_src: Option<&str>,
    presentation: Option<CompanyPresentation>,
) -> Result<PdfResult, PdfError> {
    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = match template_src.map(str::trim).filter(|s| !s.is_empty()) {
        None => quotation_pdf_template(&prefs),
        Some(s) => super::pdf_templates::resolved_quotation_pdf_template(Some(s)),
    };
    let mut root = sample_invoice_root();
    root.preview = true;
    let mut preview_company = CompanyPresentation::sample();
    CompanyPresentation::from_work_order_prefs(&prefs).overlay_nonempty(&mut preview_company);
    if let Some(form_company) = presentation {
        form_company.overlay_nonempty(&mut preview_company);
    }
    preview_company.apply_to(&mut root);
    let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
    let ctx = quotation_pdf_context(&root)?;
    let bytes = compile_pdf(tmpl_src, ctx, amount_words_from_decimal(grand), fs).await?;
    Ok(PdfResult {
        bytes,
        filename_base: "quotation-preview".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_orders::pdf_templates::{
        DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE, DEFAULT_QUOTATION_PDF_TEMPLATE,
        DEFAULT_WORK_ORDER_PDF_TEMPLATE, is_stock_draft_work_order_template,
        is_stock_quotation_template, is_stock_work_order_template,
        resolved_draft_work_order_pdf_template, resolved_quotation_pdf_template,
        resolved_work_order_pdf_template,
    };

    #[test]
    fn amount_words_match() {
        assert_eq!(
            amount_words_from_decimal(Decimal::from(2672)),
            "Two Thousand Six Hundred And Seventy-Two Rupees"
        );
    }

    #[test]
    fn filename_base_sanitizes() {
        assert_eq!(sanitize_pdf_filename_base("WO/2026/0001"), "WO-2026-0001");
        assert_eq!(sanitize_pdf_filename_base("  "), "document");
    }

    #[test]
    fn default_work_order_template_renders() {
        let root = sample_work_order_root(Utc::now(), "Asia/Kolkata");
        let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
        let ctx = work_order_pdf_context(&root).expect("serialize");
        let out = render_template(
            DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE,
            ctx,
            &amount_words_from_decimal(grand),
            None,
        )
        .expect("render");
        let out_lower = out.to_lowercase();
        assert!(
            out_lower.contains("acme industries"),
            "customer name missing:\n{out}"
        );
        assert!(out.contains("DRAFT WORK ORDER"));
        assert!(out.contains("WO/2026/0001"));
        assert!(out.contains("Buyer (Bill to)"));
        assert!(out.contains("KDS Tagore"));
        assert!(out.contains("CNC Lathe"));
        assert!(out.contains("Machine Operations"));
        assert!(!out.contains("[Logo]"));
        assert!(!out.contains("Customer ID"));
    }

    #[test]
    fn default_issued_work_order_template_renders() {
        let root = sample_work_order_root(Utc::now(), "Asia/Kolkata");
        let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
        let ctx = work_order_pdf_context(&root).expect("serialize");
        let out = render_template(
            DEFAULT_WORK_ORDER_PDF_TEMPLATE,
            ctx,
            &amount_words_from_decimal(grand),
            None,
        )
        .expect("render");
        let out_lower = out.to_lowercase();
        assert!(
            out_lower.contains("acme industries"),
            "customer name missing:\n{out}"
        );
        assert!(out.contains("*WORK ORDER*"));
        assert!(!out.contains("DRAFT WORK ORDER"));
        assert!(out.contains("WO/2026/0001"));
        assert!(out.contains("Buyer (Bill to)"));
        assert!(out.contains("KDS Tagore"));
        assert!(out.contains("Computer Generated Work Order"));
        assert!(!out.contains("[Logo]"));
        assert!(!out.contains("Customer ID"));
    }

    #[test]
    fn default_invoice_template_renders() {
        let root = sample_invoice_root();
        let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
        let ctx = quotation_pdf_context(&root).expect("serialize");
        let out = render_template(
            DEFAULT_QUOTATION_PDF_TEMPLATE,
            ctx,
            &amount_words_from_decimal(grand),
            None,
        )
        .expect("render");
        assert!(out.contains("QUOTATION"));
        assert!(out.contains("PF/2026/0042"));
        assert!(out.contains("ORIGINAL FOR RECIPIENT"));
        assert!(out.contains("KDS Tagore"));
        assert!(out.contains("CNC Lathe"));
        assert!(out.contains("Buyer (Bill to)"));
        assert!(out.contains("www.acme.example"));
        assert!(out.contains("27AAAAA0000A1Z5"));
        assert!(!out.contains("[Logo]"));
    }

    #[test]
    fn preview_uses_sample_placeholders_for_blank_company_fields() {
        let mut company = CompanyPresentation::sample();
        CompanyPresentation {
            name: "Acme Steel Works".into(),
            ..Default::default()
        }
        .overlay_nonempty(&mut company);
        assert_eq!(company.name, "Acme Steel Works");
        assert!(company.address.contains("Pune 411019"));
        assert_eq!(company.phone, "+91 20 0000 0000");
        assert_eq!(company.gstin, "27AAAAA0000A1Z5");
        assert_eq!(company.place_of_supply, "Maharashtra");
    }

    #[test]
    fn preview_template_shows_logo_and_signature_placeholders() {
        let mut root = sample_invoice_root();
        root.preview = true;
        let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
        let ctx = quotation_pdf_context(&root).expect("serialize");
        let out = render_template(
            DEFAULT_QUOTATION_PDF_TEMPLATE,
            ctx,
            &amount_words_from_decimal(grand),
            None,
        )
        .expect("render");
        assert!(out.contains("[Logo]"), "{out}");
        assert!(out.contains("[Signature]"), "{out}");
        assert!(out.contains("KDS Tagore"));
        assert!(out.contains("Industrial Area"));
    }

    #[test]
    fn quotation_context_includes_snake_and_pascal_company_keys() {
        let mut root = sample_invoice_root();
        root.company_name = "Acme Steel Works".into();
        root.company_phone = "+91 20 1111 2222".into();
        let ctx = quotation_pdf_context(&root).expect("serialize");
        assert_eq!(ctx["company_name"], "Acme Steel Works");
        assert_eq!(ctx["CompanyName"], "Acme Steel Works");
        assert_eq!(ctx["company_phone"], "+91 20 1111 2222");
        assert_eq!(ctx["CompanyPhone"], "+91 20 1111 2222");
    }

    #[test]
    fn default_quotation_template_renders_preference_company_fields() {
        let mut root = sample_invoice_root();
        root.company_name = "Acme Steel Works".into();
        root.company_address = "Plot 12, MIDC".into();
        root.company_phone = "+91 20 1111 2222".into();
        root.company_gstin = "27BBBBB1111B1Z5".into();
        root.place_of_supply = "Maharashtra".into();
        let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
        let ctx = quotation_pdf_context(&root).expect("serialize");
        let out = render_template(
            DEFAULT_QUOTATION_PDF_TEMPLATE,
            ctx,
            &amount_words_from_decimal(grand),
            None,
        )
        .expect("render");
        assert!(out.contains("Acme Steel Works"), "{out}");
        assert!(out.contains("Plot 12, MIDC"), "{out}");
        assert!(out.contains("+91 20 1111 2222"), "{out}");
        assert!(out.contains("27BBBBB1111B1Z5"), "{out}");
        assert!(out.contains("Maharashtra"), "{out}");
        assert!(!out.contains("KDS Tagore"), "{out}");
    }

    #[test]
    fn legacy_stock_quotation_template_resolves_to_current_default() {
        let legacy = include_str!("templates/legacy_quotation_pdf.typ.tmpl");
        assert!(is_stock_quotation_template(Some(legacy)));
        let resolved = resolved_quotation_pdf_template(Some(legacy));
        assert!(resolved.contains("{{ company_name }}"));
        assert_eq!(resolved, DEFAULT_QUOTATION_PDF_TEMPLATE);
    }

    #[test]
    fn legacy_stock_work_order_templates_resolve_to_current_default() {
        let draft_legacy = include_str!("templates/legacy_draft_work_order_pdf.typ.tmpl");
        assert!(is_stock_draft_work_order_template(Some(draft_legacy)));
        let draft = resolved_draft_work_order_pdf_template(Some(draft_legacy));
        assert!(draft.contains("{{ company_name }}"));
        assert_eq!(draft, DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE);

        let issued_legacy = include_str!("templates/legacy_work_order_pdf.typ.tmpl");
        assert!(is_stock_work_order_template(Some(issued_legacy)));
        let issued = resolved_work_order_pdf_template(Some(issued_legacy));
        assert!(issued.contains("{{ company_name }}"));
        assert_eq!(issued, DEFAULT_WORK_ORDER_PDF_TEMPLATE);
    }

    #[test]
    fn template_missing_company_fields_is_treated_as_stock() {
        let old = "#set page(paper: \"a4\")\n*QUOTATION*\n{{ Customer.Name }}\n";
        assert!(is_stock_quotation_template(Some(old)));
        assert_eq!(
            resolved_quotation_pdf_template(Some(old)),
            DEFAULT_QUOTATION_PDF_TEMPLATE
        );
    }

    #[test]
    fn preview_work_order_template_shows_logo_and_signature_placeholders() {
        let mut root = sample_work_order_root(Utc::now(), "Asia/Kolkata");
        root.preview = true;
        let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
        let ctx = work_order_pdf_context(&root).expect("serialize");
        let out = render_template(
            DEFAULT_WORK_ORDER_PDF_TEMPLATE,
            ctx,
            &amount_words_from_decimal(grand),
            None,
        )
        .expect("render");
        assert!(out.contains("[Logo]"), "{out}");
        assert!(out.contains("[Signature]"), "{out}");
        assert!(out.contains("KDS Tagore"));
        assert!(out.contains("Industrial Area"));
    }

    #[test]
    fn default_work_order_template_compiles_with_blank_company_prefs() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let mut root = sample_work_order_root(Utc::now(), "Asia/Kolkata");
        root.company_name.clear();
        root.company_address.clear();
        root.company_phone.clear();
        root.company_gstin.clear();
        root.place_of_supply.clear();
        root.company_logo_vnode_id = None;
        root.company_signature_vnode_id = None;
        let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
        let ctx = work_order_pdf_context(&root).expect("serialize");
        let bytes = rt
            .block_on(compile_pdf(
                DEFAULT_WORK_ORDER_PDF_TEMPLATE,
                ctx,
                amount_words_from_decimal(grand),
                None,
            ))
            .expect("blank company fields should still compile");
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn default_work_order_template_compiles_to_pdf() {
        compile_sample_template_to_pdf(SamplePdfKind::DraftWorkOrder);
    }

    #[test]
    fn default_issued_work_order_template_compiles_to_pdf() {
        compile_sample_template_to_pdf(SamplePdfKind::IssuedWorkOrder);
    }

    #[test]
    fn default_invoice_template_compiles_to_pdf() {
        compile_sample_template_to_pdf(SamplePdfKind::Quotation);
    }

    #[test]
    fn default_quotation_template_compiles_with_blank_company_prefs() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let mut root = sample_invoice_root();
        root.company_name.clear();
        root.company_address.clear();
        root.company_phone.clear();
        root.company_gstin.clear();
        root.place_of_supply.clear();
        root.company_logo_vnode_id = None;
        root.company_signature_vnode_id = None;
        let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
        let ctx = quotation_pdf_context(&root).expect("serialize");
        let bytes = rt
            .block_on(compile_pdf(
                DEFAULT_QUOTATION_PDF_TEMPLATE,
                ctx,
                amount_words_from_decimal(grand),
                None,
            ))
            .expect("blank company fields should still compile");
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn preview_quotation_template_compiles_with_company_placeholders() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let mut root = sample_invoice_root();
        root.preview = true;
        let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
        let ctx = quotation_pdf_context(&root).expect("serialize");
        let typst = render_template(
            DEFAULT_QUOTATION_PDF_TEMPLATE,
            ctx.clone(),
            &amount_words_from_decimal(grand),
            None,
        )
        .expect("render");
        assert!(
            typst.contains("#text(size: 12pt, weight: \"bold\")[KDS Tagore]"),
            "company name missing from generated Typst:\n{typst}"
        );
        assert!(
            typst.contains("[Logo]"),
            "logo placeholder missing from generated Typst:\n{typst}"
        );
        let bytes = rt
            .block_on(compile_pdf(
                DEFAULT_QUOTATION_PDF_TEMPLATE,
                ctx,
                amount_words_from_decimal(grand),
                None,
            ))
            .expect("preview placeholders should compile");
        assert!(bytes.starts_with(b"%PDF"));
    }

    enum SamplePdfKind {
        DraftWorkOrder,
        IssuedWorkOrder,
        Quotation,
    }

    fn compile_sample_template_to_pdf(kind: SamplePdfKind) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let result = rt.block_on(async {
            match kind {
                SamplePdfKind::DraftWorkOrder | SamplePdfKind::IssuedWorkOrder => {
                    let tmpl = match kind {
                        SamplePdfKind::DraftWorkOrder => DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE,
                        SamplePdfKind::IssuedWorkOrder => DEFAULT_WORK_ORDER_PDF_TEMPLATE,
                        SamplePdfKind::Quotation => unreachable!(),
                    };
                    let root = sample_work_order_root(Utc::now(), "Asia/Kolkata");
                    let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
                    let ctx = work_order_pdf_context(&root).expect("serialize");
                    compile_pdf(tmpl, ctx, amount_words_from_decimal(grand), None).await
                }
                SamplePdfKind::Quotation => {
                    let root = sample_invoice_root();
                    let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
                    let ctx = quotation_pdf_context(&root).expect("serialize");
                    compile_pdf(
                        DEFAULT_QUOTATION_PDF_TEMPLATE,
                        ctx,
                        amount_words_from_decimal(grand),
                        None,
                    )
                    .await
                }
            }
        });
        let bytes = result.expect("render preview");
        assert!(!bytes.is_empty(), "compiled PDF bytes should be non-empty");
        assert!(bytes.starts_with(b"%PDF"), "output should be a PDF");
    }

    #[test]
    fn typst_missing_value_error_names_empty_str_call() {
        let src = "#let pretax = float(str())\n#pretax\n";
        let msg = format_typst_error("missing argument: value", src);
        assert!(
            msg.contains("line 1: #let pretax = float(str())"),
            "expected snippet in:\n{msg}"
        );
        assert!(
            msg.contains("numeric placeholders"),
            "expected hint in:\n{msg}"
        );
        assert!(
            msg.contains("can stay blank"),
            "expected optional-prefs hint in:\n{msg}"
        );
    }

    #[test]
    fn typst_missing_image_source_mentions_logo_preference() {
        let src = "#image(\"\")\n";
        let msg = format_typst_error("missing argument: source", src);
        assert!(msg.contains("Quotation logo or signature"), "{msg}");
        assert!(msg.contains("line 1: #image(\"\")"), "{msg}");
    }
}
