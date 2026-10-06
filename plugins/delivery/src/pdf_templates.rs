/// Default Typst + Minijinja PDF template shipped with the Delivery plugin.
pub const DEFAULT_DELIVERY_CHALLAN_PDF_TEMPLATE: &str =
    include_str!("templates/example_delivery_challan.typ.tmpl");

fn normalize_template(s: &str) -> &str {
    s.trim()
}

/// True when `stored` is empty or an unmodified copy of the shipped example.
pub fn is_stock_delivery_challan_template(stored: Option<&str>) -> bool {
    let Some(s) = stored.map(normalize_template).filter(|s| !s.is_empty()) else {
        return true;
    };
    s == normalize_template(DEFAULT_DELIVERY_CHALLAN_PDF_TEMPLATE)
}

/// Stored template, or the shipped default when the row is blank or still the example.
pub fn resolved_delivery_challan_pdf_template(stored: Option<&str>) -> &str {
    if is_stock_delivery_challan_template(stored) {
        DEFAULT_DELIVERY_CHALLAN_PDF_TEMPLATE
    } else {
        stored
            .map(normalize_template)
            .unwrap_or(DEFAULT_DELIVERY_CHALLAN_PDF_TEMPLATE)
    }
}
