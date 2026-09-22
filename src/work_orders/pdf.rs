//! PDF rendering for draft work orders and quotations.
//!
//! Pipeline: Minijinja template (Jinja2-style) → Typst source → PDF via the
//! `typst` crate, mirroring the finance invoices renderer (`electronics style`).

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use lariv_rs::plugins::customer::entities::customer::Entity as CustomerEntity;
use minijinja::Environment;
use num2words::{Lang, Num2Words};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;

use super::entities::{
    component, draft_work_order, draft_work_order_machine_line, draft_work_order_material_line,
    quotation, quotation_machine_line, quotation_material_line,
};
use super::preferences::{draft_work_order_pdf_template, load_preferences, quotation_pdf_template};
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
struct PdfMaterialLine {
    #[serde(rename = "ID")]
    id: i64,
    component_id: i64,
    component: String,
    variables_display: String,
    final_cost: String,
    taxes: String,
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
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct InvoiceRoot {
    #[serde(rename = "ID")]
    id: i64,
    invoice_number: String,
    customer_id: i64,
    date: String,
    duration: String,
    customer: PdfCustomer,
    material_lines: Vec<PdfMaterialLine>,
    machine_lines: Vec<PdfMachineLine>,
    grand_total: String,
}

fn dec_str(d: Decimal) -> String {
    d.normalize().to_string()
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

fn render_template(
    tmpl_src: &str,
    ctx: serde_json::Value,
    grand_words: &str,
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
    let tmpl = env
        .template_from_str(tmpl_src)
        .map_err(|e| PdfError::msg(format!("invalid PDF template: {e}")))?;
    tmpl.render(ctx)
        .map_err(|e| PdfError::msg(format!("rendering PDF template failed: {e}")))
}

async fn compile_pdf(
    tmpl_src: &str,
    ctx: serde_json::Value,
    grand_words: String,
) -> Result<Vec<u8>, PdfError> {
    let typst_src = render_template(tmpl_src, ctx, &grand_words)?;
    lariv_rs::plugins::finance_common::typst::typst_compile(&typst_src)
        .await
        .map_err(PdfError::msg)
}

/// Render a draft work order to PDF bytes using the configured template.
pub async fn render_work_order_pdf(
    db: &DatabaseConnection,
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
    let machine_map: HashMap<i64, String> = machines.into_iter().map(|m| (m.id, m.name)).collect();

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
                machine: machine_map
                    .get(&l.machine_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Machine #{}", l.machine_id)),
                variables_display: l.format_variables_display(),
                pre_tax: dec_str(l.line_total()),
                taxes: tax_assoc::tax_labels_display(taxes),
                line_total: dec_str(l.taxed_total(taxes)),
            }
        })
        .collect();

    let created_at =
        lariv_rs::datetime::format_date_in_tz(order.created_at.unwrap_or(Utc::now()), tz);
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
    };
    let ctx = serde_json::to_value(&root)
        .map_err(|e| PdfError::msg(format!("serialize work order PDF context: {e}")))?;
    let bytes = compile_pdf(&tmpl_src, ctx, amount_words_from_decimal(total_amount)).await?;
    let base = pdf_filename_base(
        Some(&order.order_number),
        &format!("draft-work-order-{}", order.id),
    );
    Ok(PdfResult {
        bytes,
        filename_base: base,
    })
}

/// Render a quotation to PDF bytes using the configured template.
pub async fn render_quotation_pdf(
    db: &DatabaseConnection,
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
    let machine_map: HashMap<i64, String> = machines.into_iter().map(|m| (m.id, m.name)).collect();

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
                    .and_then(|id| machine_map.get(&id).cloned())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| l.name.clone()),
                variables_display: l.format_variables_display(),
                pre_tax: dec_str(l.line_total()),
                taxes: tax_assoc::tax_labels_display(taxes),
                line_total: dec_str(l.taxed_total(taxes)),
            }
        })
        .collect();

    let date = lariv_rs::datetime::format_date(inv.date);
    let root = InvoiceRoot {
        id: inv.id,
        invoice_number: inv.invoice_number.clone(),
        customer_id: inv.customer_id,
        date,
        duration: format_job_duration(inv.duration),
        customer,
        material_lines: pdf_material_lines,
        machine_lines: pdf_machine_lines,
        grand_total: dec_str(grand_total),
    };
    let ctx = serde_json::to_value(&root)
        .map_err(|e| PdfError::msg(format!("serialize quotation PDF context: {e}")))?;
    let bytes = compile_pdf(&tmpl_src, ctx, amount_words_from_decimal(grand_total)).await?;
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
        website: None,
    }
}

fn sample_work_order_root(dt: DateTime<Utc>, tz: &str) -> WorkOrderRoot {
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
            final_cost: "351.05".into(),
        }],
        machine_lines: vec![PdfMachineLine {
            id: 1,
            machine_id: 1,
            machine: "CNC Lathe".into(),
            variables_display: "duration: 2h 30m".into(),
            pre_tax: "2375".into(),
            taxes: "GST 18%".into(),
            line_total: "2802.5".into(),
        }],
        total_amount: "3153.55".into(),
    }
}

