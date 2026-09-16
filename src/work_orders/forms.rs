use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use lariv_rs::{
    components::icon,
    html_form::{
        html_form,
        widgets::{ForeignKey, List, Text, Textarea},
        FieldRender, FormCtx, FormWidget,
    },
};
use maud::{html, Markup, PreEscaped};

#[html_form]
pub struct ShapeForm {
    #[form(label = "Shape Name", required, widget = Text)]
    pub name: String,

    #[form(
        label = "Variables",
        widget = List,
        placeholder = "e.g. length"
    )]
    pub variables: Vec<String>,

    #[form(
        label = "OpenSCAD Code",
        required,
        widget = Textarea,
        rows = 6,
        placeholder = "cube([length, width, height], center=true);"
    )]
    pub openscad_code: String,
}

pub type ShapeCreateForm = ShapeForm;
pub type ShapeEditForm = ShapeForm;

pub const ALLOWED_VARIABLE_NAMES: &[&str] = &[
    "length",
    "width",
    "thickness",
    "height",
    "diameter",
    "outer_diameter",
    "inner_diameter",
    "across_flats",
    "wall_thickness",
    "web_thickness",
    "flange_thickness",
    "flange_width",
    "radius",
    "bolt_circle_diameter",
    "bolt_diameter",
    "num_bolts",
];

pub fn is_allowed_variable_name(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    ALLOWED_VARIABLE_NAMES.iter().any(|&v| v == lower)
}

pub struct KvList;

impl FormWidget for KvList {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let shape_vars = ctx.list_of(field.name);
        let choices = ctx.choices_of(field.name);
        let allowed_keys: Vec<String> = if !shape_vars.is_empty() {
            shape_vars.to_vec()
        } else if !choices.is_empty() {
            Vec::new()
        } else {
            ALLOWED_VARIABLE_NAMES.iter().map(|s| s.to_string()).collect()
        };

        let mut shape_vars_map: HashMap<String, Vec<String>> = HashMap::new();
        for (sid, vars_json) in choices {
            if let Ok(vars) = serde_json::from_str::<Vec<String>>(vars_json) {
                shape_vars_map.insert(sid.clone(), vars);
            }
        }
        let shape_vars_map_json = serde_json::to_string(&shape_vars_map).unwrap_or_else(|_| "{}".into());

