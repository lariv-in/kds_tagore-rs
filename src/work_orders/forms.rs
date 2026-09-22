#[allow(unused_imports)]
use lariv_rs::html_form::widgets::{ForeignKey, ManyToMany};
use lariv_rs::{
    components::{
        InputForeignKey, InputLength,
        attrs::{HtmlAttrs, escape_attr},
        icon, input_foreign_key, input_length, label, label_hint,
        swap::SwapKey,
    },
    html_form::{
        FieldRender, FormCtx, FormFieldKey, FormWidget, html_form,
        widgets::{CodeEditor, Duration, List, Section, Text, Textarea},
    },
    plugins::finance_taxes::routes::TaxMultiSelectRouteTag,
};
use maud::{Markup, PreEscaped, html};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Embeddable length control (no form `name`) for Alpine lists.
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

/// Embeddable FK picker (no form `name`) for Alpine lists.
#[derive(Copy, Clone)]
pub enum LineEditorDisplayKey {
    TaxesData,
}

impl FormFieldKey for LineEditorDisplayKey {
    fn html_name(self) -> &'static str {
        "taxes_data"
    }
}

fn line_tax_catalog(ctx: &FormCtx<'_>) -> (String, String, String, String, String) {
    #[derive(serde::Deserialize, Default)]
    struct Catalog {
        #[serde(default)]
        tax_pct_by_id: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        tax_kind_by_id: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        all_taxes: Vec<serde_json::Value>,
        #[serde(default)]
        default_material_taxes: Vec<serde_json::Value>,
        #[serde(default)]
        default_machine_taxes: Vec<serde_json::Value>,
    }
    let parsed: Catalog = serde_json::from_str(ctx.display_of("taxes_data")).unwrap_or_default();
    let pct = serde_json::to_string(&parsed.tax_pct_by_id).unwrap_or_else(|_| "{}".into());
    let kind = serde_json::to_string(&parsed.tax_kind_by_id).unwrap_or_else(|_| "{}".into());
    let all = serde_json::to_string(&parsed.all_taxes).unwrap_or_else(|_| "[]".into());
    let default_material =
        serde_json::to_string(&parsed.default_material_taxes).unwrap_or_else(|_| "[]".into());
    let default_machine =
        serde_json::to_string(&parsed.default_machine_taxes).unwrap_or_else(|_| "[]".into());
    (pct, kind, all, default_material, default_machine)
}

