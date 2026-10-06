//! PDF rendering for delivery challans.
//!
//! Pipeline: Minijinja template → Typst source → PDF, same as quotations.

use std::sync::Arc;

use chrono::NaiveDate;
use lariv_plugin_customer::entities::customer::Entity as CustomerEntity;
use lariv_plugin_filesystem::state::FilesystemState;
use lariv_plugin_finance_common::typst::{
    typst_address_lines, typst_compile, typst_compile_in, typst_work_dir,
};
use lariv_plugin_finance_invoices::VnodeImageContext;
use lariv_plugin_finance_products::entities::product::Entity as ProductEntity;
use minijinja::{Environment, ErrorKind as MiniJinjaErrorKind};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;

use super::entities::delivery_challan::Entity as ChallanEntity;
use super::entities::delivery_challan_line::{self, Entity as LineEntity};
use super::preferences::{delivery_challan_pdf_template, load_preferences};

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
struct PdfLine {
    sr_no: i32,
    product_id: i64,
    product: String,
    qty: String,
    qty_kind: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct ChallanRoot {
    #[serde(rename = "ID")]
    id: i64,
    challan_number: String,
    date: String,
    customer_id: i64,
    customer: PdfCustomer,
    lines: Vec<PdfLine>,
    vehicle_no: String,
    eway_bill: String,
    #[serde(rename = "company_name")]
    company_name: String,
    #[serde(rename = "company_address")]
    company_address: String,
    #[serde(rename = "company_phone")]
    company_phone: String,
    #[serde(rename = "company_email")]
    company_email: String,
    #[serde(rename = "company_gstin")]
    company_gstin: String,
    #[serde(rename = "terms_and_conditions")]
    terms_and_conditions: String,
    #[serde(rename = "company_logo_vnode_id")]
    company_logo_vnode_id: Option<i64>,
    #[serde(rename = "company_signature_vnode_id")]
    company_signature_vnode_id: Option<i64>,
}

fn typst_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | '#' | '[' | ']' | '*' | '_' | '$' | '`' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape plain text and keep line breaks as Typst line breaks.
fn typst_lines(s: &str) -> String {
    s.split('\n')
        .map(str::trim_end)
        .map(typst_escape)
        .collect::<Vec<_>>()
        .join(" \\\n")
}

fn pdf_context(root: ChallanRoot) -> Result<serde_json::Value, PdfError> {
    let mut value = serde_json::to_value(root)
        .map_err(|e| PdfError::msg(format!("serialize delivery challan PDF context: {e}")))?;
    if let Some(obj) = value.as_object_mut() {
        for (snake, pascal) in [
            ("company_name", "CompanyName"),
            ("company_address", "CompanyAddress"),
            ("company_phone", "CompanyPhone"),
            ("company_email", "CompanyEmail"),
            ("company_gstin", "CompanyGstin"),
            ("terms_and_conditions", "TermsAndConditions"),
            ("company_logo_vnode_id", "CompanyLogoVnodeId"),
            ("company_signature_vnode_id", "CompanySignatureVnodeId"),
        ] {
            if let Some(v) = obj.get(snake).cloned() {
                obj.entry(pascal.to_string()).or_insert(v);
            }
        }
    }
    Ok(value)
}

fn format_date(date: NaiveDate) -> String {
    date.format(lariv_core::datetime::DATE_FMT).to_string()
}

async fn load_customer(db: &DatabaseConnection, id: i64) -> Result<PdfCustomer, PdfError> {
    let c = CustomerEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    Ok(match c {
        Some(c) => PdfCustomer {
            id: c.id,
            name: typst_escape(&c.name),
            address: c.formatted_address_for_typst(),
            gstin: c.gstin.map(|s| typst_escape(&s)),
            pan: c.pan.map(|s| typst_escape(&s)),
            phone: c.phone.map(|s| typst_escape(&s)),
            email: c.email,
            website: c.website.map(|s| typst_escape(&s)),
        },
        None => PdfCustomer {
            id,
            name: format!("Customer \\#{id}"),
            address: None,
            gstin: None,
            pan: None,
            phone: None,
            email: None,
            website: None,
        },
    })
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
    vnode_ctx: Option<&VnodeImageContext>,
) -> Result<String, PdfError> {
    let mut env = Environment::new();
    if let Some(ctx) = vnode_ctx {
        let ctx = ctx.clone();
        env.add_function(
            "vnodeImage",
            move |vnode_id: minijinja::Value| -> Result<String, minijinja::Error> {
                let vnode_id = vnode_id_i64(&vnode_id);
                if vnode_id <= 0 {
                    return Err(minijinja::Error::new(
                        MiniJinjaErrorKind::InvalidOperation,
                        "vnodeImage: no file selected. Choose a logo or signature under Delivery → Preferences.",
                    ));
                }
                ctx.resolve_sync(vnode_id).map_err(|e| {
                    minijinja::Error::new(
                        MiniJinjaErrorKind::InvalidOperation,
                        format!("{e}. Check the logo and signature under Delivery → Preferences."),
                    )
                })
            },
        );
    }
    env.add_template("delivery_challan.typ.tmpl", tmpl_src)
        .map_err(|e| PdfError::msg(format!("Template render failed: {e}")))?;
    let tmpl = env
        .get_template("delivery_challan.typ.tmpl")
        .map_err(|e| PdfError::msg(format!("Template render failed: {e}")))?;
    tmpl.render(ctx)
        .map_err(|e| PdfError::msg(format!("Template render failed: {e}")))
}

async fn compile_pdf(
    tmpl_src: &str,
    ctx: serde_json::Value,
    fs: Option<&FilesystemState>,
) -> Result<Vec<u8>, PdfError> {
    if let Some(fs) = fs {
        let work_dir = typst_work_dir();
        let vnode_ctx =
            VnodeImageContext::new(fs.db.clone(), Arc::clone(&fs.store), work_dir.clone());
        let typst_src = render_template(tmpl_src, ctx, Some(&vnode_ctx))?;
        let result = typst_compile_in(&work_dir, &typst_src)
            .await
            .map_err(|e| PdfError::msg(format!("Typst compile failed: {e}")));
        if let Err(e) = std::fs::remove_dir_all(&work_dir) {
            tracing::warn!(error = %e, path = %work_dir.display(), "failed to remove typst work dir");
        }
        result
    } else {
        let typst_src = render_template(tmpl_src, ctx, None)?;
        typst_compile(&typst_src)
            .await
            .map_err(|e| PdfError::msg(format!("Typst compile failed: {e}")))
    }
}

fn filename_base(number: &str, id: i64) -> String {
    let trimmed = number.trim();
    let raw = if trimmed.is_empty() {
        format!("delivery-challan-{id}")
    } else {
        trimmed.to_string()
    };
    let mut s = raw;
    for ch in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
        s = s.replace(ch, "-");
    }
    s
}