        let mut rows = Vec::new();
        if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(field.value) {
            for (i, (k, v)) in map.into_iter().enumerate() {
                let val_str = match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                rows.push(serde_json::json!({
                    "id": i + 1,
                    "key": k,
                    "query": k,
                    "value": val_str,
                    "open": false,
                    "error": "",
                }));
            }
        }
        if rows.is_empty() {
            rows.push(serde_json::json!({
                "id": 1,
                "key": "",
                "query": "",
                "value": "",
                "open": false,
                "error": "",
            }));
        }
        let next_id = rows.len() + 1;
        let rows_json = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());
        let allowed_keys_json = serde_json::to_string(&allowed_keys).unwrap_or_else(|_| "[]".into());

        let alpine_data = format!(
            r#"{{
                items: {rows_json},
                nextId: {next_id},
                allowedKeys: {allowed_keys_json},
                allShapeVars: {shape_vars_map_json},
                init() {{
                    window.addEventListener('fk-select', (e) => this.onFkSelect(e.detail));
                    this.items.forEach(it => this.validateItem(it));
                }},
                hasShape() {{
                    return Array.isArray(this.allowedKeys) && this.allowedKeys.length > 0;
                }},
                onFkSelect(detail) {{
                    if (!detail) return;
                    const name = String(detail.name || '').toLowerCase().replace(/[-_]/g, '');
                    if (name === 'shapeid' || name === 'shape') {{
                        let vars = null;
                        if (detail.variables) {{
                            try {{
                                vars = typeof detail.variables === 'string' ? JSON.parse(detail.variables) : detail.variables;
                            }} catch (e) {{}}
                        }}
                        if ((!vars || vars.length === 0) && this.allShapeVars && detail.value && this.allShapeVars[String(detail.value)]) {{
                            vars = this.allShapeVars[String(detail.value)];
                        }}
                        if (Array.isArray(vars)) {{
                            this.allowedKeys = vars;
                            for (let i = 0; i < this.items.length; i++) {{
                                this.validateItem(this.items[i]);
                            }}
                        }}
                    }}
                }},
                usedKeys(excludeId) {{
                    const set = new Set();
                    for (let i = 0; i < this.items.length; i++) {{
                        const it = this.items[i];
                        if (it.id !== excludeId && it.key) {{
                            set.add(it.key.toLowerCase());
                        }}
                    }}
                    return set;
                }},
                availableKeys(item) {{
                    const used = this.usedKeys(item.id);
                    return this.allowedKeys.filter(k => !used.has(k.toLowerCase()));
                }},
                filteredKeys(item, query) {{
                    const avail = this.availableKeys(item);
                    const q = String(query || '').trim().toLowerCase();
                    if (!q) return avail;
                    return avail.filter(k => k.toLowerCase().includes(q));
                }},
                validateItem(item) {{
                    const q = String(item.query || item.key || '').trim();
                    if (!q) {{
                        item.error = '';
                        return;
                    }}
                    if (!this.hasShape()) {{
                        item.error = 'Please select a shape first';
                        return;
                    }}
                    const qLower = q.toLowerCase();
                    const allowedLower = (this.allowedKeys || []).map(k => k.toLowerCase());
                    if (!allowedLower.includes(qLower)) {{
                        item.error = `'${{q}}' is not allowed for this shape (allowed: ${{ (this.allowedKeys || []).join(', ') }})`;
                        return;
                    }}
                    const used = this.usedKeys(item.id);
                    if (used.has(qLower)) {{
                        item.error = `'${{q}}' is already used`;
                        return;
                    }}
                    item.error = '';
                }},
                selectKey(item, opt) {{
                    item.key = opt;
                    item.query = opt;
                    item.open = false;
                    item.error = '';
                }},
                onKeyBlur(item) {{
                    item.open = false;
                    const q = String(item.query || '').trim();
                    if (!q) {{
                        item.key = '';
                        item.query = '';
                        item.error = '';
                        return;
                    }}
                    const qLower = q.toLowerCase();
                    const avail = this.availableKeys(item);
                    const matched = avail.find(k => k.toLowerCase() === qLower);
                    if (matched) {{
                        item.key = matched;
                        item.query = matched;
                        item.error = '';
                    }} else {{
                        item.key = q;
                        this.validateItem(item);
                    }}
                }},
                onKeyEnter(item) {{
                    const filtered = this.filteredKeys(item, item.query);
                    if (filtered.length > 0) {{
                        this.selectKey(item, filtered[0]);
                    }} else {{
                        this.onKeyBlur(item);
                    }}
                }},
                canAdd() {{
                    if (!this.hasShape()) return false;
                    const used = this.usedKeys(null);
                    return used.size < this.allowedKeys.length;
                }},
                add() {{
                    if (!this.canAdd()) return;
                    this.items.push({{ id: this.nextId++, key: '', query: '', value: '', open: false, error: '' }});
                    this.$nextTick(() => {{
                        const inputs = this.$el.querySelectorAll('[data-kv-key-input]');
                        const last = inputs[inputs.length - 1];
                        if (last) last.focus();
                    }});
                }},
                remove(idx) {{
                    if (this.items.length <= 1) {{
                        this.items = [{{ id: this.nextId++, key: '', query: '', value: '', open: false, error: '' }}];
                        return;
                    }}
                    this.items.splice(idx, 1);
                }},
                moveUp(idx) {{
                    if (idx <= 0) return;
                    const arr = this.items.slice();
                    const tmp = arr[idx - 1];
                    arr[idx - 1] = arr[idx];
                    arr[idx] = tmp;
                    this.items = arr;
                }},
                moveDown(idx) {{
                    if (idx >= this.items.length - 1) return;
                    const arr = this.items.slice();
                    const tmp = arr[idx + 1];
                    arr[idx + 1] = arr[idx];
                    arr[idx] = tmp;
                    this.items = arr;
                }},
                jsonOutput() {{
                    const obj = {{}};
                    for (let i = 0; i < this.items.length; i++) {{
                        const k = String(this.items[i].key || this.items[i].query || '').trim();
                        const raw = this.items[i].value;
                        if (k && raw !== '' && raw !== null && raw !== undefined) {{
                            const v = parseFloat(raw);
                            if (!isNaN(v)) {{
                                obj[k] = v;
                            }}
                        }}
                    }}
                    return JSON.stringify(obj);
                }}
            }}"#
        );

        html! {
            div class="form-control mb-3 w-full" x-data=(alpine_data) {
                @if !field.label.is_empty() {
                    label class="label pb-1" {
                        span class="label-text font-semibold text-sm" { (field.label) }
                        span class="label-text-alt text-xs text-base-content/60" {
                            "Filtered for selected shape dimensions (mm)"
                        }
                    }
                }
                div class="flex flex-col gap-2 w-full" {
                    template x-for="(item, idx) in items" x-bind:key="item.id" {
                        (PreEscaped(r#"<div class="flex items-center gap-2 w-full p-2 bg-base-200/50 rounded-lg border border-base-200">"#))
                        // Reorder buttons
                        div class="flex flex-col shrink-0" {
                            (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-square btn-xs" @click="moveUp(idx)" :disabled="idx === 0" aria-label="Move up">"#))
                            (icon("chevron-up", "w-3 h-3"))
                            (PreEscaped("</button>"))
                            (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-square btn-xs" @click="moveDown(idx)" :disabled="idx === items.length - 1" aria-label="Move down">"#))
                            (icon("chevron-down", "w-3 h-3"))
                            (PreEscaped("</button>"))
                        }

                        // Key Searchable Dropdown Input (Only unassigned options allowed)
                        (PreEscaped(r#"
                        <div class="relative flex-1" @click.outside="onKeyBlur(item)">
                            <div class="relative">
                                <input type="text"
                                      class="input input-bordered input-sm w-full font-mono text-xs pr-7"
                                      :class="{ 'input-error': !!item.error }"
                                      :value="item.query"
                                      @focus="item.open = true; item.query = item.key"
                                      @click="item.open = true"
                                      @input="item.query = $event.target.value; item.open = true; validateItem(item)"
                                      @blur="onKeyBlur(item)"
                                      @keydown.enter.prevent="onKeyEnter(item)"
                                      @keydown.escape="item.open = false; item.query = item.key; validateItem(item)"
                                      placeholder="Select variable (e.g. width)"
                                      autocomplete="off"
                                      data-kv-key-input>
                                <div class="absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none text-base-content/40">
                                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
                                </div>
                            </div>
                            <div x-show="item.error"
                                 x-cloak
                                 class="text-xs text-error font-medium mt-1">
                                <span x-text="item.error"></span>
                            </div>

                            <div x-show="item.open"
                                 x-cloak
                                 class="absolute z-50 mt-1 max-h-48 w-full overflow-y-auto rounded-md bg-base-100 p-1 shadow-xl border border-base-300">
                                <template x-for="opt in filteredKeys(item, item.query)">
                                    <div class="px-2 py-1.5 text-xs rounded font-mono cursor-pointer hover:bg-primary/10 hover:text-primary transition-colors flex items-center justify-between"
                                         :class="{ 'bg-primary/15 font-semibold text-primary': item.key === opt }"
                                         @mousedown.prevent="selectKey(item, opt)">
                                        <span x-text="opt"></span>
                                        <span x-show="item.key === opt" class="text-xs text-primary">✓</span>
                                    </div>
                                </template>
                                <div x-show="filteredKeys(item, item.query).length === 0"
                                     class="px-2 py-2 text-xs text-base-content/60 italic text-center">
                                    <span x-show="!hasShape()">Please select a shape first</span>
                                    <span x-show="hasShape()">No available variables for this shape</span>
                                </div>
                            </div>
                        </div>
                        "#))

                        span class="text-base-content/40 font-bold shrink-0" { "=" }

                        // Value Input with unit badge
                        (PreEscaped(r#"
                        <div class="flex items-center gap-1 shrink-0">
                            <input type="number"
                                   step="any"
                                   class="input input-bordered input-sm w-28 font-mono text-xs"
                                   x-model="item.value"
                                   placeholder="Value"
                                   data-kv-val-input>
                            <span class="badge badge-ghost badge-sm font-mono text-xs">mm</span>
                        </div>
                        "#))

                        // Remove button
                        (PreEscaped(r#"
                        <button type="button"
                                class="btn btn-ghost btn-square btn-sm shrink-0 text-error hover:bg-error/10"
                                @click="remove(idx)"
                                title="Remove dimension"
                                aria-label="Remove">
                        "#))
                        (icon("x-mark", "w-4 h-4"))
                        (PreEscaped("</button></div>"))
                    }
                }

                // Add dimension button
                div class="flex items-center justify-between mt-2" {
                    (PreEscaped(r#"
                    <button type="button"
                            class="btn btn-outline btn-xs gap-1"
                            @click="add()"
                            :disabled="!canAdd()"
                            data-kv-add-btn>
                    "#))
                    (icon("plus", "w-3 h-3"))
                    "Add Fixed Dimension"
                    (PreEscaped("</button>"))
                    span class="text-xs text-base-content/50" {
                        "Dimensions left empty will remain parametric / free for work orders"
                    }
                }

                // Hidden input that syncs to form submission
                (PreEscaped(format!(
                    r#"<input type="hidden" name="{}" :value="jsonOutput()" data-kv-hidden>"#,
                    field.name
                )))
            }
        }
    }
}

#[html_form]
pub struct ComponentForm {
    #[form(label = "Component Name", required, widget = Text)]
    pub name: String,

    #[form(
        label = "Shape",
        required,
        widget = ForeignKey,
        url = "/work-orders/shapes/pick",
        swap_key = "fk-component-shape",
        placeholder = "Select a shape..."
    )]
    pub shape_id: i64,

    #[form(
        label = "Material",
        required,
        widget = ForeignKey,
        url = "/work-orders/materials/pick",
        swap_key = "fk-component-material",
        placeholder = "Select a material..."
    )]
    pub material_id: i64,

    #[form(
        label = "Fixed Dimensions / Stock Sizes (mm)",
        widget = KvList,
    )]
    pub fixed_variables: Option<String>,
}

pub type ComponentCreateForm = ComponentForm;
pub type ComponentEditForm = ComponentForm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMeta {
    pub id: i64,
    pub name: String,
    pub shape_id: i64,
    pub shape_name: String,
    pub shape_kind: Option<String>,
    pub material_id: i64,
    pub material_name: String,
    pub density: f64,
    pub material_rate: f64,
    pub variable_names: Vec<String>,
    pub fixed_variables: HashMap<String, f64>,
    pub free_variables: Vec<String>,
}

pub struct DraftWorkOrderMaterialLinesWidget;

impl FormWidget for DraftWorkOrderMaterialLinesWidget {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let comp_str = ctx.display_of(field.name);
        let components_json = if comp_str.is_empty() {
            let alt = ctx.display_of("components_data");
            if alt.is_empty() { "[]" } else { alt }
        } else {
            comp_str
        };
        let mut rows = Vec::new();
        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(field.value) {
            for (i, v) in arr.into_iter().enumerate() {
                if let serde_json::Value::Object(obj) = v {
                    let component_id = obj.get("component_id").and_then(|x| x.as_i64()).unwrap_or(0);
                    let variables = obj.get("variables").cloned().unwrap_or_else(|| serde_json::json!({}));
                    let quantity = obj.get("quantity").map(|x| match x {
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        _ => "1".to_string(),
                    }).unwrap_or_else(|| "1".to_string());
                    let unit_weight = obj.get("unit_weight").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let material_rate = obj.get("material_rate").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let final_cost = obj.get("final_cost").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let extra_data = obj.get("extra_data").map(|x| match x {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Object(_) | serde_json::Value::Array(_) => x.to_string(),
                        _ => String::new(),
                    }).unwrap_or_default();

                    let dim_units = obj.get("extra_data")
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
                            serde_json::Value::String(s) => {
                                serde_json::from_str::<serde_json::Value>(s).ok().and_then(|j| {
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
                                })
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| serde_json::json!({}));

                    rows.push(serde_json::json!({
                        "id": i + 1,
                        "component_id": component_id,
                        "variables": variables,
                        "dim_units": dim_units,
                        "mode": "dim",
                        "target_weight": "",
                        "target_cost": "",
                        "quantity": quantity,
                        "unit_weight": unit_weight,
                        "material_rate": material_rate,
                        "final_cost": final_cost,
                        "extra_data": extra_data,
                    }));
                }
            }
        }
        let next_id = rows.len() + 1;
        let rows_json = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());

        let alpine_data = format!(
            r#"{{
                components: {components_json},
                items: {rows_json},
                nextId: {next_id},
                unitFactors: {{
                    'mm': 1.0,
                    'cm': 10.0,
                    'm': 1000.0,
                    'km': 1000000.0,
                    'in': 25.4,
                    'ft': 304.8
                }},
                roundVal(val) {{
                    if (val === '' || val === null || isNaN(val) || val === 0) return val;
                    return parseFloat(Number(val).toFixed(6));
                }},
                init() {{
                    this.items.forEach(it => {{
                        if (!it.dim_units) it.dim_units = {{}};
                        it.dim_inputs = {{}};
                        if (it.variables) {{
                            for (const k in it.variables) {{
                                const unit = it.dim_units[k] || it.dim_units['default'] || 'mm';
                                it.dim_units[k] = unit;
                                const factor = this.unitFactors[unit] || 1.0;
                                const v_mm = parseFloat(it.variables[k]);
                                if (!isNaN(v_mm) && v_mm > 0) {{
                                    it.dim_inputs[k] = this.roundVal(v_mm / factor);
                                }} else {{
                                    it.dim_inputs[k] = '';
                                }}
                            }}
                        }}
                        this.recalc(it);
                    }});
                }},
                getComp(cid) {{
                    return this.components.find(c => String(c.id) === String(cid));
                }},
                addItem() {{
                    const firstComp = this.components.length > 0 ? this.components[0] : null;
                    const compId = firstComp ? firstComp.id : 0;
                    const initialVars = {{}};
                    const initialDimInputs = {{}};
                    const initialDimUnits = {{}};
                    if (firstComp && firstComp.free_variables) {{
                        firstComp.free_variables.forEach(v => {{
                            initialVars[v] = '';
                            initialDimInputs[v] = '';
                            initialDimUnits[v] = 'mm';
                        }});
                    }}
                    const it = {{
                        id: this.nextId++,
                        component_id: compId,
                        variables: initialVars,
                        dim_inputs: initialDimInputs,
                        dim_units: initialDimUnits,
                        mode: 'dim',
                        target_weight: '',
                        target_cost: '',
                        quantity: '1',
                        unit_weight: 0,
                        material_rate: firstComp ? firstComp.material_rate : 0,
                        final_cost: 0,
                        extra_data: ''
                    }};
                    this.recalc(it);
                    this.items.push(it);
                }},
                removeItem(idx) {{
                    this.items.splice(idx, 1);
                }},
                onCompChange(item) {{
                    const comp = this.getComp(item.component_id);
                    if (!comp) return;
                    item.variables = {{}};
                    item.dim_inputs = {{}};
                    item.dim_units = {{}};
                    if (comp.free_variables) {{
                        comp.free_variables.forEach(v => {{
                            item.variables[v] = '';
                            item.dim_inputs[v] = '';
                            item.dim_units[v] = 'mm';
                        }});
                    }}
                    item.material_rate = comp.material_rate || 0;
                    item.mode = 'dim';
                    item.target_weight = '';
                    item.target_cost = '';
                    this.recalc(item);
                }},
                onDimInput(item, varName) {{
                    if (!item.dim_units) item.dim_units = {{}};
                    const unit = item.dim_units[varName] || 'mm';
                    const factor = this.unitFactors[unit] || 1.0;
                    const rawVal = item.dim_inputs ? item.dim_inputs[varName] : '';
                    if (rawVal === '' || rawVal === null || isNaN(rawVal)) {{
                        item.variables[varName] = '';
                    }} else {{
                        const val = parseFloat(rawVal);
                        item.variables[varName] = this.roundVal(val * factor);
                    }}
                    this.recalc(item);
                }},
                onVarUnitChange(item, varName, newUnit) {{
                    if (!item.dim_units) item.dim_units = {{}};
                    item.dim_units[varName] = newUnit;
                    const newFactor = this.unitFactors[newUnit || 'mm'] || 1.0;
                    if (!item.dim_inputs) item.dim_inputs = {{}};
                    let val_mm = parseFloat(item.variables[varName]);
                    if ((isNaN(val_mm) || val_mm <= 0) && item.dim_inputs[varName] !== '' && !isNaN(item.dim_inputs[varName])) {{
                        val_mm = parseFloat(item.dim_inputs[varName]);
                    }}
                    if (!isNaN(val_mm) && val_mm > 0) {{
                        item.dim_inputs[varName] = this.roundVal(val_mm / newFactor);
                    }} else {{
                        item.dim_inputs[varName] = '';
                    }}
                    this.recalc(item);
                }},
                calcWeight(comp, vars) {{
                    if (!comp || !comp.density) return 0;
                    const allVars = Object.assign({{}}, comp.fixed_variables || {{}}, vars || {{}});
                    const kind = (comp.shape_kind || '').toLowerCase();
                    const name = (comp.shape_name || '').toLowerCase();
                    let vol_m3 = 0;
                    if (kind === 'box' || name.includes('box') || name.includes('plate')) {{
                        const l = parseFloat(allVars['length']) || 0;
                        const w = parseFloat(allVars['width']) || 0;
                        const t = parseFloat(allVars['thickness']) || 0;
                        vol_m3 = (l * w * t) * 1e-9;
                    }} else if (kind === 'cylinder' || name.includes('cylinder') || name.includes('round bar')) {{
                        const d = parseFloat(allVars['diameter']) || 0;
                        const l = parseFloat(allVars['length']) || 0;
                        vol_m3 = Math.PI * Math.pow(d / 2.0, 2) * l * 1e-9;
                    }} else if (kind === 'hexbar' || name.includes('hex')) {{
                        const s = parseFloat(allVars['across_flats']) || 0;
                        const l = parseFloat(allVars['length']) || 0;
                        vol_m3 = (Math.sqrt(3) / 2.0) * Math.pow(s, 2) * l * 1e-9;
                    }} else if (kind === 'pipe' || name.includes('pipe')) {{
                        const od = parseFloat(allVars['outer_diameter']) || 0;
                        const id = parseFloat(allVars['inner_diameter']) || 0;
                        const l = parseFloat(allVars['length']) || 0;
                        vol_m3 = (Math.PI / 4.0) * (Math.pow(od, 2) - Math.pow(id, 2)) * l * 1e-9;
                    }} else {{
                        let prod = 1.0;
                        let count = 0;
                        for (const k in allVars) {{
                            const v = parseFloat(allVars[k]);
                            if (v > 0) {{ prod *= v; count++; }}
                        }}
                        if (count >= 3) vol_m3 = prod * 1e-9;
                    }}
                    return vol_m3 * (comp.density || 0);
                }},
                solveDim(comp, freeVar, targetWeight) {{
                    if (!comp || targetWeight <= 0 || !comp.density) return 0;
                    const vol_m3 = targetWeight / comp.density;
                    const vol_mm3 = vol_m3 * 1e9;
                    const kind = (comp.shape_kind || '').toLowerCase();
                    const name = (comp.shape_name || '').toLowerCase();
                    const fixed = comp.fixed_variables || {{}};
                    if (kind === 'box' || name.includes('box') || name.includes('plate')) {{
                        let otherProd = 1.0;
                        ['length', 'width', 'thickness'].forEach(k => {{
                            if (k !== freeVar) {{
                                otherProd *= (parseFloat(fixed[k]) || 1.0);
                            }}
                        }});
                        return otherProd > 0 ? (vol_mm3 / otherProd) : 0;
                    }} else if (kind === 'cylinder' || name.includes('cylinder') || name.includes('round bar')) {{
                        if (freeVar === 'length') {{
                            const d = parseFloat(fixed['diameter']) || 0;
                            const area = Math.PI * Math.pow(d / 2.0, 2);
                            return area > 0 ? (vol_mm3 / area) : 0;
                        }} else if (freeVar === 'diameter') {{
                            const l = parseFloat(fixed['length']) || 0;
                            return l > 0 ? (2.0 * Math.sqrt(vol_mm3 / (Math.PI * l))) : 0;
                        }}
                    }} else if (kind === 'hexbar' || name.includes('hex')) {{
                        if (freeVar === 'length') {{
                            const s = parseFloat(fixed['across_flats']) || 0;
                            const area = (Math.sqrt(3) / 2.0) * Math.pow(s, 2);
                            return area > 0 ? (vol_mm3 / area) : 0;
                        }}
                    }}
                    return 0;
                }},
                recalc(item) {{
                    const comp = this.getComp(item.component_id);
                    if (!comp) {{
                        item.unit_weight = 0;
                        item.final_cost = 0;
                        return;
                    }}
                    item.material_rate = comp.material_rate || 0;
                    const free = comp.free_variables || [];
                    if (!item.dim_inputs) item.dim_inputs = {{}};
                    if (!item.dim_units) item.dim_units = {{}};

                    if (free.length === 1) {{
                        const freeVar = free[0];
                        const unit = item.dim_units[freeVar] || 'mm';
                        const factor = this.unitFactors[unit] || 1.0;
                        if (item.mode === 'weight' && parseFloat(item.target_weight) > 0) {{
                            const w = parseFloat(item.target_weight);
                            const dim_mm = this.solveDim(comp, freeVar, w);
                            if (dim_mm > 0) {{
                                item.variables[freeVar] = parseFloat(dim_mm.toFixed(2));
                                item.dim_inputs[freeVar] = this.roundVal(dim_mm / factor);
                                item.target_cost = (w * comp.material_rate).toFixed(2);
                            }}
                        }} else if (item.mode === 'cost' && parseFloat(item.target_cost) > 0 && comp.material_rate > 0) {{
                            const c = parseFloat(item.target_cost);
                            const w = c / comp.material_rate;
                            item.target_weight = w.toFixed(3);
                            const dim_mm = this.solveDim(comp, freeVar, w);
                            if (dim_mm > 0) {{
                                item.variables[freeVar] = parseFloat(dim_mm.toFixed(2));
                                item.dim_inputs[freeVar] = this.roundVal(dim_mm / factor);
                            }}
                        }} else {{
                            // mode === 'dim'
                            const raw = item.dim_inputs[freeVar];
                            if (raw !== '' && raw !== null && !isNaN(raw)) {{
                                item.variables[freeVar] = this.roundVal(parseFloat(raw) * factor);
                            }}
                            const dimVal = parseFloat(item.variables[freeVar]) || 0;
                            if (dimVal > 0) {{
                                const w = this.calcWeight(comp, item.variables);
                                item.target_weight = w > 0 ? w.toFixed(3) : '';
                                item.target_cost = (w * comp.material_rate).toFixed(2);
                            }}
                        }}
                    }} else if (free.length > 1) {{
                        free.forEach(v => {{
                            const unit = item.dim_units[v] || 'mm';
                            const factor = this.unitFactors[unit] || 1.0;
                            const raw = item.dim_inputs ? item.dim_inputs[v] : '';
                            if (raw !== '' && raw !== null && !isNaN(raw)) {{
                                item.variables[v] = this.roundVal(parseFloat(raw) * factor);
                            }} else {{
                                item.variables[v] = '';
                            }}
                        }});
                    }}
                    const weight = this.calcWeight(comp, item.variables);
                    item.unit_weight = weight;
                    const qty = parseFloat(item.quantity) || 0;
                    item.final_cost = qty * (comp.material_rate || 0) * weight;
                }},
                grandTotal() {{
                    return this.items.reduce((sum, it) => sum + (it.final_cost || 0), 0);
                }},
                formatMoney(val) {{
                    return '₹ ' + (val || 0).toFixed(2);
                }},
                jsonOutput() {{
                    const valid = this.items
                        .filter(it => it.component_id && parseInt(it.component_id, 10) > 0)
                        .map(it => {{
                            let extra = {{}};
                            if (typeof it.extra_data === 'string' && it.extra_data.trim().length > 0) {{
                                try {{ extra = JSON.parse(it.extra_data); }} catch(e) {{ extra = {{ raw: it.extra_data }}; }}
                            }} else if (typeof it.extra_data === 'object' && it.extra_data !== null) {{
                                extra = Object.assign({{}}, it.extra_data);
                            }}
                            extra.dim_units = it.dim_units || {{}};
                            const comp = this.getComp(it.component_id);
                            const free = comp ? (comp.free_variables || []) : [];
                            if (free.length === 1 && it.dim_units && it.dim_units[free[0]]) {{
                                extra.dim_unit = it.dim_units[free[0]];
                            }}
                            return {{
                                id: it.id || null,
                                component_id: parseInt(it.component_id, 10),
                                variables: JSON.stringify(it.variables || {{}}),
                                quantity: String(it.quantity || '1'),
                                unit_weight: String((it.unit_weight || 0).toFixed(4)),
                                material_rate: String((it.material_rate || 0).toFixed(2)),
                                final_cost: String((it.final_cost || 0).toFixed(2)),
                                extra_data: JSON.stringify(extra)
                            }};
                        }});
                    return JSON.stringify(valid);
                }}
            }}"#
        );

        html! {
            div class="form-control mb-4 w-full" x-data=(alpine_data) {
                input type="hidden" name=(field.name) x-bind:value="jsonOutput()";

                div class="flex justify-between items-center mb-2" {
                    label class="label p-0" {
                        span class="label-text font-bold text-base" { (field.label) }
                    }
                    (PreEscaped(r#"<button type="button" class="btn btn-outline btn-xs btn-primary gap-1" @click="addItem()">"#))
                    (icon("plus", "w-3 h-3"))
                    "Add Line"
                    (PreEscaped("</button>"))
                }

                (PreEscaped(r#"<template x-if="items.length === 0">"#))
                div class="p-4 text-center text-xs text-base-content/60 bg-base-100 rounded-lg border border-dashed border-base-300" {
                    "No lines added yet. Click \"Add Line\" above to add items to this draft work order."
                }
                (PreEscaped("</template>"))

                (PreEscaped(r#"<template x-if="items.length > 0">"#))
                div class="overflow-x-auto border border-base-300 rounded-lg bg-base-100 shadow-sm" {
                    table class="table table-xs w-full" {
                        thead class="bg-base-200/80 text-base-content/70" {
                            tr class="text-xs" {
                                th class="w-7 text-center" { "#" }
                                th class="w-40 min-w-[140px]" { "Component" }
                                th class="min-w-[240px]" { "Dimensions / Variables" }
                                th class="text-right w-16" { "Weight (kg)" }
                                th class="text-right w-16" { "Rate (₹/kg)" }
                                th class="text-center w-14" { "Qty" }
                                th class="text-right w-20" { "Final Cost (₹)" }
                                th class="w-7 text-center" { "" }
                            }
                        }
                        tbody {
                            (PreEscaped(r#"<template x-for="(item, idx) in items" :key="item.id">"#))
                            tr class="hover border-b border-base-200 last:border-none" {
                                (PreEscaped(r#"<td class="align-middle text-center opacity-60 font-mono text-xs" x-text="idx + 1"></td>"#))
                                td class="align-middle" {
                                    (PreEscaped(r#"
                                    <select class="select select-bordered select-xs w-full h-7 min-h-0 text-xs font-medium"
                                            x-model="item.component_id"
                                            @change="onCompChange(item)">
                                        <template x-for="c in components" :key="c.id">
                                            <option :value="c.id" x-text="c.name + ' (' + c.material_name + ')'"></option>
                                        </template>
                                    </select>
                                    "#))
                                }
                                td class="align-middle" {
                                    (PreEscaped(r#"
                                    <div class="flex items-center">
                                        <template x-if="getComp(item.component_id) && (getComp(item.component_id).free_variables || []).length === 1">
                                            <div class="flex flex-col gap-1.5 py-1">
                                                <div class="flex items-center gap-2">
                                                    <span class="w-24 shrink-0"></span>
                                                    <div class="join join-horizontal shadow-none shrink-0">
                                                        <button type="button" class="btn btn-xs join-item h-6 min-h-0 px-2 font-semibold text-[11px]"
                                                                :class="{ 'btn-primary': item.mode === 'dim' }"
                                                                @click="item.mode = 'dim'; recalc(item)">Dim</button>
                                                        <button type="button" class="btn btn-xs join-item h-6 min-h-0 px-2 font-semibold text-[11px]"
                                                                :class="{ 'btn-primary': item.mode === 'weight' }"
                                                                @click="item.mode = 'weight'; recalc(item)">Weight</button>
                                                        <button type="button" class="btn btn-xs join-item h-6 min-h-0 px-2 font-semibold text-[11px]"
                                                                :class="{ 'btn-primary': item.mode === 'cost' }"
                                                                @click="item.mode = 'cost'; recalc(item)">Cost</button>
                                                    </div>
                                                </div>

                                                <template x-if="item.mode === 'dim'">
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-xs font-mono font-medium opacity-80 w-24 text-right shrink-0"
                                                              x-text="getComp(item.component_id).free_variables[0] + ':'"></span>
                                                        <div class="join join-horizontal items-center shrink-0">
                                                            <input type="number" step="any"
                                                                   class="input input-xs input-bordered join-item w-20 font-mono text-xs h-7 min-h-0 text-right pr-1"
                                                                   :placeholder="getComp(item.component_id).free_variables[0]"
                                                                   x-model="item.dim_inputs[getComp(item.component_id).free_variables[0]]"
                                                                   @input="onDimInput(item, getComp(item.component_id).free_variables[0])">
                                                            <select class="select select-xs select-bordered join-item font-mono text-xs h-7 min-h-0 px-2 cursor-pointer bg-base-100"
                                                                    :value="(item.dim_units && item.dim_units[getComp(item.component_id).free_variables[0]]) || 'mm'"
                                                                    @change="onVarUnitChange(item, getComp(item.component_id).free_variables[0], $event.target.value)">
                                                                <option value="mm">mm</option>
                                                                <option value="cm">cm</option>
                                                                <option value="m">m</option>
                                                                <option value="km">km</option>
                                                                <option value="in">in</option>
                                                                <option value="ft">ft</option>
                                                            </select>
                                                        </div>
                                                    </div>
                                                </template>

                                                <template x-if="item.mode === 'weight'">
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-xs font-mono font-medium opacity-80 w-24 text-right shrink-0">weight:</span>
                                                        <div class="join join-horizontal items-center shrink-0">
                                                            <input type="number" step="any"
                                                                   class="input input-xs input-bordered join-item w-20 font-mono text-xs h-7 min-h-0 text-right pr-1"
                                                                   placeholder="Target"
                                                                   x-model="item.target_weight"
                                                                   @input="recalc(item)">
                                                            <span class="btn btn-xs join-item no-animation bg-base-200 h-7 min-h-0 px-2 font-mono text-xs opacity-70 pointer-events-none">kg</span>
                                                        </div>
                                                        <template x-if="item.dim_inputs && item.dim_inputs[getComp(item.component_id).free_variables[0]] > 0">
                                                            <span class="badge badge-sm badge-ghost font-mono text-[11px] h-6 whitespace-nowrap"
                                                                  x-text="item.dim_inputs[getComp(item.component_id).free_variables[0]] + ' ' + ((item.dim_units && item.dim_units[getComp(item.component_id).free_variables[0]]) || 'mm')"></span>
                                                        </template>
                                                    </div>
                                                </template>

                                                <template x-if="item.mode === 'cost'">
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-xs font-mono font-medium opacity-80 w-24 text-right shrink-0">cost:</span>
                                                        <div class="join join-horizontal items-center shrink-0">
                                                            <input type="number" step="any"
                                                                   class="input input-xs input-bordered join-item w-20 font-mono text-xs h-7 min-h-0 text-right pr-1"
                                                                   placeholder="Target"
                                                                   x-model="item.target_cost"
                                                                   @input="recalc(item)">
                                                            <span class="btn btn-xs join-item no-animation bg-base-200 h-7 min-h-0 px-2 font-mono text-xs opacity-70 pointer-events-none">₹</span>
                                                        </div>
                                                        <template x-if="item.dim_inputs && item.dim_inputs[getComp(item.component_id).free_variables[0]] > 0">
                                                            <span class="badge badge-sm badge-ghost font-mono text-[11px] h-6 whitespace-nowrap"
                                                                  x-text="item.dim_inputs[getComp(item.component_id).free_variables[0]] + ' ' + ((item.dim_units && item.dim_units[getComp(item.component_id).free_variables[0]]) || 'mm')"></span>
                                                        </template>
                                                    </div>
                                                </template>
                                            </div>
                                        </template>

                                        <template x-if="getComp(item.component_id) && (getComp(item.component_id).free_variables || []).length > 1">
                                            <div class="flex flex-col gap-1.5 py-1">
                                                <template x-for="v in getComp(item.component_id).free_variables" :key="v">
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-xs font-mono font-medium opacity-80 w-24 text-right shrink-0" x-text="v + ':'"></span>
                                                        <div class="join join-horizontal items-center shrink-0">
                                                            <input type="number" step="any"
                                                                   class="input input-xs input-bordered join-item w-20 font-mono text-xs h-7 min-h-0 text-right pr-1"
                                                                   :placeholder="v"
                                                                   x-model="item.dim_inputs[v]"
                                                                   @input="onDimInput(item, v)">
                                                            <select class="select select-xs select-bordered join-item font-mono text-xs h-7 min-h-0 px-2 cursor-pointer bg-base-100"
                                                                    :value="(item.dim_units && item.dim_units[v]) || 'mm'"
                                                                    @change="onVarUnitChange(item, v, $event.target.value)">
                                                                <option value="mm">mm</option>
                                                                <option value="cm">cm</option>
                                                                <option value="m">m</option>
                                                                <option value="km">km</option>
                                                                <option value="in">in</option>
                                                                <option value="ft">ft</option>
                                                            </select>
                                                        </div>
                                                    </div>
                                                </template>
                                            </div>
                                        </template>

                                        <template x-if="getComp(item.component_id) && (getComp(item.component_id).free_variables || []).length === 0">
                                            <span class="text-xs italic opacity-60">Fully fixed stock sizes</span>
                                        </template>
                                    </div>
                                    "#))
                                }
                                td class="align-middle text-right font-mono text-xs" {
                                    (PreEscaped(r#"<span x-text="(item.unit_weight || 0).toFixed(3)"></span>"#))
                                }
                                td class="align-middle text-right font-mono text-xs opacity-80" {
                                    (PreEscaped(r#"<span x-text="(item.material_rate || 0).toFixed(2)"></span>"#))
                                }
                                td class="align-middle text-center" {
                                    (PreEscaped(r#"<input type="number" min="0.001" step="any" x-model="item.quantity" @input="recalc(item)" placeholder="1" class="input input-xs input-bordered w-14 text-center font-mono h-7 min-h-0 mx-auto block" required>"#))
                                }
                                td class="align-middle text-right font-mono font-bold text-primary text-xs" {
                                    (PreEscaped(r#"<span x-text="formatMoney(item.final_cost)"></span>"#))
                                }
                                td class="align-middle text-center" {
                                    (PreEscaped(r#"<button type="button" class="btn btn-ghost btn-xs text-error h-7 w-7 min-h-0 p-0 flex items-center justify-center mx-auto" title="Remove line" @click="removeItem(idx)">✕</button>"#))
                                }
                            }
                            (PreEscaped("</template>"))
                        }
                        tfoot class="bg-base-200/60 font-semibold border-t border-base-300" {
                            tr {
                                td colspan="6" class="align-middle text-right font-medium py-2" { "Final Grand Total (₹):" }
                                td class="align-middle text-right font-mono font-extrabold text-primary text-sm py-2" {
                                    (PreEscaped(r#"<span x-text="formatMoney(grandTotal())"></span>"#))
                                }
                                td class="align-middle" {}
                            }
                        }
                    }
                }
                (PreEscaped("</template>"))
            }
        }
    }
}

pub struct DraftWorkOrderMachineLinesWidget;

impl FormWidget for DraftWorkOrderMachineLinesWidget {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let machines_json = ctx.display_of(field.name);
        let mut rows = Vec::new();
        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(field.value) {
            for (i, v) in arr.into_iter().enumerate() {
                if let serde_json::Value::Object(obj) = v {
                    let machine_id = obj.get("machine_id").and_then(|x| match x {
                        serde_json::Value::Number(n) => n.as_i64(),
                        serde_json::Value::String(s) => s.trim().parse::<i64>().ok(),
                        _ => None,
                    }).unwrap_or(0);
                    let rate = obj.get("rate").or_else(|| obj.get("rate_decimal")).map(|x| match x {
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        _ => String::new(),
                    }).unwrap_or_default();
                    let duration = obj.get("duration").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let db_id = obj.get("id").and_then(|x| match x {
                        serde_json::Value::Number(n) => n.as_i64(),
                        _ => None,
                    }).unwrap_or(0);

                    rows.push(serde_json::json!({
                        "id": i + 1,
                        "db_id": db_id,
                        "machine_id": machine_id,
                        "rate": rate,
                        "duration": duration,
                        "total": 0,
                    }));
                }
            }
        }
        let next_id = rows.len() + 1;
        let rows_json = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());

        let alpine_data = format!(
            r#"{{
                machines: {machines_json},
                items: {rows_json},
                nextId: {next_id},
                init() {{
                    this.items.forEach(it => {{
                        const m = this.getMachine(it.machine_id);
                        if (m && (!it.rate || it.rate === '0')) it.rate = m.rate_decimal;
                        this.recalc(it);
                    }});
                }},
                getMachine(id) {{
                    return this.machines.find(c => String(c.id) === String(id));
                }},
                addItem() {{
                    const first = this.machines.length > 0 ? this.machines[0] : null;
                    this.items.push({{
                        id: this.nextId++,
                        db_id: null,
                        machine_id: first ? first.id : 0,
                        rate: first ? first.rate_decimal : '0',
                        duration: '',
                        total: 0
                    }});
                }},
                removeItem(idx) {{
                    this.items.splice(idx, 1);
                }},
                onMachineChange(item) {{
                    const m = this.getMachine(item.machine_id);
                    if (m) item.rate = m.rate_decimal;
                    this.recalc(item);
                }},
                parseDuration(s) {{
                    s = (s || '').trim().toLowerCase();
                    if (!s) return 0;
                    let h = 0;
                    const hh = s.match(/([\d.]+)\s*h/);
                    const mm = s.match(/([\d.]+)\s*m/);
                    if (hh) h += parseFloat(hh[1]);
                    if (mm) h += parseFloat(mm[1]) / 60;
                    if (!hh && !mm) h = parseFloat(s) || 0;
                    return h;
                }},
                recalc(item) {{
                    const rate = parseFloat(item.rate) || 0;
                    item.total = Math.round(rate * this.parseDuration(item.duration) * 100) / 100;
                }},
                grandTotal() {{
                    return this.items.reduce((s, it) => s + (it.total || 0), 0);
                }},
                formatMoney(val) {{
                    return '₹ ' + (val || 0).toFixed(2);
                }},
                jsonOutput() {{
                    const valid = this.items
                        .filter(it => it.machine_id && parseInt(it.machine_id, 10) > 0 && (it.duration || '').trim() !== '')
                        .map(it => ({{
                            id: it.db_id || null,
                            machine_id: parseInt(it.machine_id, 10),
                            rate: String(it.rate || ''),
                            duration: it.duration || ''
                        }}));
                    return JSON.stringify(valid);
                }}
            }}"#
        );

        html! {
            div class="form-control mb-4 w-full" x-data=(alpine_data) {
                input type="hidden" name=(field.name) x-bind:value="jsonOutput()";

                div class="flex justify-between items-center mb-2" {
                    label class="label p-0" {
                        span class="label-text font-bold text-base" { (field.label) }
                    }
                    (PreEscaped(r#"<button type="button" class="btn btn-outline btn-xs btn-primary gap-1" @click="addItem()">"#))
                    (icon("plus", "w-3 h-3"))
                    "Add Machine Line"
                    (PreEscaped("</button>"))
                }

                (PreEscaped(r#"<template x-if="items.length === 0">"#))
                div class="p-4 text-center text-xs text-base-content/60 bg-base-100 rounded-lg border border-dashed border-base-300" {
                    "No machine lines added yet. Click \"Add Machine Line\" above to add machine time to this draft work order."
                }
                (PreEscaped("</template>"))

                (PreEscaped(r#"<template x-if="items.length > 0">"#))
                div class="overflow-x-auto border border-base-300 rounded-lg bg-base-100 shadow-sm" {
                    table class="table table-xs w-full" {
                        thead class="bg-base-200/80 text-base-content/70" {
                            tr class="text-xs" {
                                th class="w-7 text-center" { "#" }
                                th class="min-w-[180px]" { "Machine" }
                                th class="text-right w-28" { "Rate (₹/hr)" }
                                th class="w-40" { "Duration" }
                                th class="text-right w-24" { "Total (₹)" }
                                th class="w-7 text-center" { "" }
                            }
                        }
                        tbody {
                            (PreEscaped(r#"<template x-for="(item, idx) in items" :key="item.id">"#))
                            tr class="hover border-b border-base-200 last:border-none" {
                                (PreEscaped(r#"<td class="align-middle text-center opacity-60 font-mono text-xs" x-text="idx + 1"></td>"#))
                                td class="align-middle" {
                                    (PreEscaped(r#"
                                    <select class="select select-bordered select-xs w-full h-7 min-h-0 text-xs font-medium"
                                            x-model="item.machine_id"
                                            @change="onMachineChange(item)">
                                        <template x-for="m in machines" :key="m.id">
                                            <option :value="m.id" x-text="m.name"></option>
                                        </template>
                                    </select>
                                    "#))
                                }
                                td class="align-middle" {
                                    (PreEscaped(r#"<input type="number" min="0" step="any" x-model="item.rate" @input="recalc(item)" placeholder="0.00" class="input input-xs input-bordered w-full h-7 min-h-0 text-right font-mono text-xs">"#))
                                }
                                td class="align-middle" {
                                    (PreEscaped(r#"<input type="text" x-model="item.duration" @input="recalc(item)" placeholder="e.g. 2h 30m" class="input input-xs input-bordered w-full h-7 min-h-0 font-mono text-xs">"#))
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
                                td colspan="4" class="align-middle text-right font-medium py-2" { "Machine Grand Total (₹):" }
                                td class="align-middle text-right font-mono font-extrabold text-primary text-sm py-2" {
                                    (PreEscaped(r#"<span x-text="formatMoney(grandTotal())"></span>"#))
                                }
                                td class="align-middle" {}
                            }
                        }
                    }
                }
                (PreEscaped("</template>"))
            }
        }
    }
}

pub type DraftWorkOrderItemsWidget = DraftWorkOrderMaterialLinesWidget;
pub type WorkOrderItemsWidget = DraftWorkOrderMaterialLinesWidget;

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

    #[form(
        label = "Draft Work Order Material Lines",
        widget = DraftWorkOrderMaterialLinesWidget,
    )]
    pub items: Option<String>,

    #[form(
        label = "Draft Work Order Machine Lines",
        widget = DraftWorkOrderMachineLinesWidget,
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

    #[form(label = "Variables (JSON mm)", required, widget = Text, placeholder = r#"{"length": 1000}"#)]
    pub variables: String,

    #[form(label = "Quantity", required, widget = Text, placeholder = "1")]
    pub quantity: String,

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
        route = super::routes::MachineFkSelectRouteTag,
        swap_key = "fk-draft-work-order-machine-line-machine",
        display = "machine",
        placeholder = "Select machine…"
    )]
    pub machine_id: i64,

    #[form(label = "Rate (₹/hr)", required, widget = Text, placeholder = "Auto-filled from machine")]
    pub rate: String,

    #[form(label = "Duration", required, widget = Text, placeholder = "e.g. 2h 30m")]
    pub duration: String,
}

pub type WorkOrderMachineLineForm = DraftWorkOrderMachineLineForm;
pub type WorkOrderMachineLineEditForm = DraftWorkOrderMachineLineForm;
pub type WorkOrderMachineLineFormField = DraftWorkOrderMachineLineFormField;
pub type WorkOrderMachineLineEditFormField = DraftWorkOrderMachineLineFormField;
pub type DraftWorkOrderMachineLineEditForm = DraftWorkOrderMachineLineForm;
pub type DraftWorkOrderMachineLineEditFormField = DraftWorkOrderMachineLineFormField;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DraftWorkOrderMachineLineInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub machine_id: i64,
    #[serde(default)]
    pub rate: Option<serde_json::Value>,
    #[serde(default)]
    pub duration: String,
}

pub type WorkOrderMachineLineInput = DraftWorkOrderMachineLineInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftWorkOrderMaterialLineInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub component_id: i64,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub target_weight: Option<f64>,
    #[serde(default)]
    pub target_cost: Option<f64>,
    #[serde(default)]
    pub quantity: serde_json::Value,
    #[serde(default)]
    pub unit_weight: Option<serde_json::Value>,
    #[serde(default)]
    pub material_rate: Option<serde_json::Value>,
    #[serde(default)]
    pub final_cost: Option<serde_json::Value>,
    #[serde(default)]
    pub extra_data: Option<serde_json::Value>,
}

pub type DraftWorkOrderLineInput = DraftWorkOrderMaterialLineInput;

pub type WorkOrderItemInput = DraftWorkOrderLineInput;
pub type DraftWorkOrderItemInput = DraftWorkOrderLineInput;


