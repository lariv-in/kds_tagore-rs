use lariv_rs::{
    components::{
        InputForeignKey, InputLength,
        attrs::{HtmlAttrs, escape_attr},
        icon, input_foreign_key, input_length, label,
        swap::SwapKey,
    },
    html_form::{
        FieldRender, FormCtx, FormWidget, html_form,
        widgets::{CodeEditor, Section, Text, Textarea},
    },
    plugins::filesystem::routes::VNodeFileSelectRouteTag,
};
use maud::{Markup, PreEscaped, html};

use super::challan_number::CHALLAN_NUMBER_FORMAT_HINT;

/// Embeddable length control (no form `name`) for Alpine line rows.
fn embed_input_length() -> Markup {
    input_length(InputLength {
        label: "",
        name: "",
        value: "",
        unit: "mm",
        required: false,
        classes: "!my-0",
        attrs: Default::default(),
    })
}

fn embed_input_fkey(url: &str, placeholder: &'static str) -> Markup {
    input_foreign_key(InputForeignKey {
        label: "",
        name: "",
        value: "",
        display: "",
        placeholder,
        url,
        required: false,
        classes: "!my-0",
        ..Default::default()
    })
}

/// Sync Alpine line widgets into the HTMX request body before every form POST.
pub const SYNC_LINES_HTMX_CONFIG_REQUEST: &str = r#"var ctx=event.detail&&(event.detail.ctx||event.detail);var body=ctx&&ctx.request&&ctx.request.body;if(!body||typeof body.set!=='function')return;document.querySelectorAll('[data-lines-root]').forEach(function(el){if(!window.Alpine)return;var d=Alpine.$data(el);if(!d||typeof d.jsonOutput!=='function')return;if(typeof d.syncLineItemsFromFkeyPickers==='function')d.syncLineItemsFromFkeyPickers();if(typeof d.syncQtyFromDom==='function')d.syncQtyFromDom();var h=el.querySelector('[data-lines-hidden]');if(!h||!h.name)return;var json='[]';try{json=d.jsonOutput()}catch(e){}h.value=json;body.set(h.name,json)})"#;

pub fn lines_form_hx_post<K: SwapKey>(url: &str) -> HtmlAttrs {
    lariv_rs::components::swap::form_hx_post_url::<K>(url)
        .set("hx-on::config:request", SYNC_LINES_HTMX_CONFIG_REQUEST)
        .set("hx-on:htmx:config:request", SYNC_LINES_HTMX_CONFIG_REQUEST)
}