/// Render a delivery challan to PDF bytes using the configured template.
pub async fn render_challan_pdf(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    id: i64,
) -> Result<PdfResult, PdfError> {
    let challan = ChallanEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?
        .ok_or(PdfError::NotFound)?;

    let lines = LineEntity::find()
        .filter(delivery_challan_line::Column::DeliveryChallanId.eq(challan.id))
        .order_by_asc(delivery_challan_line::Column::SrNo)
        .order_by_asc(delivery_challan_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;

    let product_ids: Vec<i64> = lines.iter().map(|l| l.product_id).collect();
    let products = if product_ids.is_empty() {
        Vec::new()
    } else {
        ProductEntity::find()
            .filter(lariv_plugin_finance_products::entities::product::Column::Id.is_in(product_ids))
            .all(db)
            .await
            .map_err(|e| PdfError::msg(e.to_string()))?
    };
    let product_names: std::collections::HashMap<i64, String> =
        products.into_iter().map(|p| (p.id, p.name)).collect();

    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::msg(e.to_string()))?;
    let tmpl_src = delivery_challan_pdf_template(&prefs).to_string();
    let customer = load_customer(db, challan.customer_id).await?;
    let pdf_lines = lines
        .iter()
        .map(|line| {
            let product = product_names
                .get(&line.product_id)
                .map(|name| typst_escape(name))
                .unwrap_or_else(|| format!("Product \\#{}", line.product_id));
            PdfLine {
                sr_no: line.sr_no,
                product_id: line.product_id,
                product,
                qty: typst_escape(&line.qty_display()),
                qty_kind: line.qty_kind.clone(),
            }
        })
        .collect();

    let root = ChallanRoot {
        id: challan.id,
        challan_number: typst_escape(&challan.challan_number),
        date: format_date(challan.date),
        customer_id: challan.customer_id,
        customer,
        lines: pdf_lines,
        vehicle_no: typst_escape(challan.vehicle_no.as_deref().unwrap_or("")),
        eway_bill: typst_escape(challan.eway_bill.as_deref().unwrap_or("")),
        company_name: typst_escape(&prefs.company_name.clone().unwrap_or_default()),
        company_address: typst_address_lines(&prefs.company_address.clone().unwrap_or_default()),
        company_phone: typst_escape(&prefs.company_phone.clone().unwrap_or_default()),
        company_email: prefs.company_email.clone().unwrap_or_default(),
        company_gstin: typst_escape(&prefs.company_gstin.clone().unwrap_or_default()),
        terms_and_conditions: typst_lines(&prefs.terms_and_conditions.clone().unwrap_or_default()),
        company_logo_vnode_id: prefs.logo_vnode_id.filter(|&id| id > 0),
        company_signature_vnode_id: prefs.signature_vnode_id.filter(|&id| id > 0),
    };
    let ctx = pdf_context(root)?;
    let bytes = compile_pdf(&tmpl_src, ctx, fs).await?;
    Ok(PdfResult {
        bytes,
        filename_base: filename_base(&challan.challan_number, challan.id),
    })
}
