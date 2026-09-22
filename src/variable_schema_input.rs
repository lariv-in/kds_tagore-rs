//! Variable schema editor: name string + searchable type dropdown.

use lariv_rs::components::input::single_choice_combobox_alpine_shell;
use lariv_rs::components::{icon, label_hint};
use lariv_rs::html_form::{FieldRender, FormCtx, FormWidget};
use maud::{Markup, PreEscaped, html};

use crate::formula::VariableType;

/// Rows of named formula variables with a searchable type picker.
pub struct VariableSchemaList;

impl FormWidget for VariableSchemaList {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let from_ctx = ctx.list_of(field.name);
        let from_value: Vec<String> = if from_ctx.is_empty() && !field.value.is_empty() {
            field
                .value
                .split(|c: char| c == '\n' || c == '\r' || c == ',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        } else {
            Vec::new()
        };
        let items = if from_ctx.is_empty() {
            from_value.as_slice()
        } else {
            from_ctx
        };
        label_hint(
            field.label,
            ctx.hint_of(field.spec),
            input_variable_schema(InputVariableSchema {
                name: field.name,
                items,
                name_placeholder: field.spec.placeholder.unwrap_or("Variable name"),
                ..Default::default()
            }),
        )
    }
}

pub struct InputVariableSchema<'a> {
    pub name: &'a str,
    pub items: &'a [String],
    pub name_placeholder: &'a str,
    pub classes: &'a str,
}

impl Default for InputVariableSchema<'_> {
    fn default() -> Self {
        Self {
            name: "",
            items: &[],
            name_placeholder: "Variable name",
            classes: "",
        }
    }
}

fn variable_type_choices() -> Vec<(String, String)> {
    VariableType::all()
        .iter()
        .map(|ty| (ty.as_str().to_string(), ty.label().to_string()))
        .collect()
}

pub(crate) fn parse_schema_entry(raw: &str) -> Option<(String, String)> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    match s.split_once(':') {
        Some((name, ty)) => Some((name.trim().to_string(), ty.trim().to_string())),
        None => Some((s.to_string(), String::new())),
    }
}

fn items_json(entries: &[String]) -> (String, usize) {
    let parsed: Vec<(String, String)> = entries
        .iter()
        .filter_map(|s| parse_schema_entry(s))
        .collect();
    let rows: Vec<serde_json::Value> = if parsed.is_empty() {
        vec![serde_json::json!({ "id": 1, "name": "", "type": "" })]
    } else {
        parsed
            .into_iter()
            .enumerate()
            .map(|(i, (name, ty))| serde_json::json!({ "id": i + 1, "name": name, "type": ty }))
            .collect()
    };
    let next_id = rows.len() + 1;
    let json = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());
    (json, next_id)
}