const ALPINE_LINES: &str = r#"
{
    items: __ROWS__,
    nextId: __NEXT__,
    lengthData(el) {
        const root = el.querySelector('[x-data]');
        return root && window.Alpine ? Alpine.$data(root) : null;
    },
    bindLengthInput(el, target) {
        this.$nextTick(() => {
            const d = this.lengthData(el);
            if (!d) return;
            d.unit = target.qty_unit || 'mm';
            const mm = target.qty_mm;
            d.mm = (mm !== '' && mm !== null && mm !== undefined) ? String(mm) : '';
            if (typeof d.mmToDisplay === 'function') d.mmToDisplay();
        });
    },
    pullLengthInput(el, target) {
        const d = this.lengthData(el);
        if (!d) return;
        target.qty_mm = d.mm;
        target.qty_unit = d.unit || 'mm';
    },
    syncQtyFromDom() {
        this.$el.querySelectorAll('[data-qty-length]').forEach((el) => {
            const itemId = parseInt(el.getAttribute('data-line-item-id'), 10);
            const item = (this.items || []).find((it) => it.id === itemId);
            if (item) this.pullLengthInput(el, item);
        });
    },
    fkeyRoot(el) {
        const results = el.querySelector('.fk-picker-results');
        return results ? results.closest('[x-data]') : null;
    },
    fkeyData(el) {
        const root = this.fkeyRoot(el);
        return root && window.Alpine ? Alpine.$data(root) : null;
    },
    associatePickerForm(search, tableBtn, formId) {
        if (!formId) return;
        let f = document.getElementById(formId);
        if (!f) {
            f = document.createElement('form');
            f.id = formId;
            f.hidden = true;
            f.setAttribute('aria-hidden', 'true');
            document.body.appendChild(f);
        }
        if (search) search.setAttribute('form', formId);
        if (tableBtn) {
            tableBtn.setAttribute('form', formId);
            tableBtn.removeAttribute('hx-include');
        }
    },
    applyProduct(item, detail) {
        if (!item || !detail) return;
        const id = parseInt(detail.value, 10) || 0;
        item.product_id = id;
        item.product_label = detail.display ? String(detail.display) : '';
    },
    onFkeySelect(detail) {
        if (!detail) return;
        const n = String(detail.name || '');
        for (const item of this.items) {
            if (n === 'product-' + item.id) {
                this.applyProduct(item, detail);
                return;
            }
        }
    },
    clearLineFkeyItem(fieldName) {
        if (!fieldName) return;
        const dash = String(fieldName).lastIndexOf('-');
        if (dash < 0) return;
        const field = String(fieldName).slice(0, dash);
        const itemId = parseInt(String(fieldName).slice(dash + 1), 10);
        if (!itemId || field !== 'product') return;
        const it = (this.items || []).find((row) => row.id === itemId);
        if (it) this.applyProduct(it, { value: '', display: '' });
    },
    bindFkeyInput(el, item) {
        const root = this.fkeyRoot(el);
        const d = this.fkeyData(el);
        if (!root || !d) {
            const n = Number(el.dataset.fkeyTries || 0);
            if (n > 40) return;
            el.dataset.fkeyTries = String(n + 1);
            this.$nextTick(() => this.bindFkeyInput(el, item));
            return;
        }
        if (!d._deliveryFkeyDom && typeof d.applySelect !== 'function') {
            const n = Number(el.dataset.fkeyTries || 0);
            if (n > 40) return;
            el.dataset.fkeyTries = String(n + 1);
            this.$nextTick(() => this.bindFkeyInput(el, item));
            return;
        }
        el.dataset.fkeyTries = '0';
        const slot = 'product-' + item.id;
        const idVal = item.product_id;
        const label = (idVal && Number(idVal) > 0) ? (item.product_label || '') : '';
        const uid = 'fk-dropdown-' + slot;
        const search = root.querySelector('input[type="search"]');
        const results = root.querySelector('.fk-picker-results');
        const tableBtn = root.querySelector('button[aria-label="Open selection table"]');
        const setTarget = (node) => {
            if (!node) return;
            const raw = node.getAttribute('hx-get') || '';
            try {
                const u = new URL(raw, window.location.href);
                u.searchParams.set('target_input', slot);
                node.setAttribute('hx-get', u.pathname + u.search + u.hash);
            } catch (e) {}
        };
        const self = this;
        if (!d._deliveryFkeyClearWrapped) {
            d._deliveryFkeyClearWrapped = true;
            const origClear = d.clear.bind(d);
            d.clear = function() {
                origClear.call(this);
                self.clearLineFkeyItem(this.fieldName);
            };
        }
        if (!d._deliveryFkeyDom) {
            d._deliveryFkeyDom = true;
            if (search) {
                search.id = uid + '-q';
                search.setAttribute('hx-target', '#' + uid);
                search.setAttribute('aria-controls', uid);
            }
            if (results) results.id = uid;
            const itemId = item.id;
            const syncItem = function(detail) {
                if (!detail || String(detail.name) !== String(this.fieldName)) return;
                this.value = detail.value != null ? String(detail.value) : '';
                this.display = detail.display ? String(detail.display) : '';
                this.query = this.display;
                this.open = false;
                this.pendingCreate = false;
                const it = (self.items || []).find((row) => row.id === itemId);
                if (it) self.applyProduct(it, { value: this.value, display: this.display });
            };
            if (typeof d.applySelect === 'function') {
                const orig = d.applySelect;
                d.applySelect = function(detail) {
                    orig.call(this, detail);
                    syncItem.call(this, detail);
                };
            } else {
                d.applySelect = syncItem;
            }
        }
        this.associatePickerForm(search, tableBtn, uid + '-form');
        if (search) {
            setTarget(search);
            if (window.htmx) window.htmx.process(search);
        }
        if (tableBtn) {
            setTarget(tableBtn);
            if (window.htmx) window.htmx.process(tableBtn);
        }
        d.fieldName = slot;
        const nextValue = idVal && Number(idVal) > 0 ? String(idVal) : '';
        if (String(d.value || '') !== nextValue) d.value = nextValue;
        if (!nextValue) {
            d.display = '';
            d.query = '';
            if (search) search.value = '';
        } else if (String(d.display || '') !== label) {
            d.display = label;
            if (!d.open) d.query = label;
        }
    },
    rewireFkeyPickers() {
        this.$el.querySelectorAll('[data-fkey-field]').forEach((el) => {
            const itemId = parseInt(el.dataset.fkeyItemId, 10);
            const item = (this.items || []).find((it) => it.id === itemId);
            if (item) this.bindFkeyInput(el, item);
        });
    },
    syncLineItemsFromFkeyPickers() {
        this.$el.querySelectorAll('[data-fkey-field]').forEach((el) => {
            const itemId = parseInt(el.dataset.fkeyItemId, 10);
            const item = (this.items || []).find((it) => it.id === itemId);
            if (!item) return;
            const d = this.fkeyData(el);
            if (!d) return;
            const id = parseInt(d.value, 10) || 0;
            if (id <= 0) return;
            const display = d.display ? String(d.display) : (d.query ? String(d.query) : '');
            if ((parseInt(item.product_id, 10) || 0) === id && item.product_label) return;
            this.applyProduct(item, { value: String(id), display });
        });
    },
    addItem() {
        const n = this.items.length + 1;
        this.items.push({
            id: this.nextId++,
            sr_no: n,
            product_id: 0,
            product_label: '',
            qty_kind: 'quantity',
            qty_mm: '',
            qty_unit: 'mm',
            qty_weight: '',
            qty_number: ''
        });
    },
    removeItem(idx) {
        this.items.splice(idx, 1);
    },
    jsonOutput() {
        this.syncQtyFromDom();
        return JSON.stringify((this.items || []).map((it, i) => ({
            sr_no: parseInt(it.sr_no, 10) || (i + 1),
            product_id: parseInt(it.product_id, 10) || 0,
            product_label: it.product_label || '',
            qty_kind: it.qty_kind || 'quantity',
            qty_mm: it.qty_mm == null ? '' : String(it.qty_mm),
            qty_unit: it.qty_unit || 'mm',
            qty_weight: it.qty_weight == null ? '' : String(it.qty_weight),
            qty_number: it.qty_number == null ? '' : String(it.qty_number)
        })));
    }
}
"#;

