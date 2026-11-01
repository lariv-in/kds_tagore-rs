//! Alpine line editors for the work order form.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::attrs::escape_attr;
use lariv_core::components::label::label_hint;

const PARTS_JS: &str = r#"{
lines: [],
init() {
	let parsed = [];
	try { parsed = JSON.parse(this.$el.dataset.defaults || '[]'); } catch (e) { parsed = []; }
	this.lines = Array.isArray(parsed) ? parsed : [];
	this.serialized();
},
serialized() {
	const rows = (this.lines || []).map((line) => ({
		name: line && line.name != null ? String(line.name) : '',
		measure_kind: line && line.measure_kind ? String(line.measure_kind) : 'quantity',
		measure_unit: line && line.measure_unit != null ? String(line.measure_unit) : '',
	}));
	if (window.Alpine && Alpine.store) Alpine.store('woPlanParts', rows);
	return JSON.stringify(rows);
},
add() {
	this.lines.push({ name: '', measure_kind: 'quantity', measure_unit: 'Number' });
},
remove(i) { this.lines.splice(i, 1); },
onKind(line) {
	if (line.measure_kind === 'length') line.measure_unit = 'mm';
	else if (line.measure_kind === 'weight') line.measure_unit = 'kg';
	else if (line.measure_kind === 'decimal') line.measure_unit = 'None';
	else line.measure_unit = 'Number';
}
}"#;

const SUPPLY_JS: &str = r#"{
lines: [],
init() {
	let parsed = [];
	try { parsed = JSON.parse(this.$el.dataset.defaults || '[]'); } catch (e) { parsed = []; }
	this.lines = Array.isArray(parsed) ? parsed : [];
},
serialized() {
	const rows = (this.lines || []).map((line) => ({
		part_name: line && line.part_name != null ? String(line.part_name) : '',
		qty_value: line && line.qty_value != null ? String(line.qty_value) : '',
		qty_unit: line && line.qty_unit != null ? String(line.qty_unit) : '',
	}));
	return JSON.stringify(rows);
},
partChoices() {
	const rows = window.Alpine && Alpine.store ? Alpine.store('woPlanParts') : [];
	return Array.isArray(rows) ? rows : [];
},
onPart(line) {
	const part = this.partChoices().find((row) => row.name === line.part_name);
	if (part) line.qty_unit = part.measure_unit || '';
},
add() { this.lines.push({ part_name: '', qty_value: '', qty_unit: '' }); },
remove(i) { this.lines.splice(i, 1); }
}"#;

const STEPS_JS: &str = r#"{
lines: [],
init() {
	let parsed = [];
	try { parsed = JSON.parse(this.$el.dataset.defaults || '[]'); } catch (e) { parsed = []; }
	if (!Array.isArray(parsed)) parsed = [];
	for (const line of parsed) {
		if (!Array.isArray(line.inputs)) line.inputs = [];
		if (!Array.isArray(line.outputs)) line.outputs = [];
	}
	this.lines = parsed;
},
serialized() {
	const rows = (this.lines || []).map((line) => ({
		name: line && line.name != null ? String(line.name) : '',
		planned_batches: line && line.planned_batches != null ? String(line.planned_batches) : '',
		inputs: (line.inputs || []).map((flow) => ({
			part_name: flow && flow.part_name != null ? String(flow.part_name) : '',
			qty_value: flow && flow.qty_value != null ? String(flow.qty_value) : '',
			qty_unit: flow && flow.qty_unit != null ? String(flow.qty_unit) : '',
		})),
		outputs: (line.outputs || []).map((flow) => ({
			part_name: flow && flow.part_name != null ? String(flow.part_name) : '',
			qty_value: flow && flow.qty_value != null ? String(flow.qty_value) : '',
			qty_unit: flow && flow.qty_unit != null ? String(flow.qty_unit) : '',
		})),
	}));
	return JSON.stringify(rows);
},
partChoices() {
	const rows = window.Alpine && Alpine.store ? Alpine.store('woPlanParts') : [];
	return Array.isArray(rows) ? rows : [];
},
onPart(flow) {
	const part = this.partChoices().find((row) => row.name === flow.part_name);
	if (part) flow.qty_unit = part.measure_unit || '';
},
add() { this.lines.push({ name: '', planned_batches: '1', inputs: [], outputs: [] }); },
remove(i) { this.lines.splice(i, 1); },
addFlow(list) { list.push({ part_name: '', qty_value: '', qty_unit: '' }); }
}"#;