fn parse_tax_ids_from_obj(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<i64> {
    crate::work_orders::tax_assoc::parse_tax_ids_json(
        obj.get("tax_ids").or_else(|| obj.get("TaxIds")),
    )
}

fn embed_line_tax_picker() -> Markup {
    use lariv_rs::components::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL};
    html! {
        td class="align-middle min-w-[10rem] max-w-xs overflow-visible" {
            div class="my-1" {
                (PreEscaped(format!(
                    r#"<div class="input input-bordered input-xs min-h-7 h-auto w-full whitespace-normal overflow-visible flex flex-wrap items-center gap-1 cursor-pointer py-0.5 px-1.5" :class="(item.line_taxes && item.line_taxes.length) ? '' : 'opacity-50'" x-bind:hx-get="lineTaxPickHref(item)" hx-target="{}" hx-swap="{}" hx-push-url="false" @click="syncLineTaxStore(item)">"#,
                    HTMX_TARGET_BODY_MODAL,
                    HTMX_SWAP_BODY_MODAL
                )))
                span class="text-xs" x-show="!item.line_taxes || item.line_taxes.length === 0" { "Select taxes…" }
                template x-for="ltItem in (item.line_taxes || [])" x-bind:key="ltItem.Key" {
                    (PreEscaped(r#"<div class="flex items-center gap-1 rounded bg-base-200 pl-1.5 pr-0.5 py-0 max-w-full" @click="$event.stopPropagation()">"#))
                    span class="text-[10px] truncate max-w-[6rem]" x-text="ltItem.Value" {}
                    (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-square btn-xs shrink-0 h-5 w-5 min-h-0 p-0" @click.stop="removeLineTax(item, ltItem.Key)" aria-label="Remove tax">"#))
                    (icon("x-mark", "w-3 h-3"))
                    (PreEscaped("</button></div>"))
                }
                (PreEscaped("</div>"))
            }
        }
    }
}

fn embed_input_fkey(url: &'static str, placeholder: &'static str) -> Markup {
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

const ALPINE_LENGTH_BRIDGE: &str = r#"
                lengthData(el) {
                    const root = el.querySelector('[x-data]');
                    return root && window.Alpine ? Alpine.$data(root) : null;
                },
                bindLengthInput(el, target, key) {
                    this.$nextTick(() => {
                        const d = this.lengthData(el);
                        if (!d) return;
                        let mm;
                        if (key) {
                            mm = target.variables ? target.variables[key] : '';
                            d.unit = (target.dim_units && target.dim_units[key]) || 'mm';
                        } else {
                            mm = target.value;
                        }
                        d.mm = (mm !== '' && mm !== null && mm !== undefined) ? String(mm) : '';
                        if (typeof d.mmToDisplay === 'function') d.mmToDisplay();
                    });
                },
                pullLengthInput(el, target, key) {
                    const d = this.lengthData(el);
                    if (!d) return;
                    if (key) {
                        if (!target.variables) target.variables = {};
                        if (!target.dim_units) target.dim_units = {};
                        if (!target.dim_inputs) target.dim_inputs = {};
                        target.variables[key] = d.mm;
                        target.dim_units[key] = d.unit || 'mm';
                        target.dim_inputs[key] = d.display;
                        if (typeof this.recalc === 'function') this.recalc(target);
                    } else {
                        target.value = d.mm;
                    }
                },
"#;

/// Sync Alpine line widgets into the HTMX request body before every form POST.
/// Declared on the form element so it survives `outerMorph` (widget `init()` listeners do not).
pub const SYNC_LINES_HTMX_CONFIG_REQUEST: &str = r#"var ctx=event.detail&&(event.detail.ctx||event.detail);var body=ctx&&ctx.request&&ctx.request.body;if(!body||typeof body.set!=='function')return;document.querySelectorAll('[data-lines-root]').forEach(function(el){if(!window.Alpine)return;var d=Alpine.$data(el);if(!d||typeof d.jsonOutput!=='function')return;if(typeof d.syncLineItemsFromFkeyPickers==='function')d.syncLineItemsFromFkeyPickers();if(typeof d.syncLineItemsFromDomInputs==='function')d.syncLineItemsFromDomInputs();var h=el.querySelector('[data-lines-hidden]');if(!h||!h.name)return;var json='[]';try{json=d.jsonOutput()}catch(e){}h.value=json;body.set(h.name,json)})"#;

pub fn lines_form_hx_post<K: SwapKey>(url: &str) -> HtmlAttrs {
    lariv_rs::components::swap::form_hx_post_url::<K>(url)
        .set("hx-on::config:request", SYNC_LINES_HTMX_CONFIG_REQUEST)
        .set("hx-on:htmx:config:request", SYNC_LINES_HTMX_CONFIG_REQUEST)
}

pub fn draft_work_order_form_hx_post<K: SwapKey>(url: &str) -> HtmlAttrs {
    lines_form_hx_post::<K>(url)
}

const ALPINE_LINES_SUBMIT_BRIDGE: &str = r#"
                linesHidden: '[]',
                syncHiddenOutput() {
                    if (typeof this.syncLineItemsFromFkeyPickers === 'function') {
                        this.syncLineItemsFromFkeyPickers();
                    }
                    if (typeof this.syncLineItemsFromDomInputs === 'function') {
                        this.syncLineItemsFromDomInputs();
                    }
                    const h = this.$el.querySelector('[data-lines-hidden], [data-kv-hidden]');
                    const fieldName = this.linesFieldName || (h && h.name) || '';
                    let json = '[]';
                    try {
                        json = this.jsonOutput();
                    } catch (e) {
                        json = '[]';
                    }
                    this.linesHidden = json;
                    if (h) h.value = json;
                    return { fieldName, json };
                },
                pushLinesToHtmxBody(ev) {
                    const { fieldName, json } = this.syncHiddenOutput();
                    if (!fieldName || !json) return;
                    const ctx = ev.detail && (ev.detail.ctx || ev.detail);
                    const body = ctx && ctx.request && ctx.request.body;
                    if (body && typeof body.set === 'function') {
                        body.set(fieldName, json);
                    }
                },
                bindLineFormSubmit() {
                    if (this._linesFormWired) return;
                    this._linesFormWired = true;
                    const h = this.$el.querySelector('[data-lines-hidden], [data-kv-hidden]');
                    this.linesFieldName = h && h.name ? h.name : '';
                    const form = this.$el.closest('form');
                    if (!form) return;
                    const sync = () => { this.syncHiddenOutput(); };
                    const push = (ev) => { this.pushLinesToHtmxBody(ev); };
                    for (const evtName of ['htmx:config:request', 'htmx:before:request']) {
                        form.addEventListener(evtName, push);
                    }
                    form.addEventListener('submit', sync, true);
                    if (typeof this.$watch === 'function') {
                        this.$watch('items', sync, { deep: true });
                    }
                    sync();
                },
                hookLinesMorphRestore() {
                    const form = this.$el.closest('form');
                    if (!form || form.dataset.linesMorphRestoreHooked === '1') return;
                    form.dataset.linesMorphRestoreHooked = '1';
                    form.addEventListener('htmx:afterSettle', () => {
                        form.querySelectorAll('[data-lines-root]').forEach(el => {
                            if (!window.Alpine) return;
                            const d = Alpine.$data(el);
                            if (!d || typeof d.restoreItemsFromHidden !== 'function') return;
                            d.$nextTick(() => {
                                if (typeof d.seedHiddenFromServer === 'function') {
                                    d.seedHiddenFromServer();
                                }
                                d.restoreItemsFromHidden();
                            });
                        });
                    });
                },
                linesSeedRaw() {
                    const seed = (this.$el.dataset.linesSeed || '').trim();
                    if (seed && seed !== '[]') return seed;
                    const h = this.$el.querySelector('[data-lines-hidden]');
                    if (h) {
                        const attr = (h.getAttribute('value') || '').trim();
                        if (attr && attr !== '[]') return attr;
                    }
                    return '';
                },
                seedHiddenFromServer() {
                    const raw = this.linesSeedRaw();
                    if (!raw) return;
                    const h = this.$el.querySelector('[data-lines-hidden]');
                    if (h) h.value = raw;
                },
                finalizeLinesRestore() {
                    if (typeof this.syncLineItemsFromFkeyPickers === 'function') {
                        this.syncLineItemsFromFkeyPickers();
                    }
                    if (typeof this.syncLineItemsFromDomInputs === 'function') {
                        this.syncLineItemsFromDomInputs();
                    }
                    if (typeof this.rewireFkeyPickers === 'function') {
                        this.rewireFkeyPickers();
                    }
                    if (typeof this.syncHiddenOutput === 'function') {
                        this.syncHiddenOutput();
                    }
                    if (typeof this.syncAllLineTaxStores === 'function') {
                        this.syncAllLineTaxStores();
                    }
                },
                syncLineItemsFromDomInputs() {
                    this.$el.querySelectorAll('[data-line-item-id]').forEach((row) => {
                        const itemId = parseInt(row.getAttribute('data-line-item-id'), 10);
                        if (!itemId) return;
                        const item = (this.items || []).find((it) => it.id === itemId);
                        if (!item) return;
                        row.querySelectorAll('[data-line-field]').forEach((input) => {
                            const field = input.dataset.lineField;
                            if (!field || !Object.prototype.hasOwnProperty.call(item, field)) return;
                            item[field] = input.value;
                        });
                    });
                },
"#;

const ALPINE_LINE_TAX_BRIDGE: &str = r#"
                taxKindForId(id) {
                    const k = this.tax_kind_by_id && this.tax_kind_by_id[id];
                    return k === 'withholding' ? 'withholding' : 'levied';
                },
                lineTaxAmountForKind(item, kind) {
                    const base = this.lineUntaxedNumber(item);
                    if (!Array.isArray(item.line_taxes) || item.line_taxes.length === 0) return 0;
                    let sum = 0;
                    for (const t of item.line_taxes) {
                        const id = String(t.Key);
                        if (this.taxKindForId(id) !== kind) continue;
                        const pctStr = this.tax_pct_by_id ? this.tax_pct_by_id[id] : null;
                        const pct = pctStr != null && pctStr !== '' ? parseFloat(String(pctStr)) : NaN;
                        if (!isNaN(pct)) sum += base * (pct / 100);
                    }
                    return sum;
                },
                lineTaxedTotal(item) {
                    const u = this.lineUntaxedNumber(item);
                    return u + this.lineTaxAmountForKind(item, 'levied') - this.lineTaxAmountForKind(item, 'withholding');
                },
                hydrateLineTaxes(item) {
                    if (!Array.isArray(item.line_taxes)) item.line_taxes = [];
                    const ids = item.tax_ids;
                    if (Array.isArray(ids) && ids.length > 0 && item.line_taxes.length === 0 && Array.isArray(this.all_taxes)) {
                        for (const tid of ids) {
                            const t = this.all_taxes.find(x => Number(x.id) === Number(tid));
                            if (t) item.line_taxes.push({ Key: String(t.id), Value: t.name });
                        }
                    }
                    delete item.tax_ids;
                    this.syncLineTaxStore(item);
                },
                lineTaxPickHref(item) {
                    const b = this.tax_pick_base || '/finance-taxes/multi-select';
                    const name = this.lineTaxStoreName(item);
                    const sep = b.indexOf('?') >= 0 ? '&' : '?';
                    return b + sep + 'target_input=' + encodeURIComponent(name);
                },
                onTaxMultiSelect(detail) {
                    if (!detail) return;
                    const n = String(detail.name || '');
                    for (const item of this.items) {
                        const expected = this.lineTaxStoreName(item);
                        if (expected !== n) continue;
                        const value = String(detail.value);
                        const items = item.line_taxes || (item.line_taxes = []);
                        const idx = items.findIndex(x => x.Key === value);
                        if (idx >= 0) items.splice(idx, 1);
                        else items.push({ Key: value, Value: String(detail.display || value) });
                        this.syncLineTaxStore(item);
                        if (typeof this.recalc === 'function') this.recalc(item);
                        break;
                    }
                },
                taxIdsOf(item) {
                    return (item.line_taxes || []).map(t => parseInt(String(t.Key), 10)).filter(id => !isNaN(id) && id > 0);
                },
                lineTaxStoreName(item) {
                    return 'LineTaxes_' + String(item.id);
                },
                ensureM2mStore() {
                    if (typeof Alpine === 'undefined' || !Alpine.store) return;
                    if (!Alpine.store('m2mSelections')) Alpine.store('m2mSelections', {});
                },
                syncLineTaxStore(item) {
                    this.ensureM2mStore();
                    if (typeof Alpine === 'undefined' || !Alpine.store) return;
                    Alpine.store('m2mSelections')[this.lineTaxStoreName(item)] = (item.line_taxes || []).slice();
                },
                syncAllLineTaxStores() {
                    for (const it of this.items || []) this.syncLineTaxStore(it);
                },
                removeLineTax(item, key) {
                    item.line_taxes = (item.line_taxes || []).filter(it => it.Key !== key);
                    this.syncLineTaxStore(item);
                    if (typeof this.recalc === 'function') this.recalc(item);
                },
                defaultLineTaxes(kind) {
                    const src = kind === 'machine'
                        ? (this.default_machine_taxes || [])
                        : (this.default_material_taxes || []);
                    return src.map(t => ({
                        Key: String(t.id),
                        Value: String(t.name || t.id)
                    })).filter(t => t.Key && t.Key !== '0');
                },
"#;

const ALPINE_FKEY_BRIDGE: &str = r#"
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
                applyFkeyToItem(item, field, detail) {
                    if (!item || !detail) return;
                    const id = parseInt(detail.value, 10) || 0;
                    const display = detail.display ? String(detail.display) : '';
                    if (field === 'component') {
                        item.component_id = id;
                        item.component_label = display;
                        this.ingestPickedSchema(detail, 'component', id, display);
                        if (typeof this.onCompChange === 'function') this.onCompChange(item);
                        if (typeof this.recalc === 'function') this.recalc(item);
                    } else if (field === 'machine') {
                        item.machine_id = id;
                        item.machine_label = display;
                        if (id <= 0) {
                            item.machine_label = '';
                            if ('name' in item) item.name = '';
                        } else if (item.name !== undefined) {
                            item.name = display || item.name;
                        }
                        this.ingestPickedSchema(detail, 'machine', id, display);
                        if (typeof this.onMachineChange === 'function') this.onMachineChange(item);
                        if (typeof this.recalc === 'function') this.recalc(item);
                    }
                },
                ingestPickedSchema(detail, kind, id, display) {
                    if (!id) return;
                    let vars = detail.variables;
                    if (typeof vars === 'string') {
                        try { vars = JSON.parse(vars); } catch (e) { vars = null; }
                    }
                    if (kind === 'component') {
                        if (!Array.isArray(this.components)) this.components = [];
                        let c = this.components.find(x => String(x.id) === String(id));
                        if (!c) {
                            c = { id, name: display, variables: vars || {}, cost_formula: detail.cost_formula || '', weight_formula: detail.weight_formula || '' };
                            this.components.push(c);
                        } else {
                            if (display) c.name = display;
                            if (vars && typeof vars === 'object') c.variables = vars;
                            if (detail.cost_formula) c.cost_formula = detail.cost_formula;
                            if (detail.weight_formula) c.weight_formula = detail.weight_formula;
                        }
                    } else {
                        if (!Array.isArray(this.machines)) this.machines = [];
                        let m = this.machines.find(x => String(x.id) === String(id));
                        if (!m) {
                            m = { id, name: display, variables: vars || {}, cost_formula: detail.cost_formula || '' };
                            this.machines.push(m);
                        } else {
                            if (display) m.name = display;
                            if (vars && typeof vars === 'object') m.variables = vars;
                            if (detail.cost_formula) m.cost_formula = detail.cost_formula;
                        }
                    }
                },
                onFkeySelect(detail) {
                    if (!detail) return;
                    const n = String(detail.name || '');
                    for (const item of this.items) {
                        if (n === 'component-' + item.id) {
                            this.applyFkeyToItem(item, 'component', detail);
                            return;
                        }
                        if (n === 'machine-' + item.id) {
                            this.applyFkeyToItem(item, 'machine', detail);
                            return;
                        }
                    }
                },
                rewireFkeyPickers() {
                    this.$el.querySelectorAll('[data-fkey-field]').forEach((el) => {
                        const field = el.dataset.fkeyField;
                        const itemId = parseInt(el.dataset.fkeyItemId, 10);
                        const item = (this.items || []).find((it) => it.id === itemId);
                        if (item && field) this.bindFkeyInput(el, item, field);
                    });
                },
                syncLineItemsFromFkeyPickers() {
                    this.$el.querySelectorAll('[data-fkey-field]').forEach((el) => {
                        const field = el.dataset.fkeyField;
                        const itemId = parseInt(el.dataset.fkeyItemId, 10);
                        const item = (this.items || []).find((it) => it.id === itemId);
                        if (!item || !field) return;
                        const d = this.fkeyData(el);
                        if (!d) return;
                        const id = parseInt(d.value, 10) || 0;
                        if (id <= 0) return;
                        const idKey = field + '_id';
                        const labelKey = field + '_label';
                        const current = parseInt(item[idKey], 10) || 0;
                        const display = d.display ? String(d.display) : (d.query ? String(d.query) : '');
                        if (current === id && item[labelKey]) return;
                        const detail = {
                            value: String(id),
                            display,
                            name: field + '-' + itemId,
                        };
                        if (field === 'machine') {
                            if (d.variables != null) detail.variables = d.variables;
                            if (d.cost_formula != null) detail.cost_formula = d.cost_formula;
                        } else if (field === 'component') {
                            if (d.variables != null) detail.variables = d.variables;
                            if (d.cost_formula != null) detail.cost_formula = d.cost_formula;
                            if (d.weight_formula != null) detail.weight_formula = d.weight_formula;
                        }
                        this.applyFkeyToItem(item, field, detail);
                    });
                },
                clearLineFkeyItem(fieldName) {
                    if (!fieldName) return;
                    const dash = String(fieldName).lastIndexOf('-');
                    if (dash < 0) return;
                    const field = String(fieldName).slice(0, dash);
                    const itemId = parseInt(String(fieldName).slice(dash + 1), 10);
                    if (!itemId || (field !== 'component' && field !== 'machine')) return;
                    const it = (this.items || []).find((row) => row.id === itemId);
                    if (!it) return;
                    this.applyFkeyToItem(it, field, { value: '', display: '', name: fieldName });
                    if (typeof this.recalc === 'function') this.recalc(it);
                },
                bindFkeyInput(el, item, field) {
                    const root = this.fkeyRoot(el);
                    const d = this.fkeyData(el);
                    if (!root || !d) {
                        const n = Number(el.dataset.fkeyTries || 0);
                        if (n > 40) return;
                        el.dataset.fkeyTries = String(n + 1);
                        this.$nextTick(() => this.bindFkeyInput(el, item, field));
                        return;
                    }
                    if (!d._woFkeyDom && typeof d.applySelect !== 'function') {
                        const n = Number(el.dataset.fkeyTries || 0);
                        if (n > 40) return;
                        el.dataset.fkeyTries = String(n + 1);
                        this.$nextTick(() => this.bindFkeyInput(el, item, field));
                        return;
                    }
                    el.dataset.fkeyTries = '0';
                    const slot = field + '-' + item.id;
                    const idKey = field + '_id';
                    const labelKey = field + '_label';
                    const idVal = item[idKey];
                    let label = '';
                    if (idVal && Number(idVal) > 0) {
                        label = item[labelKey] || '';
                        if (field === 'component') {
                            const c = typeof this.getComp === 'function' ? this.getComp(idVal) : null;
                            if (c && c.name) label = c.name;
                        } else if (field === 'machine' && typeof this.getMachine === 'function') {
                            const m = this.getMachine(idVal);
                            if (m && m.name) label = m.name;
                        }
                    }
                    if (item[labelKey] !== label) item[labelKey] = label;
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
                    if (!d._woFkeyClearWrapped) {
                        d._woFkeyClearWrapped = true;
                        const origClear = d.clear.bind(d);
                        d.clear = function() {
                            origClear.call(this);
                            self.clearLineFkeyItem(this.fieldName);
                        };
                    }
                    if (!d._woFkeyDom) {
                        d._woFkeyDom = true;
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
                            if (!it) return;
                            self.applyFkeyToItem(it, field, Object.assign({}, detail, {
                                value: this.value,
                                display: this.display,
                            }));
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
                    const nextDisplay = label || '';
                    if (String(d.value || '') !== nextValue) {
                        d.value = nextValue;
                    }
                    if (!nextValue) {
                        d.display = '';
                        d.query = '';
                        if (search) search.value = '';
                    } else if (String(d.display || '') !== nextDisplay) {
                        d.display = nextDisplay;
                        if (!d.open) d.query = nextDisplay;
                    }
                },
"#;

const ALPINE_CALC_BRIDGE: &str = r#"
                calcApi: '/work-orders/api/calculate',
                schemaEntries(schema) {
                    if (!schema || typeof schema !== 'object' || Array.isArray(schema)) return [];
                    return Object.keys(schema).sort().map(name => ({
                        name,
                        type: String(schema[name] || '').toLowerCase()
                    }));
                },
                initVarsFromSchema(item, schema) {
                    if (!item.variables || typeof item.variables !== 'object' || Array.isArray(item.variables)) {
                        item.variables = {};
                    }
                    if (!item.dim_units) item.dim_units = {};
                    const next = {};
                    for (const name of Object.keys(schema || {})) {
                        next[name] = (item.variables[name] != null && item.variables[name] !== undefined)
                            ? item.variables[name] : '';
                        if (String(schema[name]).toLowerCase() === 'length' && !item.dim_units[name]) {
                            item.dim_units[name] = 'mm';
                        }
                    }
                    item.variables = next;
                },
                parseVariables(raw) {
                    if (!raw) return {};
                    if (typeof raw === 'string') {
                        try { return JSON.parse(raw) || {}; } catch (e) { return {}; }
                    }
                    if (typeof raw === 'object' && !Array.isArray(raw)) return Object.assign({}, raw);
                    return {};
                },
                parseExtra(raw) {
                    if (!raw) return {};
                    if (typeof raw === 'string' && raw.trim()) {
                        try { return JSON.parse(raw) || {}; } catch (e) { return { raw }; }
                    }
                    if (typeof raw === 'object' && !Array.isArray(raw)) return Object.assign({}, raw);
                    return {};
                },
                scheduleRecalc(item) {
                    if (item._recalcTimer) clearTimeout(item._recalcTimer);
                    const self = this;
                    item._recalcTimer = setTimeout(() => self.recalc(item), 250);
                },
                postCalculate(kind, id, item) {
                    const seq = (item._calcSeq = (item._calcSeq || 0) + 1);
                    const parsedId = parseInt(id, 10) || 0;
                    if (!parsedId) {
                        item.final_cost = '';
                        item.weight = '';
                        item.calc_error = '';
                        item.untaxed = 0;
                        if (typeof this.lineTaxedTotal === 'function') item.total = 0;
                        return;
                    }
                    const extra = (typeof this.parseExtra === 'function')
                        ? this.parseExtra(item.extra_data)
                        : (item.extra_data && typeof item.extra_data === 'object' && !Array.isArray(item.extra_data)
                            ? Object.assign({}, item.extra_data) : {});
                    extra.dim_units = item.dim_units || extra.dim_units || {};
                    const body = {
                        variables: item.variables || {},
                        extra_data: extra
                    };
                    if (kind === 'machine') body.machine_id = parsedId;
                    else body.component_id = parsedId;
                    fetch(this.calcApi, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
                        body: JSON.stringify(body)
                    }).then(r => r.json()).then(data => {
                        if (seq !== item._calcSeq) return;
                        if (data && data.error) {
                            item.calc_error = String(data.error);
                            return;
                        }
                        item.calc_error = '';
                        const cost = (data && (data.cost != null ? data.cost : data.cost_inr));
                        if (cost != null && cost !== '') item.final_cost = String(cost);
                        const weight = (data && (data.weight != null ? data.weight : data.weight_kg));
                        if (weight != null && weight !== '') item.weight = String(weight);
                        item.untaxed = this.lineUntaxedNumber(item);
                        item.total = Math.round(this.lineTaxedTotal(item) * 100) / 100;
                    }).catch(err => {
                        if (seq !== item._calcSeq) return;
                        item.calc_error = String(err && err.message ? err.message : err);
                    });
                },
"#;

#[html_form]
pub struct ComponentForm {
    #[form(label = "Component Name", required, widget = Text)]
    pub name: String,

    #[form(
        label = "Variables",
        widget = List,
        placeholder = "name:type  e.g. length:length",
        hint = "One per row as name:type. Types: length, weight, duration, quantity."
    )]
    pub variables: Vec<String>,

    #[form(
        label = "Cost Formula",
        required,
        widget = Textarea,
        rows = 4,
        placeholder = "e.g. length * qty * decimal(\"0.085\")",
        hint = "Rune expression evaluated with the variables above. Length is mm, weight kg, duration seconds, quantity integer."
    )]
    pub cost_formula: String,

    #[form(
        label = "Weight Formula",
        required,
        widget = Textarea,
        rows = 3,
        placeholder = "e.g. length * qty * decimal(\"0.001\")",
        hint = "Rune expression for weight in kg."
    )]
    pub weight_formula: String,
}

