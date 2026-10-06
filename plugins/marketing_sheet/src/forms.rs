use lariv_core::html_form::{Upload, html_form, widgets::File};

#[html_form(default)]
pub struct ImportForm {
    #[form(label = "XLSX file", widget = File, accept = ".xlsx", required)]
    pub file: Upload,
}
