use lariv_rs::components::{label_hint, attrs::escape_attr};
use lariv_rs::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Duration, ManyToMany, Number, Text, Textarea},
};
use maud::{Markup, PreEscaped, html};

use super::routes::MachineFkSelectRouteTag;

/// Number input with a `%` specifier, 0–100 range, and a completed-job warning.
pub struct ProgressPercent;

impl FormWidget for ProgressPercent {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let hint = ctx.hint_of(field.spec);
        let required = if field.required { " required" } else { "" };
        label_hint(
            field.label,
            hint,
            html! {
                div class="join w-full" {
                    (PreEscaped(format!(
                        r#"<input type="number" name="{name}" value="{value}" min="0" max="100" step="1" inputmode="numeric" class="input input-bordered join-item w-full" x-model.number="progress"{required}>"#,
                        name = escape_attr(field.name),
                        value = escape_attr(field.value),
                        required = required,
                    )))
                    span class="btn join-item no-animation pointer-events-none" { "%" }
                }
                p class="text-sm opacity-70 mt-1" {
                    "Enter a percentage from 0 to 100."
                }
                p class="text-sm text-warning mt-1" x-show="progress === 100" x-cloak {
                    "Saving at 100% will move this job to Completed."
                }
            },
        )
    }
}

#[html_form]
pub struct MachineForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,
}

#[html_form]
pub struct MachineFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}

#[html_form]
pub struct JobForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(
        label = "Machines",
        widget = ManyToMany,
        route = MachineFkSelectRouteTag,
        swap_key = "ms-job-machines",
        placeholder = "Select machines…"
    )]
    pub machines: Vec<i64>,

    #[form(label = "Duration", required, widget = Duration)]
    pub duration: String,

    #[form(
        label = "Files",
        widget = ManyToMany,
        url = "/filesystem/file-select/",
        swap_key = "ms-job-files",
        placeholder = "Select files…"
    )]
    pub files: Vec<i64>,

    #[form(label = "Order", widget = Number)]
    pub order: i64,

    #[form(label = "Remarks", widget = Textarea, rows = 3)]
    pub remarks: String,

    #[form(
        label = "Progress",
        widget = ProgressPercent,
        hint = "0–100. Saving at 100% moves this job to Completed."
    )]
    pub progress: i64,
}

#[html_form]
pub struct JobFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}