pub type ComponentCreateForm = ComponentForm;
pub type ComponentEditForm = ComponentForm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMeta {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub cost_formula: String,
    #[serde(default)]
    pub weight_formula: String,
}

pub fn schema_entries_from_json(value: &serde_json::Value) -> Vec<String> {
    crate::formula::parse_schema(value)
        .map(|s| crate::formula::schema_to_entries(&s))
        .unwrap_or_default()
}

fn json_object_field(v: Option<&serde_json::Value>) -> serde_json::Value {
    match v {
        Some(serde_json::Value::String(s)) => {
            serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({}))
        }
        Some(serde_json::Value::Object(_)) => v.cloned().unwrap_or_else(|| serde_json::json!({})),
        None => serde_json::json!({}),
        _ => serde_json::json!({}),
    }
}

fn json_i64_id(v: Option<&serde_json::Value>) -> i64 {
    match v {
        Some(serde_json::Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|u| u as i64))
            .unwrap_or(0),
        Some(serde_json::Value::String(s)) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

fn extra_dim_units(extra: Option<&serde_json::Value>) -> serde_json::Value {
    extra
        .and_then(|x| match x {
            serde_json::Value::Object(m) => {
                if let Some(serde_json::Value::Object(units)) = m.get("dim_units") {
                    Some(serde_json::Value::Object(units.clone()))
                } else if let Some(serde_json::Value::String(u)) = m.get("dim_unit") {
                    let mut map = serde_json::Map::new();
                    map.insert("default".into(), serde_json::Value::String(u.clone()));
                    Some(serde_json::Value::Object(map))
                } else {
                    None
                }
            }
            serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(s)
                .ok()
                .and_then(|j| {
                    if let serde_json::Value::Object(ref m) = j {
                        if let Some(serde_json::Value::Object(units)) = m.get("dim_units") {
                            Some(serde_json::Value::Object(units.clone()))
                        } else if let Some(serde_json::Value::String(u)) = m.get("dim_unit") {
                            let mut map = serde_json::Map::new();
                            map.insert("default".into(), serde_json::Value::String(u.clone()));
                            Some(serde_json::Value::Object(map))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }),
            _ => None,
        })
        .unwrap_or_else(|| serde_json::json!({}))
}

pub struct MaterialLinesWidget;

impl FormWidget for MaterialLinesWidget {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let comp_str = ctx.display_of(field.name);
        let components_json = if comp_str.is_empty() {
            let alt = ctx.display_of("components_data");
            if alt.is_empty() { "[]" } else { alt }
        } else {
            comp_str
        };
        let component_metas: Vec<ComponentMeta> =
            serde_json::from_str(components_json).unwrap_or_default();
        let mut rows = Vec::new();
        if let Ok(serde_json::Value::Array(arr)) =
            serde_json::from_str::<serde_json::Value>(field.value)
        {
            for (i, v) in arr.into_iter().enumerate() {
                if let serde_json::Value::Object(obj) = v {
                    let component_id = json_i64_id(obj.get("component_id"));
                    let variables = json_object_field(obj.get("variables"));
                    let extra_data = obj
                        .get("extra_data")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    let dim_units = extra_dim_units(obj.get("extra_data"));
                    let db_id = json_i64_id(obj.get("id"));
                    let final_cost = obj
                        .get("final_cost")
                        .cloned()
                        .unwrap_or(serde_json::json!(""));
                    let component_label = component_metas
                        .iter()
                        .find(|c| c.id == component_id)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    rows.push(serde_json::json!({
                        "id": i + 1,
                        "db_id": db_id,
                        "component_id": component_id,
                        "component_label": component_label,
                        "variables": variables,
                        "dim_units": dim_units,
                        "final_cost": final_cost,
                        "extra_data": extra_data,
                        "tax_ids": parse_tax_ids_from_obj(&obj),
                        "line_taxes": [],
                    }));
                }
            }
        }
        let next_id = rows.len() + 1;
        let rows_json = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());
        let (
            tax_pct_json,
            tax_kind_json,
            all_taxes_json,
            default_material_json,
            default_machine_json,
        ) = line_tax_catalog(ctx);

        let alpine_data = format!(
            r#"{{
                {ALPINE_LENGTH_BRIDGE}
                {ALPINE_LINES_SUBMIT_BRIDGE}
                {ALPINE_FKEY_BRIDGE}
                {ALPINE_LINE_TAX_BRIDGE}
                {ALPINE_CALC_BRIDGE}
                tax_pct_by_id: {tax_pct_json},
                tax_kind_by_id: {tax_kind_json},
                all_taxes: {all_taxes_json},
                default_material_taxes: {default_material_json},
                default_machine_taxes: {default_machine_json},
                tax_pick_base: '/finance-taxes/multi-select',
                components: {components_json},
                items: {rows_json},
                nextId: {next_id},
                init() {{
                    this.seedHiddenFromServer();
                    this.items.forEach(it => {{
                        it.variables = this.parseVariables(it.variables);
                        if (!it.component_label) {{
                            const c = this.getComp(it.component_id);
                            it.component_label = c ? c.name : '';
                        }}
                        if (!it.dim_units) it.dim_units = {{}};
                        this.initVarsFromSchema(it, this.schemaOfComp(it.component_id));
                        this.hydrateLineTaxes(it);
                        this.recalc(it);
                    }});
                    this.hookLinesMorphRestore();
                    this.restoreItemsFromHidden();
                    this.bindLineFormSubmit();
                }},
                restoreItemsFromHidden() {{
                    const h = this.$el.querySelector('[data-lines-hidden]');
                    if (!h) return;
                    this.seedHiddenFromServer();
                    const raw = (this.linesSeedRaw() || (h.value || '')).trim();
                    if (!raw || raw === '[]') {{
                        this.finalizeLinesRestore();
                        return;
                    }}
                    let arr;
                    try {{ arr = JSON.parse(raw); }} catch (e) {{
                        this.finalizeLinesRestore();
                        return;
                    }}
                    if (!Array.isArray(arr) || arr.length === 0) {{
                        this.finalizeLinesRestore();
                        return;
                    }}
                    let currentJson = '[]';
                    try {{ currentJson = this.jsonOutput(); }} catch (e) {{}}
                    const tbody = this.$el.querySelector('tbody');
                    const rowCount = tbody ? tbody.querySelectorAll('tr').length : 0;
                    if (currentJson === raw && rowCount === this.items.length && this.items.length > 0) {{
                        this.finalizeLinesRestore();
                        return;
                    }}
                    const rebuilt = arr.map((obj, i) => {{
                        const component_id = parseInt(obj.component_id, 10) || 0;
                        const variables = this.parseVariables(obj.variables);
                        const extra = this.parseExtra(obj.extra_data);
                        const dim_units = extra.dim_units || {{}};
                        const comp = this.getComp(component_id);
                        let component_label = obj.component_label || '';
                        if (!component_label && comp) component_label = comp.name;
                        return {{
                            id: i + 1,
                            db_id: obj.id != null ? obj.id : (obj.db_id != null ? obj.db_id : null),
                            component_id,
                            component_label,
                            variables,
                            dim_units,
                            final_cost: obj.final_cost != null ? String(obj.final_cost) : '',
                            extra_data: extra,
                            tax_ids: Array.isArray(obj.tax_ids) ? obj.tax_ids : [],
                            line_taxes: Array.isArray(obj.line_taxes) ? obj.line_taxes : [],
                        }};
                    }});
                    this.items = rebuilt;
                    this.nextId = rebuilt.reduce((m, it) => Math.max(m, it.id), 0) + 1;
                    rebuilt.forEach(it => {{
                        this.initVarsFromSchema(it, this.schemaOfComp(it.component_id));
                        this.hydrateLineTaxes(it);
                        this.recalc(it);
                    }});
                    this.finalizeLinesRestore();
                }},
                getComp(cid) {{
                    return this.components.find(c => String(c.id) === String(cid));
                }},
                schemaOfComp(cid) {{
                    const c = this.getComp(cid);
                    return (c && c.variables) ? c.variables : {{}};
                }},
                addItem() {{
                    const line_taxes = this.defaultLineTaxes('material');
                    const it = {{
                        id: this.nextId++,
                        db_id: null,
                        component_id: 0,
                        component_label: '',
                        variables: {{}},
                        dim_units: {{}},
                        final_cost: '',
                        extra_data: {{}},
                        tax_ids: line_taxes.map(t => parseInt(t.Key, 10)),
                        line_taxes
                    }};
                    this.items.push(it);
                    this.syncLineTaxStore(it);
                }},
                removeItem(idx) {{
                    this.items.splice(idx, 1);
                }},
                onCompChange(item) {{
                    const comp = this.getComp(item.component_id);
                    item.variables = {{}};
                    item.dim_units = {{}};
                    this.initVarsFromSchema(item, comp && comp.variables ? comp.variables : {{}});
                    item.final_cost = '';
                    this.recalc(item);
                }},
                lineUntaxedNumber(item) {{
                    return parseFloat(item.final_cost) || 0;
                }},
                recalc(item) {{
                    this.postCalculate('component', item.component_id, item);
                }},
                grandTotal() {{
                    return this.items.reduce((sum, it) => sum + this.lineTaxedTotal(it), 0);
                }},
                formatMoney(val) {{
                    return '₹ ' + (parseFloat(val) || 0).toFixed(2);
                }},
                jsonOutput() {{
                    const valid = this.items
                        .filter(it => it.component_id && parseInt(it.component_id, 10) > 0)
                        .map(it => {{
                            const extra = this.parseExtra(it.extra_data);
                            extra.dim_units = it.dim_units || {{}};
                            return {{
                                id: it.db_id || null,
                                component_id: parseInt(it.component_id, 10),
                                variables: it.variables || {{}},
                                extra_data: extra,
                                tax_ids: this.taxIdsOf(it)
                            }};
                        }});
                    return JSON.stringify(valid);
                }}
            }}"#
        );

        html! {
            div class="form-control mb-4 w-full min-w-0" data-lines-root="" data-lines-seed=(field.value)
                x-data=(alpine_data)
                x-effect="items.length; $nextTick(() => { if (window.htmx) window.htmx.process($el); if (typeof this.rewireFkeyPickers === 'function') this.rewireFkeyPickers(); })" {
                (PreEscaped(r#"<div hidden x-on:fk-select.window="onFkeySelect($event.detail)" x-on:fk-multi-select.window="onTaxMultiSelect($event.detail)"></div>"#))
                input type="hidden" name=(field.name) data-lines-hidden value=(field.value);

                (label(field.label, html! {
                (PreEscaped(r#"<div x-show="items.length === 0" class="p-4 text-center text-xs text-base-content/60 bg-base-100 rounded-lg border border-dashed border-base-300">"#))
                    "No material lines added yet. Click \"Add Line\" below to add items."
                (PreEscaped("</div>"))

                (PreEscaped(r#"<div x-show="items.length > 0" class="overflow-x-auto min-w-0 w-full border border-base-300 rounded-lg bg-base-100 shadow-sm">"#))
                    table class="table table-xs w-full min-w-max" {
                        thead class="bg-base-200/80 text-base-content/70" {
                            tr class="text-xs" {
                                th class="w-7 text-center" { "#" }
                                th class="w-40 min-w-[140px]" { "Component" }
                                th class="min-w-[240px]" { "Variables" }
                                th class="min-w-[8rem]" { "Taxes" }
                                th class="text-right w-24" { "Pre-tax (₹)" }
                                th class="text-right w-24" { "Total (₹)" }
                                th class="w-7 text-center" { "" }
                            }
                        }
                        tbody {
                            (PreEscaped(r#"<template x-for="(item, idx) in items" :key="item.id">"#))
                            tr class="hover border-b border-base-200 last:border-none" {
                                (PreEscaped(r#"<td class="align-middle text-center opacity-60 font-mono text-xs" x-text="idx + 1"></td>"#))
                                td class="align-middle min-w-[12rem]" {
                                    div class="min-w-0" data-fkey-field="component" x-bind:data-fkey-item-id="item.id"
                                        x-effect="bindFkeyInput($el, item, 'component')" {
                                        (embed_input_fkey("/work-orders/components/pick", "Select component…"))
                                    }
                                }
                                td class="align-middle" {
                                    (PreEscaped(r#"
                                    <div class="flex flex-col gap-1.5 py-1">
                                        <template x-for="v in schemaEntries(schemaOfComp(item.component_id))" :key="v.name">
                                            <div class="flex items-center gap-2">
                                                <span class="text-xs font-mono font-medium opacity-80 w-24 text-right shrink-0" x-text="v.name + ':'"></span>
                                    "#))
                                    div class="min-w-0" x-show="v.type === 'length'" x-cloak
                                        x-init="bindLengthInput($el, item, v.name)"
                                        x-on:input="pullLengthInput($el, item, v.name)"
                                        x-on:change="pullLengthInput($el, item, v.name)" {
                                        (embed_input_length())
                                    }
                                    (PreEscaped(r#"
                                                <input x-show="v.type === 'weight'" x-cloak type="number" min="0" step="any"
                                                       x-model="item.variables[v.name]" @input="scheduleRecalc(item)"
                                                       placeholder="kg"
                                                       class="input input-xs input-bordered w-24 text-right font-mono h-7 min-h-0">
                                                <input x-show="v.type === 'quantity'" x-cloak type="number" step="1"
                                                       x-model="item.variables[v.name]" @input="scheduleRecalc(item)"
                                                       placeholder="qty"
                                                       class="input input-xs input-bordered w-20 text-right font-mono h-7 min-h-0">
                                                <input x-show="v.type === 'duration'" x-cloak type="text"
                                                       x-model="item.variables[v.name]" @input="scheduleRecalc(item)"
                                                       placeholder="e.g. 2h 30m"
                                                       class="input input-xs input-bordered w-28 font-mono h-7 min-h-0">
                                            </div>
                                        </template>
                                        <template x-if="schemaEntries(schemaOfComp(item.component_id)).length === 0">
                                            <span class="text-xs italic opacity-60">No variables on this component</span>
                                        </template>
                                        <div x-show="item.calc_error" x-cloak class="text-xs text-error" x-text="item.calc_error"></div>
                                    </div>
                                    "#))
                                }
                                (embed_line_tax_picker())
                                td class="align-middle text-right font-mono text-xs" {
                                    (PreEscaped(r#"<span x-text="formatMoney(lineUntaxedNumber(item))"></span>"#))
                                }
                                td class="align-middle text-right font-mono font-bold text-primary text-xs" {
                                    (PreEscaped(r#"<span x-text="formatMoney(lineTaxedTotal(item))"></span>"#))
                                }
                                td class="align-middle text-center" {
                                    (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-xs text-error h-7 w-7 min-h-0 p-0 flex items-center justify-center mx-auto" title="Remove line" @click="removeItem(idx)">✕</button>"#))
                                }
                            }
                            (PreEscaped("</template>"))
                        }
                        tfoot class="bg-base-200/60 font-semibold border-t border-base-300" {
                            tr {
                                td colspan="5" class="align-middle text-right font-medium py-2" { "Final Grand Total (₹):" }
                                td class="align-middle text-right font-mono font-extrabold text-primary text-sm py-2" {
                                    (PreEscaped(r#"<span x-text="formatMoney(grandTotal())"></span>"#))
                                }
                                td class="align-middle" {}
                            }
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

pub struct MachineLinesWidget;

impl FormWidget for MachineLinesWidget {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let machines_json: &str = ctx.display_of(field.name);
        let machines_json = if machines_json.is_empty() {
            "[]"
        } else {
            machines_json
        };
        let mut rows = Vec::new();
        if let Ok(serde_json::Value::Array(arr)) =
            serde_json::from_str::<serde_json::Value>(field.value)
        {
            for (i, v) in arr.into_iter().enumerate() {
                if let serde_json::Value::Object(obj) = v {
                    let machine_id = json_i64_id(obj.get("machine_id"));
                    let machine_label = obj
                        .get("machine_label")
                        .or_else(|| obj.get("name"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    let variables = json_object_field(obj.get("variables"));
                    let db_id = json_i64_id(obj.get("id"));
                    let final_cost = obj
                        .get("final_cost")
                        .cloned()
                        .unwrap_or(serde_json::json!(""));
                    rows.push(serde_json::json!({
                        "id": i + 1,
                        "db_id": db_id,
                        "machine_id": machine_id,
                        "machine_label": machine_label,
                        "variables": variables,
                        "dim_units": {},
                        "final_cost": final_cost,
                        "tax_ids": parse_tax_ids_from_obj(&obj),
                        "line_taxes": [],
                        "total": 0,
                    }));
                }
            }
        }
        let next_id = rows.len() + 1;
        let rows_json = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());
        let (
            tax_pct_json,
            tax_kind_json,
            all_taxes_json,
            default_material_json,
            default_machine_json,
        ) = line_tax_catalog(ctx);

        let alpine_data = format!(
            r#"{{
                {ALPINE_LENGTH_BRIDGE}
                {ALPINE_LINES_SUBMIT_BRIDGE}
                {ALPINE_FKEY_BRIDGE}
                {ALPINE_LINE_TAX_BRIDGE}
                {ALPINE_CALC_BRIDGE}
                tax_pct_by_id: {tax_pct_json},
                tax_kind_by_id: {tax_kind_json},
                all_taxes: {all_taxes_json},
                default_material_taxes: {default_material_json},
                default_machine_taxes: {default_machine_json},
                tax_pick_base: '/finance-taxes/multi-select',
                machines: {machines_json},
                items: {rows_json},
                nextId: {next_id},
                init() {{
                    this.seedHiddenFromServer();
                    this.items.forEach(it => {{
                        it.variables = this.parseVariables(it.variables);
                        const m = this.getMachine(it.machine_id);
                        if (m && (!it.machine_label)) it.machine_label = m.name;
                        if (!it.dim_units) it.dim_units = {{}};
                        this.initVarsFromSchema(it, this.schemaOfMachine(it.machine_id));
                        this.hydrateLineTaxes(it);
                        this.recalc(it);
                    }});
                    this.hookLinesMorphRestore();
                    this.restoreItemsFromHidden();
                    this.bindLineFormSubmit();
                }},
                restoreItemsFromHidden() {{
                    const h = this.$el.querySelector('[data-lines-hidden]');
                    if (!h) return;
                    this.seedHiddenFromServer();
                    const raw = (this.linesSeedRaw() || (h.value || '')).trim();
                    if (!raw || raw === '[]') {{
                        this.finalizeLinesRestore();
                        return;
                    }}
                    let arr;
                    try {{ arr = JSON.parse(raw); }} catch (e) {{
                        this.finalizeLinesRestore();
                        return;
                    }}
                    if (!Array.isArray(arr) || arr.length === 0) {{
                        this.finalizeLinesRestore();
                        return;
                    }}
                    let currentJson = '[]';
                    try {{ currentJson = this.jsonOutput(); }} catch (e) {{}}
                    const tbody = this.$el.querySelector('tbody');
                    const rowCount = tbody ? tbody.querySelectorAll('tr').length : 0;
                    if (currentJson === raw && rowCount === this.items.length && this.items.length > 0) {{
                        this.finalizeLinesRestore();
                        return;
                    }}
                    const rebuilt = arr.map((obj, i) => ({{
                        id: i + 1,
                        db_id: obj.id != null ? obj.id : (obj.db_id != null ? obj.db_id : null),
                        machine_id: parseInt(obj.machine_id, 10) || 0,
                        machine_label: obj.machine_label || obj.name || '',
                        variables: this.parseVariables(obj.variables),
                        dim_units: {{}},
                        final_cost: obj.final_cost != null ? String(obj.final_cost) : '',
                        tax_ids: Array.isArray(obj.tax_ids) ? obj.tax_ids : [],
                        line_taxes: Array.isArray(obj.line_taxes) ? obj.line_taxes : [],
                        total: 0,
                    }}));
                    this.items = rebuilt;
                    this.nextId = rebuilt.reduce((m, it) => Math.max(m, it.id), 0) + 1;
                    rebuilt.forEach(it => {{
                        const m = this.getMachine(it.machine_id);
                        if (m && !it.machine_label) it.machine_label = m.name;
                        this.initVarsFromSchema(it, this.schemaOfMachine(it.machine_id));
                        this.hydrateLineTaxes(it);
                        this.recalc(it);
                    }});
                    this.finalizeLinesRestore();
                }},
                getMachine(id) {{
                    return this.machines.find(c => String(c.id) === String(id));
                }},
                schemaOfMachine(id) {{
                    const m = this.getMachine(id);
                    return (m && m.variables) ? m.variables : {{}};
                }},
                addItem() {{
                    const line_taxes = this.defaultLineTaxes('machine');
                    const it = {{
                        id: this.nextId++,
                        db_id: null,
                        machine_id: 0,
                        machine_label: '',
                        variables: {{}},
                        dim_units: {{}},
                        final_cost: '',
                        tax_ids: line_taxes.map(t => parseInt(t.Key, 10)),
                        line_taxes,
                        total: 0
                    }};
                    this.items.push(it);
                    this.syncLineTaxStore(it);
                }},
                removeItem(idx) {{
                    this.items.splice(idx, 1);
                }},
                onMachineChange(item) {{
                    const m = this.getMachine(item.machine_id);
                    item.variables = {{}};
                    item.dim_units = {{}};
                    this.initVarsFromSchema(item, m && m.variables ? m.variables : {{}});
                    item.final_cost = '';
                    this.recalc(item);
                }},
                lineUntaxedNumber(item) {{
                    return parseFloat(item.final_cost) || 0;
                }},
                recalc(item) {{
                    this.postCalculate('machine', item.machine_id, item);
                }},
                grandTotal() {{
                    return this.items.reduce((s, it) => s + this.lineTaxedTotal(it), 0);
                }},
                formatMoney(val) {{
                    return '₹ ' + (parseFloat(val) || 0).toFixed(2);
                }},
                jsonOutput() {{
                    const valid = this.items
                        .filter(it => it.machine_id && parseInt(it.machine_id, 10) > 0)
                        .map(it => ({{
                            id: it.db_id || null,
                            machine_id: parseInt(it.machine_id, 10),
                            variables: it.variables || {{}},
                            tax_ids: this.taxIdsOf(it)
                        }}));
                    return JSON.stringify(valid);
                }}
            }}"#
        );

        html! {
            div class="form-control mb-4 w-full min-w-0" data-lines-root="" data-lines-seed=(field.value)
                x-data=(alpine_data)
                x-effect="items.length; $nextTick(() => { if (window.htmx) window.htmx.process($el); if (typeof this.rewireFkeyPickers === 'function') this.rewireFkeyPickers(); })" {
                (PreEscaped(r#"<div hidden x-on:fk-select.window="onFkeySelect($event.detail)" x-on:fk-multi-select.window="onTaxMultiSelect($event.detail)"></div>"#))
                input type="hidden" name=(field.name) data-lines-hidden value=(field.value);

                (label(field.label, html! {
                (PreEscaped(r#"<div x-show="items.length === 0" class="p-4 text-center text-xs text-base-content/60 bg-base-100 rounded-lg border border-dashed border-base-300">"#))
                    "No machine lines added yet. Click \"Add Machine Line\" below to add machine time."
                (PreEscaped("</div>"))

                (PreEscaped(r#"<div x-show="items.length > 0" class="overflow-x-auto min-w-0 w-full border border-base-300 rounded-lg bg-base-100 shadow-sm">"#))
                    table class="table table-xs w-full min-w-max" {
                        thead class="bg-base-200/80 text-base-content/70" {
                            tr class="text-xs" {
                                th class="w-7 text-center" { "#" }
                                th class="min-w-[180px]" { "Machine" }
                                th class="min-w-[240px]" { "Variables" }
                                th class="min-w-[8rem]" { "Taxes" }
                                th class="text-right w-24" { "Pre-tax (₹)" }
                                th class="text-right w-24" { "Total (₹)" }
                                th class="w-7 text-center" { "" }
                            }
                        }
                        tbody {
                            (PreEscaped(r#"<template x-for="(item, idx) in items" :key="item.id">"#))
                            tr class="hover border-b border-base-200 last:border-none" x-bind:data-line-item-id="item.id" {
                                (PreEscaped(r#"<td class="align-middle text-center opacity-60 font-mono text-xs" x-text="idx + 1"></td>"#))
                                td class="align-middle min-w-[12rem]" {
                                    div class="min-w-0" data-fkey-field="machine" x-bind:data-fkey-item-id="item.id"
                                        x-effect="bindFkeyInput($el, item, 'machine')" {
                                        (embed_input_fkey("/machinery-schedule/machines/pick", "Select machine…"))
                                    }
                                }
                                td class="align-middle" {
                                    (PreEscaped(r#"
                                    <div class="flex flex-col gap-1.5 py-1">
                                        <template x-for="v in schemaEntries(schemaOfMachine(item.machine_id))" :key="v.name">
                                            <div class="flex items-center gap-2">
                                                <span class="text-xs font-mono font-medium opacity-80 w-24 text-right shrink-0" x-text="v.name + ':'"></span>
                                    "#))
                                    div class="min-w-0" x-show="v.type === 'length'" x-cloak
                                        x-init="bindLengthInput($el, item, v.name)"
                                        x-on:input="pullLengthInput($el, item, v.name)"
                                        x-on:change="pullLengthInput($el, item, v.name)" {
                                        (embed_input_length())
                                    }
                                    (PreEscaped(r#"
                                                <input x-show="v.type === 'weight'" x-cloak type="number" min="0" step="any"
                                                       x-model="item.variables[v.name]" @input="scheduleRecalc(item)"
                                                       placeholder="kg"
                                                       class="input input-xs input-bordered w-24 text-right font-mono h-7 min-h-0">
                                                <input x-show="v.type === 'quantity'" x-cloak type="number" step="1"
                                                       x-model="item.variables[v.name]" @input="scheduleRecalc(item)"
                                                       placeholder="qty"
                                                       class="input input-xs input-bordered w-20 text-right font-mono h-7 min-h-0">
                                                <input x-show="v.type === 'duration'" x-cloak type="text"
                                                       x-model="item.variables[v.name]" @input="scheduleRecalc(item)"
                                                       placeholder="e.g. 2h 30m"
                                                       class="input input-xs input-bordered w-28 font-mono h-7 min-h-0">
                                            </div>
                                        </template>
                                        <template x-if="schemaEntries(schemaOfMachine(item.machine_id)).length === 0">
                                            <span class="text-xs italic opacity-60">No variables on this machine</span>
                                        </template>
                                        <div x-show="item.calc_error" x-cloak class="text-xs text-error" x-text="item.calc_error"></div>
                                    </div>
                                    "#))
                                }
                                (embed_line_tax_picker())
                                td class="align-middle text-right font-mono text-xs" {
                                    (PreEscaped(r#"<span x-text="formatMoney(item.untaxed)"></span>"#))
                                }
                                td class="align-middle text-right font-mono font-bold text-primary text-xs" {
                                    (PreEscaped(r#"<span x-text="formatMoney(item.total)"></span>"#))
                                }
                                td class="align-middle text-center" {
                                    (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-xs text-error h-7 w-7 min-h-0 p-0 flex items-center justify-center mx-auto" title="Remove machine line" @click="removeItem(idx)">✕</button>"#))
                                }
                            }
                            (PreEscaped("</template>"))
                        }
                        tfoot class="bg-base-200/60 font-semibold border-t border-base-300" {
                            tr {
                                td colspan="5" class="align-middle text-right font-medium py-2" { "Machine Grand Total (₹):" }
                                td class="align-middle text-right font-mono font-extrabold text-primary text-sm py-2" {
                                    (PreEscaped(r#"<span x-text="formatMoney(grandTotal())"></span>"#))
                                }
                                td class="align-middle" {}
                            }
                        }
                    }
                (PreEscaped("</div>"))

                div class="flex justify-end mt-2" {
                    (PreEscaped(r#"<button type="button" class="btn btn-outline btn-xs btn-primary gap-1" @click="addItem()">"#))
                    (icon("plus", "w-3 h-3"))
                    "Add Machine Line"
                    (PreEscaped("</button>"))
                }
                }))
            }
        }
    }
}

pub type DraftWorkOrderMaterialLinesWidget = MaterialLinesWidget;
pub type DraftWorkOrderMachineLinesWidget = MachineLinesWidget;
pub type QuotationMaterialLinesWidget = MaterialLinesWidget;
pub type QuotationMachineLinesWidget = MachineLinesWidget;
pub type DraftWorkOrderItemsWidget = MaterialLinesWidget;
pub type WorkOrderItemsWidget = MaterialLinesWidget;

#[html_form]
pub struct DraftWorkOrderForm {
    #[form(label = "Order Number", widget = Text, placeholder = "e.g. DWO-1001 (optional)")]
    pub order_number: String,

    #[form(
        label = "Customer",
        required,
        widget = ForeignKey,
        route = lariv_rs::plugins::customer::routes::CustomerFkSelectRouteTag,
        swap_key = "fk-draft-work-order-customer",
        display = "customer",
        placeholder = "Select customer…"
    )]
    pub customer_id: i64,

    #[form(label = "Duration", required, widget = Duration)]
    pub duration: String,

    #[form(
        label = "Material Lines",
        widget = MaterialLinesWidget,
    )]
    pub items: Option<String>,

    #[form(
        label = "Machine Lines",
        widget = MachineLinesWidget,
    )]
    pub machine_lines: Option<String>,
}

pub type DraftWorkOrderCreateForm = DraftWorkOrderForm;
pub type DraftWorkOrderEditForm = DraftWorkOrderForm;
pub type WorkOrderForm = DraftWorkOrderForm;
pub type WorkOrderCreateForm = DraftWorkOrderForm;
pub type WorkOrderEditForm = DraftWorkOrderForm;

#[html_form]
pub struct DraftWorkOrderMaterialLineForm {
    #[form(
        label = "Draft Work Order",
        required,
        widget = ForeignKey,
        route = super::routes::WorkOrderFkSelectRouteTag,
        swap_key = "fk-draft-work-order-line-order",
        display = "draft_work_order",
        placeholder = "Select draft work order…"
    )]
    pub draft_work_order_id: i64,

    #[form(
        label = "Component",
        required,
        widget = ForeignKey,
        route = super::routes::ComponentFkSelectRouteTag,
        swap_key = "fk-draft-work-order-line-component",
        display = "component",
        placeholder = "Select component…"
    )]
    pub component_id: i64,

    #[form(label = "Variables (JSON)", required, widget = Text, placeholder = r#"{"length": "1000"}"#)]
    pub variables: String,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "wo-material-line-taxes",
        placeholder = "Select taxes…"
    )]
    pub taxes: Vec<i64>,

    #[form(label = "Extra Data (JSON)", widget = Textarea, rows = 3, placeholder = "{}")]
    pub extra_data: Option<String>,
}

pub type WorkOrderFormField = DraftWorkOrderFormField;
pub type DraftWorkOrderLineForm = DraftWorkOrderMaterialLineForm;
pub type DraftWorkOrderLineFormField = DraftWorkOrderMaterialLineFormField;
pub type WorkOrderLineFormField = DraftWorkOrderMaterialLineFormField;

pub type DraftWorkOrderMaterialLineEditForm = DraftWorkOrderMaterialLineForm;
pub type DraftWorkOrderLineEditForm = DraftWorkOrderMaterialLineForm;
pub type WorkOrderLineForm = DraftWorkOrderMaterialLineForm;
pub type WorkOrderLineEditForm = DraftWorkOrderMaterialLineForm;

pub type DraftWorkOrderItemForm = DraftWorkOrderMaterialLineForm;
pub type WorkOrderItemForm = DraftWorkOrderMaterialLineForm;
pub type WorkOrderItemCreateForm = DraftWorkOrderMaterialLineForm;
pub type WorkOrderItemEditForm = DraftWorkOrderMaterialLineForm;

#[html_form]
pub struct DraftWorkOrderMachineLineForm {
    #[form(
        label = "Draft Work Order",
        required,
        widget = ForeignKey,
        route = super::routes::WorkOrderFkSelectRouteTag,
        swap_key = "fk-draft-work-order-machine-line-order",
        display = "draft_work_order",
        placeholder = "Select draft work order…"
    )]
    pub draft_work_order_id: i64,

    #[form(
        label = "Machine",
        required,
        widget = ForeignKey,
        route = crate::machinery_schedule::routes::MachineFkSelectRouteTag,
        swap_key = "fk-draft-work-order-machine-line-machine",
        display = "machine",
        placeholder = "Select machine…"
    )]
    pub machine_id: i64,

    #[form(label = "Variables (JSON)", required, widget = Text, placeholder = r#"{"duration": "2h"}"#)]
    pub variables: String,

    #[form(
        label = "Taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "wo-machine-line-taxes",
        placeholder = "Select taxes…"
    )]
    pub taxes: Vec<i64>,
}

pub type WorkOrderMachineLineForm = DraftWorkOrderMachineLineForm;
pub type WorkOrderMachineLineEditForm = DraftWorkOrderMachineLineForm;
pub type WorkOrderMachineLineFormField = DraftWorkOrderMachineLineFormField;
pub type WorkOrderMachineLineEditFormField = DraftWorkOrderMachineLineFormField;
pub type DraftWorkOrderMachineLineEditForm = DraftWorkOrderMachineLineForm;
pub type DraftWorkOrderMachineLineEditFormField = DraftWorkOrderMachineLineFormField;

#[derive(Debug, Clone, Deserialize)]
pub struct DraftWorkOrderMachineLineInput {
    #[serde(default, alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "MachineId")]
    pub machine_id: i64,
    #[serde(default, alias = "Variables")]
    pub variables: Option<serde_json::Value>,
    #[serde(default, alias = "TaxIds", alias = "taxes")]
    pub tax_ids: Vec<i64>,
}

pub type WorkOrderMachineLineInput = DraftWorkOrderMachineLineInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftWorkOrderMaterialLineInput {
    #[serde(default, alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "ComponentId")]
    pub component_id: i64,
    #[serde(default, alias = "Variables")]
    pub variables: Option<serde_json::Value>,
    #[serde(default, alias = "ExtraData")]
    pub extra_data: Option<serde_json::Value>,
    #[serde(default, alias = "TaxIds", alias = "taxes")]
    pub tax_ids: Vec<i64>,
}

pub type DraftWorkOrderLineInput = DraftWorkOrderMaterialLineInput;

pub type WorkOrderItemInput = DraftWorkOrderLineInput;
pub type DraftWorkOrderItemInput = DraftWorkOrderLineInput;

#[html_form(default)]
pub struct WorkOrdersPreferencesForm {
    #[form(widget = Section, label = "Quotation numbering")]
    _section_number: (),

    #[form(
        label = "Quotation number format",
        widget = Text,
        placeholder = "QT-{{YYYY}}-{{POSTED_SEQ}}",
        hint = crate::work_orders::quotation_number::QUOTATION_NUMBER_FORMAT_HINT
    )]
    pub quotation_number_format: String,

    #[form(widget = Section, label = "Default line taxes")]
    _section_taxes: (),

    #[form(
        label = "Default material taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "wo-prefs-default-material-taxes",
        placeholder = "Select taxes…",
        hint = "Applied to new material lines when they are added."
    )]
    pub default_material_taxes: Vec<i64>,

    #[form(
        label = "Default machine taxes",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "wo-prefs-default-machine-taxes",
        placeholder = "Select taxes…",
        hint = "Applied to new machine lines when they are added."
    )]
    pub default_machine_taxes: Vec<i64>,

    #[form(widget = Section, label = "Draft Work Order PDF")]
    _section_wo: (),

    #[form(
        label = "Draft Work Order Template (Typst)",
        widget = CodeEditor,
        language = "typst",
        rows = 24
    )]
    pub draft_work_order_pdf_template: String,

    #[form(widget = Section, label = "Quotation PDF")]
    _section_inv: (),

    #[form(
        label = "Quotation Template (Typst)",
        widget = CodeEditor,
        language = "typst",
        rows = 24
    )]
    pub quotation_pdf_template: String,
}

// ==========================================
// 10. QUOTATION FORM WIDGETS
// ==========================================

/// Optional text input that honours placeholder and hint (stock Text widget does not).
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
            label_hint(field.label, ctx.hint_of(field.spec), html! { (input) })
        }
    }
}

pub struct DateInput;

impl FormWidget for DateInput {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        use maud::{PreEscaped, html};
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

#[html_form]
pub struct InvoiceForm {
    #[form(
        label = "Quotation Number",
        widget = OptionalText,
        placeholder = "Leave blank to auto-generate",
        hint = "Leave blank to assign a number from the KDS Quotations preferences format."
    )]
    pub invoice_number: String,

    #[form(label = "Quotation Date", required, widget = DateInput)]
    pub date: String,

    #[form(
        label = "Customer",
        required,
        widget = ForeignKey,
        route = lariv_rs::plugins::customer::routes::CustomerFkSelectRouteTag,
        swap_key = "fk-quotation-customer",
        display = "customer",
        placeholder = "Select customer…"
    )]
    pub customer_id: i64,

    #[form(label = "Duration", required, widget = Duration)]
    pub duration: String,

    #[form(
        label = "Material Lines",
        widget = MaterialLinesWidget,
    )]
    pub material_lines: Option<String>,

    #[form(
        label = "Machine Lines",
        widget = MachineLinesWidget,
    )]
    pub machine_lines: Option<String>,
}

pub type InvoiceCreateForm = InvoiceForm;
pub type InvoiceEditForm = InvoiceForm;

pub type InvoiceMaterialLineInput = DraftWorkOrderMaterialLineInput;
pub type InvoiceMachineLineInput = DraftWorkOrderMachineLineInput;