fn type_picker_alpine_factory(types_json: &str) -> String {
    format!(
        r#"variableTypes: {types_json}.map(([key, label]) => ({{ key, label }})),
typeLabel(kind) {{
    const choice = this.variableTypes.find((item) => item.key === kind);
    return choice ? choice.label : (kind || '');
}},
typePickerData(row) {{
    const parent = this;
    return {{
        choices: parent.variableTypes,
        row,
        value: row.type,
        query: parent.typeLabel(row.type),
        error: '',
        open: false,
        highlight: 0,
        placeholder: 'Search type…',
        labelFor(key) {{
            const choice = this.choices.find((item) => item.key === key);
            return choice ? choice.label : key;
        }},
        filtered() {{
            const q = String(this.query || '').trim().toLowerCase();
            return this.choices.filter((choice) => {{
                if (!q) return true;
                return choice.label.toLowerCase().includes(q) || choice.key.toLowerCase().includes(q);
            }});
        }},
        unknownError(q) {{
            return 'No listed option matches ' + JSON.stringify(q) + '.';
        }},
        syncError() {{
            const q = String(this.query || '').trim();
            this.error = q && !this.filtered().length ? this.unknownError(q) : '';
        }},
        select(choice) {{
            if (!choice) return;
            this.row.type = choice.key;
            this.value = choice.key;
            this.query = choice.label;
            this.error = '';
            this.highlight = 0;
            this.open = false;
        }},
        commitQuery() {{
            const q = String(this.query || '').trim();
            if (!q) {{
                this.error = '';
                return true;
            }}
            const options = this.filtered();
            if (options.length) {{
                const idx = Math.max(0, Math.min(this.highlight, options.length - 1));
                this.select(options[idx]);
                return true;
            }}
            this.error = this.unknownError(q);
            this.open = false;
            this.$nextTick(() => this.$refs.query && this.$refs.query.focus());
            return false;
        }},
        bindForm() {{
            const form = this.$el.closest('form');
            if (!form || this._bound) return;
            this._bound = true;
            form.addEventListener('submit', (event) => {{
                if (!this.commitQuery()) {{
                    event.preventDefault();
                    event.stopImmediatePropagation();
                }}
            }});
        }},
        onFocusOut(event) {{
            if (this.$el.contains(event.relatedTarget)) return;
            this.open = false;
            const q = String(this.query || '').trim();
            if (!q || this.choices.some((choice) => choice.label === q)) {{
                this.query = this.labelFor(this.row.type);
                this.error = '';
                return;
            }}
            if (this.filtered().length) {{
                this.query = this.labelFor(this.row.type);
                this.error = '';
                return;
            }}
            this.error = this.unknownError(q);
        }},
        move(delta) {{
            const options = this.filtered();
            if (!options.length) {{
                this.open = true;
                return;
            }}
            this.open = true;
            const next = this.highlight + delta;
            this.highlight = (next + options.length) % options.length;
        }},
        onKey(event) {{
            if (event.key === 'ArrowDown') {{
                event.preventDefault();
                this.move(1);
            }} else if (event.key === 'ArrowUp') {{
                event.preventDefault();
                this.move(-1);
            }} else if (event.key === 'Enter') {{
                event.preventDefault();
                this.commitQuery();
            }} else if (event.key === 'Escape') {{
                this.open = false;
                this.query = this.labelFor(this.row.type);
                this.error = '';
            }}
        }},
    }};
}},"#
    )
}

fn alpine_methods() -> &'static str {
    r#"encoded(item) {
    const name = String(item.name || '').trim();
    const type = String(item.type || '').trim();
    if (!name) return '';
    return name + ':' + type;
},
add() {
    this.items.push({ id: this.nextId++, name: '', type: '' });
    this.$nextTick(() => {
        const inputs = this.$el.querySelectorAll('[data-var-name-input]');
        const last = inputs[inputs.length - 1];
        if (last) last.focus();
    });
},
remove(idx) {
    if (this.items.length <= 1) {
        this.items = [{ id: this.nextId++, name: '', type: '' }];
        return;
    }
    this.items.splice(idx, 1);
},
moveUp(idx) {
    if (idx <= 0) return;
    const arr = this.items.slice();
    const tmp = arr[idx - 1];
    arr[idx - 1] = arr[idx];
    arr[idx] = tmp;
    this.items = arr;
},
moveDown(idx) {
    if (idx >= this.items.length - 1) return;
    const arr = this.items.slice();
    const tmp = arr[idx + 1];
    arr[idx + 1] = arr[idx];
    arr[idx] = tmp;
    this.items = arr;
},
onNameKey(e, idx) {
    if (e.key === 'Enter') {
        e.preventDefault();
        if (idx === this.items.length - 1) this.add();
    }
}"#
}