pub fn parts_lines_editor(label: &str, hint: Option<&str>, name: &str, defaults: &str) -> Markup {
    label_hint(
        label,
        hint,
        html! {
            (open_editor(PARTS_JS, defaults))
            input type="hidden" name=(name) value=(defaults) x-bind:value="serialized()" {}
            div class="overflow-x-auto rounded-box border border-base-300 bg-base-100" {
                table class="table table-sm w-full" {
                    thead {
                        tr {
                            th { "Name" }
                            th { "Kind" }
                            th { "Unit" }
                            th class="w-24" {}
                        }
                    }
                    tbody {
                        template x-for="(line, i) in lines" x-bind:key="i" {
                            tr {
                                td {
                                    input type="text" class="input input-bordered input-sm w-full" x-model="line.name" {}
                                }
                                td {
                                    select class="select select-bordered select-sm w-full" x-model="line.measure_kind" x-on:change="onKind(line)" {
                                        option value="quantity" { "Quantity" }
                                        option value="length" { "Length" }
                                        option value="weight" { "Weight" }
                                        option value="decimal" { "Decimal" }
                                    }
                                }
                                td {
                                    input type="text" class="input input-bordered input-sm w-full" x-model="line.measure_unit" {}
                                }
                                td {
                                    button type="button" class="btn btn-ghost btn-sm" x-on:click="remove(i)" { "Remove" }
                                }
                            }
                        }
                    }
                }
            }
            button type="button" class="btn btn-outline btn-sm mt-2" x-on:click="add()" { "Add line" }
            (PreEscaped("</div>"))
        },
    )
}

pub fn presupplied_lines_editor(
    label: &str,
    hint: Option<&str>,
    name: &str,
    defaults: &str,
) -> Markup {
    label_hint(
        label,
        hint,
        html! {
            (open_editor(SUPPLY_JS, defaults))
            input type="hidden" name=(name) value=(defaults) x-bind:value="serialized()" {}
            div class="overflow-x-auto rounded-box border border-base-300 bg-base-100" {
                table class="table table-sm w-full" {
                    thead {
                        tr {
                            th { "Part" }
                            th { "Quantity" }
                            th { "Unit" }
                            th class="w-24" {}
                        }
                    }
                    tbody {
                        template x-for="(line, i) in lines" x-bind:key="i" {
                            tr {
                                td { (part_select()) }
                                td {
                                    input type="text" class="input input-bordered input-sm w-full" inputmode="decimal" x-model="line.qty_value" {}
                                }
                                td {
                                    input type="text" class="input input-bordered input-sm w-full" x-model="line.qty_unit" {}
                                }
                                td {
                                    button type="button" class="btn btn-ghost btn-sm" x-on:click="remove(i)" { "Remove" }
                                }
                            }
                        }
                    }
                }
            }
            button type="button" class="btn btn-outline btn-sm mt-2" x-on:click="add()" { "Add line" }
            (PreEscaped("</div>"))
        },
    )
}

