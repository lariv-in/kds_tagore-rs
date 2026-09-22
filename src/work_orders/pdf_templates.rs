/// Default Typst + Minijinja PDF templates shipped with the work orders plugin.
pub const DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE: &str =
    include_str!("templates/example_draft_work_order_pdf.typ.tmpl");

pub const DEFAULT_WORK_ORDER_PDF_TEMPLATE: &str =
    include_str!("templates/example_work_order_pdf.typ.tmpl");

pub const DEFAULT_QUOTATION_PDF_TEMPLATE: &str =
    include_str!("templates/example_quotation_pdf.typ.tmpl");

/// The quotation template shipped before company presentation fields existed.
/// Rows that still store this copy are treated as "use the current default".
const LEGACY_QUOTATION_PDF_TEMPLATE: &str = include_str!("templates/legacy_quotation_pdf.typ.tmpl");

/// Draft work order template shipped before company presentation fields existed.
const LEGACY_DRAFT_WORK_ORDER_PDF_TEMPLATE: &str =
    include_str!("templates/legacy_draft_work_order_pdf.typ.tmpl");

/// Finalized work order template shipped before company presentation fields existed.
const LEGACY_WORK_ORDER_PDF_TEMPLATE: &str =
    include_str!("templates/legacy_work_order_pdf.typ.tmpl");

fn normalize_template(s: &str) -> &str {
    s.trim()
}

fn is_stock_template(stored: Option<&str>, current: &str, legacy: &str) -> bool {
    let Some(s) = stored.map(normalize_template).filter(|s| !s.is_empty()) else {
        return true;
    };
    s == normalize_template(current)
        || s == normalize_template(legacy)
        || (!s.contains("company_name") && !s.contains("CompanyName"))
}

fn resolved_template<'a>(stored: Option<&'a str>, current: &'a str, legacy: &'a str) -> &'a str {
    if is_stock_template(stored, current, legacy) {
        current
    } else {
        stored.map(normalize_template).unwrap_or(current)
    }
}

/// True when `stored` is empty, an unmodified shipped example, or a copy that
/// never interpolates seller fields (so preference company/logo cannot show).
pub fn is_stock_quotation_template(stored: Option<&str>) -> bool {
    is_stock_template(
        stored,
        DEFAULT_QUOTATION_PDF_TEMPLATE,
        LEGACY_QUOTATION_PDF_TEMPLATE,
    )
}

/// Stored quotation template, or the current default when the row still has a
/// shipped example (including the pre-presentation copy).
pub fn resolved_quotation_pdf_template(stored: Option<&str>) -> &str {
    resolved_template(
        stored,
        DEFAULT_QUOTATION_PDF_TEMPLATE,
        LEGACY_QUOTATION_PDF_TEMPLATE,
    )
}

pub fn is_stock_draft_work_order_template(stored: Option<&str>) -> bool {
    is_stock_template(
        stored,
        DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE,
        LEGACY_DRAFT_WORK_ORDER_PDF_TEMPLATE,
    )
}

pub fn resolved_draft_work_order_pdf_template(stored: Option<&str>) -> &str {
    resolved_template(
        stored,
        DEFAULT_DRAFT_WORK_ORDER_PDF_TEMPLATE,
        LEGACY_DRAFT_WORK_ORDER_PDF_TEMPLATE,
    )
}

pub fn is_stock_work_order_template(stored: Option<&str>) -> bool {
    is_stock_template(
        stored,
        DEFAULT_WORK_ORDER_PDF_TEMPLATE,
        LEGACY_WORK_ORDER_PDF_TEMPLATE,
    )
}

pub fn resolved_work_order_pdf_template(stored: Option<&str>) -> &str {
    resolved_template(
        stored,
        DEFAULT_WORK_ORDER_PDF_TEMPLATE,
        LEGACY_WORK_ORDER_PDF_TEMPLATE,
    )
}
