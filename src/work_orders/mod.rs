//! Work Orders plugin — Parametric component modeling, OpenSCAD volume & weight calculations,
//! material rates, machines, and proforma invoices.

pub mod apps;
pub mod create_modals;
pub mod crumbs;
pub mod entities;
pub mod forms;
pub mod geometry;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod seed;
pub mod state;
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

define_passthrough_cap!(
    WorkOrdersStateCap,
    WorkOrdersTag,
    WorkOrdersState
);

define_plugin_install! {
    plugin: WorkOrdersTag;
    /// Register Work Orders migrations, routes, templates, and dashboard tile.
    steps: [
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