/// Render editable variable rows: identifier plus searchable [`VariableType`] dropdown.
pub fn input_variable_schema(opts: InputVariableSchema<'_>) -> Markup {
    let choices = variable_type_choices();
    let types_json = serde_json::to_string(&choices).unwrap_or_else(|_| "[]".into());
    let (rows_json, next_id) = items_json(opts.items);
    let placeholder_json =
        serde_json::to_string(opts.name_placeholder).unwrap_or_else(|_| "\"Variable name\"".into());
    let alpine_data = format!(
        "{{ items: {rows}, nextId: {next_id}, namePlaceholder: {placeholder}, {type_picker} {methods} }}",
        rows = rows_json,
        next_id = next_id,
        placeholder = placeholder_json,
        type_picker = type_picker_alpine_factory(&types_json),
        methods = alpine_methods(),
    );
    let name = opts.name;

    html! {
        div class=(format!("w-full {}", opts.classes)) x-data=(alpine_data) {
            div class="flex flex-col items-stretch gap-1 w-full overflow-visible" {
                div class="flex items-center gap-1 w-full px-1 text-[10px] uppercase tracking-wide opacity-60 font-semibold" {
                    span class="w-8 shrink-0" {}
                    span class="grow min-w-0" { "Name" }
                    span class="w-40 shrink-0" { "Type" }
                    span class="w-8 shrink-0" {}
                }
                div class="flex flex-col gap-2 w-full overflow-visible" {
                    template x-for="(item, idx) in items" x-bind:key="item.id" {
                        (PreEscaped(r#"<div class="flex items-center gap-1 w-full overflow-visible">"#))
                        div class="flex flex-col shrink-0" {
                            (PreEscaped(
                                r#"<button type="button" class="btn btn-ghost btn-square btn-xs" @click="moveUp(idx)" :disabled="idx === 0" aria-label="Move up">"#
                            ))
                            (icon("chevron-up", ""))
                            (PreEscaped("</button>"))
                            (PreEscaped(
                                r#"<button type="button" class="btn btn-ghost btn-square btn-xs" @click="moveDown(idx)" :disabled="idx === items.length - 1" aria-label="Move down">"#
                            ))
                            (icon("chevron-down", ""))
                            (PreEscaped("</button>"))
                        }
                        (PreEscaped(format!(
                            r#"<input type="hidden" name="{name}" :value="encoded(item)">
                            <input type="text" class="input input-bordered w-full min-w-0 h-12" x-model="item.name" data-var-name-input @keydown="onNameKey($event, idx)" :placeholder="namePlaceholder" autocomplete="off" spellcheck="false">"#,
                            name = lariv_rs::components::attrs::escape_attr(name),
                        )))
                        div class="w-40 shrink-0 min-w-[10rem] [&_.input]:min-h-12 [&_.input]:h-12 [&_.input]:py-0" x-data="typePickerData(item)" x-init="bindForm()" {
                            (single_choice_combobox_alpine_shell(false, "", ""))
                        }
                        (PreEscaped(
                            r#"<button type="button" class="btn btn-ghost btn-square btn-sm shrink-0" @click="remove(idx)" aria-label="Remove">"#
                        ))
                        (icon("x-mark", ""))
                        (PreEscaped("</button></div>"))
                    }
                }
                (PreEscaped(
                    r#"<button type="button" class="btn btn-outline btn-sm self-start gap-1" @click="add()">"#
                ))
                (icon("plus", ""))
                "Add"
                (PreEscaped("</button>"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_schema_entry_splits_name_and_type() {
        assert_eq!(
            parse_schema_entry(" length : duration "),
            Some(("length".into(), "duration".into()))
        );
        assert_eq!(
            parse_schema_entry("qty"),
            Some(("qty".into(), String::new()))
        );
        assert_eq!(parse_schema_entry("  "), None);
    }

    #[test]
    fn input_lists_all_variable_types_and_combobox() {
        let html = input_variable_schema(InputVariableSchema {
            name: "Variables",
            items: &["length:length".into(), "qty:quantity".into()],
            ..Default::default()
        })
        .into_string();
        assert!(html.contains(r#"name="Variables""#));
        assert!(html.contains("role=\"combobox\""));
        assert!(html.contains("h-12"));
        assert!(!html.contains("input-sm"));
        assert!(html.contains("length"));
        assert!(html.contains("weight"));
        assert!(html.contains("duration"));
        assert!(html.contains("quantity"));
        assert!(html.contains("Search type"));
        assert!(html.contains("typePickerData"));
        assert!(
            html.contains(r#"&quot;name&quot;:&quot;length&quot;"#)
                || html.contains(r#""name":"length""#)
        );
        assert!(
            html.contains(r#"&quot;type&quot;:&quot;quantity&quot;"#)
                || html.contains(r#""type":"quantity""#)
        );
    }
}