pub fn steps_lines_editor(label: &str, hint: Option<&str>, name: &str, defaults: &str) -> Markup {
    label_hint(
        label,
        hint,
        html! {
            (open_editor(STEPS_JS, defaults))
            input type="hidden" name=(name) value=(defaults) x-bind:value="serialized()" {}
            div class="flex flex-col gap-3" {
                template x-for="(line, i) in lines" x-bind:key="i" {
                    article class="rounded-box border border-base-300 p-3 flex flex-col gap-3" {
                        div class="grid gap-2 md:grid-cols-[1fr_10rem_auto] md:items-end" {
                            label class="form-control" {
                                span class="label-text text-xs" { "Name" }
                                input type="text" class="input input-bordered input-sm w-full" x-model="line.name" {}
                            }
                            label class="form-control" {
                                span class="label-text text-xs" { "Planned batches" }
                                input type="text" class="input input-bordered input-sm w-full" inputmode="numeric" x-model="line.planned_batches" {}
                            }
                            button type="button" class="btn btn-ghost btn-sm" x-on:click="remove(i)" { "Remove" }
                        }
                        div class="grid gap-3 md:grid-cols-2" {
                            (flow_editor("Inputs per batch", "line.inputs"))
                            (flow_editor("Outputs per batch", "line.outputs"))
                        }
                    }
                }
            }
            button type="button" class="btn btn-outline btn-sm mt-2" x-on:click="add()" { "Add line" }
            (PreEscaped("</div>"))
        },
    )
}

fn flow_editor(title: &str, list: &str) -> Markup {
    let rows = format!("(flow, j) in {list}");
    let add = format!("addFlow({list})");
    let remove = format!("{list}.splice(j, 1)");
    html! {
        div class="flex flex-col gap-1" {
            h5 class="text-sm font-medium" { (title) }
            div class="overflow-x-auto rounded-box border border-base-300 bg-base-100" {
                table class="table table-sm w-full" {
                    thead {
                        tr {
                            th { "Part" }
                            th { "Qty" }
                            th { "Unit" }
                            th class="w-24" {}
                        }
                    }
                    tbody {
                        template x-for=(rows) x-bind:key="j" {
                            tr {
                                td { (part_select_flow()) }
                                td {
                                    input type="text" class="input input-bordered input-sm w-full" inputmode="decimal" x-model="flow.qty_value" {}
                                }
                                td {
                                    input type="text" class="input input-bordered input-sm w-full" x-model="flow.qty_unit" {}
                                }
                                td {
                                    button type="button" class="btn btn-ghost btn-xs" x-on:click=(remove) { "Remove" }
                                }
                            }
                        }
                    }
                }
            }
            button type="button" class="btn btn-outline btn-xs mt-1" x-on:click=(add) { "Add line" }
        }
    }
}

fn part_select() -> Markup {
    html! {
        select class="select select-bordered select-sm w-full" x-model="line.part_name" x-on:change="onPart(line)" {
            option value="" { "Select part…" }
            template x-for="(part, pi) in partChoices()" x-bind:key="pi" {
                option x-bind:value="part.name" x-text="part.name" {}
            }
        }
    }
}

fn part_select_flow() -> Markup {
    html! {
        select class="select select-bordered select-sm w-full" x-model="flow.part_name" x-on:change="onPart(flow)" {
            option value="" { "Select part…" }
            template x-for="(part, pi) in partChoices()" x-bind:key="pi" {
                option x-bind:value="part.name" x-text="part.name" {}
            }
        }
    }
}

fn open_editor(js: &str, defaults: &str) -> Markup {
    PreEscaped(format!(
        r#"<div class="w-full min-w-0" x-data="{}" x-init="init()" data-defaults="{}">"#,
        escape_attr(js),
        escape_attr(defaults),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_editor_keeps_alpine_in_attributes() {
        let html = parts_lines_editor("Parts", None, "PartsLinesJson", "[]").into_string();
        assert!(html.contains("Parts"), "{html}");
        assert!(html.contains("name=\"PartsLinesJson\""));
        assert!(html.contains("x-init="));
        assert!(html.contains(">Add line<"));
        assert!(html.contains("woPlanParts"));
        assert!(
            !lariv_core::components::attrs::alpine_js_leaked_as_text(&html),
            "Alpine JS rendered as text: {html}"
        );
    }
}
