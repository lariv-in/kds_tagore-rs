use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form, widgets::Select, widgets::Text,
};
use lariv_plugin_crm::routes::CustomerFkSelectRouteTag;
use lariv_plugin_filesystem::routes::VNodeNodeSelectRouteTag;

use crate::line_editors::{parts_lines_editor, presupplied_lines_editor, steps_lines_editor};

pub struct PartsLines;
impl FormWidget for PartsLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> maud::Markup {
        parts_lines_editor(field.label, field.spec.hint, field.name, field.value)
    }
}

pub struct PresuppliedLines;
impl FormWidget for PresuppliedLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> maud::Markup {
        presupplied_lines_editor(field.label, field.spec.hint, field.name, field.value)
    }
}

pub struct StepsLines;
impl FormWidget for StepsLines {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> maud::Markup {
        steps_lines_editor(field.label, field.spec.hint, field.name, field.value)
    }
}

#[html_form]
pub struct WorkOrderForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(
        label = "Customer",
        required,
        widget = ForeignKey,
        route = CustomerFkSelectRouteTag,
        swap_key = "fk-work-order-customer",
        display = "customer",
        placeholder = "Select customer…"
    )]
    pub customer_id: i64,

    #[form(
        label = "Files",
        required,
        widget = ForeignKey,
        route = VNodeNodeSelectRouteTag,
        swap_key = "fk-work-order-files",
        display = "vnode",
        placeholder = "Select a file or folder…"
    )]
    pub folder_vnode_id: i64,

    #[form(
        label = "Parts",
        widget = PartsLines,
        hint = "Each line is a part on this work order. Kind chooses which units are accepted."
    )]
    pub parts_lines_json: String,

    #[form(
        label = "Presupplied parts",
        widget = PresuppliedLines,
        hint = "Quantity already on hand for a part from the parts lines."
    )]
    pub presupplied_lines_json: String,

    #[form(
        label = "Steps",
        widget = StepsLines,
        hint = "Each line is one step. Inputs and outputs are the quantity for a single batch."
    )]
    pub steps_lines_json: String,
}

#[html_form]
pub struct PartForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Kind", required, widget = Select, choices = "measure_kind")]
    pub measure_kind: String,

    #[form(
        label = "Unit",
        required,
        widget = Text,
        hint = "Length: mm, cm, m, in, ft. Weight: mg, g, kg, t, lb. Quantity: Number. Decimal: None."
    )]
    pub measure_unit: String,
}

#[html_form]
pub struct PresuppliedForm {
    #[form(label = "Part", required, widget = Select, choices = "part")]
    pub part_id: String,

    #[form(label = "Quantity", required, widget = Text)]
    pub qty_value: String,

    #[form(label = "Unit", required, widget = Text)]
    pub qty_unit: String,
}

#[html_form]
pub struct StepForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Planned batches", required, widget = Text)]
    pub planned_batches: String,
}

#[html_form]
pub struct FlowForm {
    #[form(label = "Part", required, widget = Select, choices = "part")]
    pub part_id: String,

    #[form(label = "Quantity per batch", required, widget = Text)]
    pub qty_value: String,

    #[form(label = "Unit", required, widget = Text)]
    pub qty_unit: String,
}