fn sample_invoice_root() -> InvoiceRoot {
    InvoiceRoot {
        id: 1,
        invoice_number: "PF/2026/0042".into(),
        customer_id: 1,
        date: lariv_rs::datetime::format_date(
            NaiveDate::from_ymd_opt(2026, 2, 8).expect("valid date"),
        ),
        duration: "3h".into(),
        customer: sample_customer(),
        material_lines: vec![PdfMaterialLine {
            id: 1,
            component_id: 1,
            component: "MS Flat Bar 2.5x3.5mm — 1000mm".into(),
            variables_display: "length: 1000 mm, qty: 50".into(),
            pre_tax: "4250".into(),
            taxes: "GST 18%".into(),
            final_cost: "5015".into(),
        }],
        machine_lines: vec![PdfMachineLine {
            id: 1,
            machine_id: 1,
            machine: "CNC Lathe".into(),
            variables_display: "duration: 3h".into(),
            pre_tax: "2850".into(),
            taxes: "GST 18%".into(),
            line_total: "3363".into(),
        }],
        grand_total: "8378".into(),
    }
}

/// Render a sample draft work order PDF using an optional template override.
pub async fn render_work_order_pdf_preview(
    db: &DatabaseConnection,
    template_src: Option<&str>,
    tz: &str,
) -> Result<PdfResult, PdfError> {
    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = template_src
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| draft_work_order_pdf_template(&prefs));
    let root = sample_work_order_root(Utc::now(), tz);
    let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
    let ctx = serde_json::to_value(&root)
        .map_err(|e| PdfError::msg(format!("serialize work order PDF context: {e}")))?;
    let bytes = compile_pdf(tmpl_src, ctx, amount_words_from_decimal(grand)).await?;
    Ok(PdfResult {
        bytes,
        filename_base: "draft-work-order-preview".to_string(),
    })
}

/// Render a sample quotation PDF using an optional template override.
pub async fn render_quotation_pdf_preview(
    db: &DatabaseConnection,
    template_src: Option<&str>,
) -> Result<PdfResult, PdfError> {
    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = template_src
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| quotation_pdf_template(&prefs));
    let root = sample_invoice_root();
    let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
    let ctx = serde_json::to_value(&root)
        .map_err(|e| PdfError::msg(format!("serialize quotation PDF context: {e}")))?;
    let bytes = compile_pdf(tmpl_src, ctx, amount_words_from_decimal(grand)).await?;
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
        let ctx = serde_json::to_value(&root).expect("serialize");
        let out = render_template(
            DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE,
            ctx,
            &amount_words_from_decimal(grand),
        )
        .expect("render");
        let out_lower = out.to_lowercase();
        assert!(
            out_lower.contains("acme industries"),
            "customer name missing:\n{out}"
        );
        assert!(out.contains("DRAFT WORK ORDER"));
        assert!(out.contains("WO/2026/0001"));
    }

    #[test]
    fn default_invoice_template_renders() {
        let root = sample_invoice_root();
        let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
        let ctx = serde_json::to_value(&root).expect("serialize");
        let out = render_template(
            DEFAULT_QUOTATION_PDF_TEMPLATE,
            ctx,
            &amount_words_from_decimal(grand),
        )
        .expect("render");
        assert!(out.contains("QUOTATION"));
        assert!(out.contains("PF/2026/0042"));
    }

    #[test]
    fn default_work_order_template_compiles_to_pdf() {
        compile_sample_template_to_pdf(true);
    }

    #[test]
    fn default_invoice_template_compiles_to_pdf() {
        compile_sample_template_to_pdf(false);
    }

    fn compile_sample_template_to_pdf(work_order: bool) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let result = rt.block_on(async {
            if work_order {
                let root = sample_work_order_root(Utc::now(), "Asia/Kolkata");
                let grand: Decimal = root.total_amount.parse().unwrap_or(Decimal::ZERO);
                let ctx = serde_json::to_value(&root).expect("serialize");
                compile_pdf(
                    DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE,
                    ctx,
                    amount_words_from_decimal(grand),
                )
                .await
            } else {
                let root = sample_invoice_root();
                let grand: Decimal = root.grand_total.parse().unwrap_or(Decimal::ZERO);
                let ctx = serde_json::to_value(&root).expect("serialize");
                compile_pdf(
                    DEFAULT_QUOTATION_PDF_TEMPLATE,
                    ctx,
                    amount_words_from_decimal(grand),
                )
                .await
            }
        });
        let bytes = result.expect("render preview");
        assert!(!bytes.is_empty(), "compiled PDF bytes should be non-empty");
        assert!(bytes.starts_with(b"%PDF"), "output should be a PDF");
    }
}
