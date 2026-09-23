//! KDS Quotations plugin — Rune formula costing, draft work orders, and quotations.

pub mod apps;
pub mod cascade;
pub mod create_modals;
pub mod crumbs;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod line_vars;
pub mod migrations;
pub mod pdf;
pub mod pdf_templates;
pub mod preferences;
pub mod quotation_number;
pub mod routes;
pub mod source_docs;
pub mod state;
pub mod tax_assoc;
pub mod templates;

use frunk::{HCons, hlist::HList};
use lariv_rs::{
    app::App,
    capability::CapStore,
    db::{DbCap, DbTag},
    define_passthrough_cap, define_plugin_install,
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByCapTag,
    },
};

use state::WorkOrdersState;

/// Plugin identity tag.
pub struct WorkOrdersTag;

define_passthrough_cap!(WorkOrdersStateCap, WorkOrdersTag, WorkOrdersState);

define_plugin_install! {
    plugin: WorkOrdersTag;
    /// Register KDS Quotations migrations, routes, templates, and dashboard tile.
    steps: [
        cap_hook(crate::machinery_schedule::JobSourceDocTag, crate::machinery_schedule::JobSourceDocCap, source_docs::Hook),
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attach DB-backed plugin state after the database is connected.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<WorkOrdersTag, TagProof>,
{
    type Output = HCons<WorkOrdersStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(WorkOrdersState::new(conn)))
    }
}