fn line_rows_json(raw: &str) -> (String, usize) {
    let value: serde_json::Value =
        serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!([]));
    let Some(arr) = value.as_array() else {
        return ("[]".into(), 1);
    };
    let mut rows = Vec::new();
    for (i, v) in arr.iter().enumerate() {
        let obj = v.as_object();
        let get_str = |key: &str| {
            obj.and_then(|o| o.get(key))
                .map(|x| match x {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => String::new(),
                })
                .unwrap_or_default()
        };
        let product_id = get_str("product_id").parse::<i64>().unwrap_or(0);
        let sr = get_str("sr_no").parse::<i32>().unwrap_or((i as i32) + 1);
        let kind = {
            let k = get_str("qty_kind");
            if k.is_empty() { "quantity".into() } else { k }
        };
        let unit = {
            let u = get_str("qty_unit");
            if u.is_empty() { "mm".into() } else { u }
        };
        rows.push(serde_json::json!({
            "id": i + 1,
            "sr_no": sr,
            "product_id": product_id,
            "product_label": get_str("product_label"),
            "qty_kind": kind,
            "qty_mm": get_str("qty_mm"),
            "qty_unit": unit,
            "qty_weight": get_str("qty_weight"),
            "qty_number": get_str("qty_number"),
        }));
    }
    let next = rows.len() + 1;
    (
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into()),
        next,
    )
}

pub struct LinesWidget;

impl FormWidget for LinesWidget {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let (rows_json, next_id) = line_rows_json(field.value);
        let alpine = ALPINE_LINES
            .replace("__ROWS__", &rows_json)
            .replace("__NEXT__", &next_id.to_string());
        let product_pick_url =
            lariv_rs::plugins::finance_products::routes::ProductFkSelectRouteTag.url();

        html! {
            div class="form-control mb-4 w-full min-w-0" data-lines-root="" data-lines-seed=(field.value)
                x-data=(alpine)
                x-effect="items.length; $nextTick(() => { if (window.htmx) window.htmx.process($el); if (typeof this.rewireFkeyPickers === 'function') this.rewireFkeyPickers(); })" {
                (PreEscaped(r#"<div hidden x-on:fk-select.window="onFkeySelect($event.detail)"></div>"#))
                input type="hidden" name=(field.name) data-lines-hidden value=(field.value);

                (label(field.label, html! {
                    (PreEscaped(r#"<div x-show="items.length === 0" class="p-4 text-center text-xs text-base-content/60 bg-base-100 rounded-lg border border-dashed border-base-300">"#))
                        "No lines yet. Click \"Add Line\" to add a product."
                    (PreEscaped("</div>"))

                    (PreEscaped(r#"<div x-show="items.length > 0" class="relative z-0 overflow-visible min-w-0 w-full border border-base-300 rounded-lg bg-base-100 shadow-sm">"#))
                        table class="table table-xs w-full min-w-max" {
                            thead class="bg-base-200/80 text-base-content/70" {
                                tr class="text-xs" {
                                    th class="w-16" { "Sr. No." }
                                    th class="min-w-[12rem]" { "Product" }
                                    th class="w-28" { "Qty kind" }
                                    th class="min-w-[12rem]" { "Qty" }
                                    th class="w-7" {}
                                }
                            }
                            tbody {
                                (PreEscaped(r#"<template x-for="(item, idx) in items" :key="item.id">"#))
                                tr class="hover border-b border-base-200 last:border-none" {
                                    td class="align-middle" {
                                        (PreEscaped(r#"<input type="number" min="1" step="1" x-model="item.sr_no" class="input input-xs input-bordered w-16 text-right font-mono h-7 min-h-0">"#))
                                    }
                                    td class="align-middle min-w-[12rem] overflow-visible" {
                                        div class="min-w-0" data-fkey-field="product" x-bind:data-fkey-item-id="item.id"
                                            x-effect="bindFkeyInput($el, item)" {
                                            (embed_input_fkey(&product_pick_url, "Select product…"))
                                        }
                                    }
                                    td class="align-middle" {
                                        (PreEscaped(r#"
                                        <select x-model="item.qty_kind" class="select select-xs select-bordered w-full h-7 min-h-0">
                                            <option value="length">Length</option>
                                            <option value="weight">Weight</option>
                                            <option value="quantity">Number</option>
                                        </select>
                                        "#))
                                    }
                                    td class="align-middle" {
                                        div class="min-w-0" x-show="item.qty_kind === 'length'" x-cloak
                                            data-qty-length="" x-bind:data-line-item-id="item.id"
                                            x-init="bindLengthInput($el, item)"
                                            x-effect="item.qty_kind === 'length' && bindLengthInput($el, item)"
                                            x-on:input="pullLengthInput($el, item)"
                                            x-on:change="pullLengthInput($el, item)" {
                                            (embed_input_length())
                                        }
                                        (PreEscaped(r#"
                                            <input x-show="item.qty_kind === 'weight'" x-cloak type="number" min="0" step="any"
                                                   x-model="item.qty_weight" placeholder="kg"
                                                   class="input input-xs input-bordered w-28 text-right font-mono h-7 min-h-0">
                                            <input x-show="item.qty_kind === 'quantity'" x-cloak type="number" min="0" step="1"
                                                   x-model="item.qty_number" placeholder="qty"
                                                   class="input input-xs input-bordered w-24 text-right font-mono h-7 min-h-0">
                                        "#))
                                    }
                                    td class="align-middle text-center" {
                                        (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-xs text-error h-7 w-7 min-h-0 p-0 flex items-center justify-center mx-auto" title="Remove line" @click="removeItem(idx)">✕</button>"#))
                                    }
                                }
                                (PreEscaped("</template>"))
                            }
                        }
                    (PreEscaped("</div>"))

                    div class="flex justify-end mt-2" {
                        (PreEscaped(r#"<button type="button" class="btn btn-outline btn-xs btn-primary gap-1" @click="addItem()">"#))
                        (icon("plus", "w-3 h-3"))
                        "Add Line"
                        (PreEscaped("</button>"))
                    }
                }))
            }
        }
    }
}

/// Optional text input that honours placeholder and hint.
pub struct OptionalText;

impl FormWidget for OptionalText {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let placeholder = field.spec.placeholder.unwrap_or("");
        let input = PreEscaped(format!(
            r#"<input type="text" name="{}" value="{}" placeholder="{}" class="input input-bordered w-full">"#,
            escape_attr(field.name),
            escape_attr(field.value),
            escape_attr(placeholder),
        ));
        if field.label.is_empty() {
            html! { (input) }
        } else {
            lariv_rs::components::label_hint(
                field.label,
                ctx.hint_of(field.spec),
                html! { (input) },
            )
        }
    }
}

pub struct DateInput;

impl FormWidget for DateInput {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let required = if field.required { " required" } else { "" };
        let input = PreEscaped(format!(
            r#"<input type="date" name="{}" value="{}" class="input input-bordered w-full"{}>"#,
            field.name, field.value, required
        ));
        if field.label.is_empty() {
            html! { (input) }
        } else {
            label(field.label, html! { (input) })
        }
    }
}

/// Customer picker kept above the lines table so its menu is not covered by that box.
pub struct CustomerPicker;

impl FormWidget for CustomerPicker {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let display_key = field.spec.display_key.unwrap_or(field.name);
        let ph = field.spec.placeholder.unwrap_or("Select customer…");
        html! {
            div class="relative z-50" {
                (input_foreign_key(InputForeignKey {
                    label: field.label,
                    name: field.name,
                    value: field.value,
                    display: ctx.display_of(display_key),
                    placeholder: ph,
                    url: ctx.url_of(field.spec),
                    uid: field.spec.swap_key.unwrap_or(""),
                    required: field.required,
                    classes: "!my-0",
                    ..Default::default()
                }))
            }
        }
    }
}

#[html_form]
pub struct DeliveryChallanForm {
    #[form(
        label = "Customer",
        required,
        widget = CustomerPicker,
        name = "customer_id",
        route = lariv_rs::plugins::customer::routes::CustomerFkSelectRouteTag,
        swap_key = "fk-delivery-customer",
        display = "customer",
        placeholder = "Select customer…"
    )]
    pub customer_id: i64,

    #[form(
        label = "Challan Number",
        widget = OptionalText,
        name = "challan_number",
        placeholder = "Leave blank to auto-generate",
        hint = "Leave blank to assign a number from the Delivery preferences format."
    )]
    pub challan_number: String,

    #[form(label = "Date", required, widget = DateInput, name = "date")]
    pub date: String,

    #[form(label = "Vehicle No.", widget = OptionalText, name = "vehicle_no")]
    pub vehicle_no: String,

    #[form(label = "E-Way Bill", widget = OptionalText, name = "eway_bill")]
    pub eway_bill: String,

    #[form(label = "Lines", widget = LinesWidget, name = "lines")]
    pub lines: String,
}

#[html_form(default)]
pub struct DeliveryPreferencesForm {
    #[form(widget = Section, label = "Challan numbering")]
    _section_number: (),

    #[form(
        label = "Challan number format",
        widget = Text,
        name = "challan_number_format",
        placeholder = "DC-{{YYYY}}-{{POSTED_SEQ}}",
        hint = CHALLAN_NUMBER_FORMAT_HINT
    )]
    pub challan_number_format: String,

    #[form(widget = Section, label = "Company")]
    _section_company: (),

    #[form(label = "Company name", widget = Text, name = "company_name")]
    pub company_name: String,

    #[form(label = "Address", widget = Textarea, rows = 4, name = "company_address")]
    pub company_address: String,

    #[form(label = "Phone number", widget = Text, name = "company_phone")]
    pub company_phone: String,

    #[form(label = "Email", widget = Text, name = "company_email")]
    pub company_email: String,

    #[form(label = "GSTIN", widget = Text, name = "company_gstin")]
    pub company_gstin: String,

    #[form(
        label = "Terms and conditions",
        widget = Textarea,
        rows = 6,
        name = "terms_and_conditions",
        hint = "Printed on delivery challan PDFs. Leave blank to omit."
    )]
    pub terms_and_conditions: String,

    #[form(
        label = "Logo",
        widget = ForeignKey,
        name = "logo_vnode_id",
        route = VNodeFileSelectRouteTag,
        swap_key = "delivery-pref-logo-vnode",
        display = "logo_vnode",
        placeholder = "Select logo file…"
    )]
    pub logo_vnode_id: String,

    #[form(
        label = "Signature",
        widget = ForeignKey,
        name = "signature_vnode_id",
        route = VNodeFileSelectRouteTag,
        swap_key = "delivery-pref-signature-vnode",
        display = "signature_vnode",
        placeholder = "Select signature file…"
    )]
    pub signature_vnode_id: String,

    #[form(widget = Section, label = "Delivery challan PDF")]
    _section_pdf: (),

    #[form(
        label = "Delivery Challan Template (Typst)",
        widget = CodeEditor,
        name = "delivery_challan_pdf_template",
        language = "typst",
        rows = 24
    )]
    pub delivery_challan_pdf_template: String,
}
