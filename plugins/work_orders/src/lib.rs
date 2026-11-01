#![feature(impl_trait_in_assoc_type)]
#![recursion_limit = "1024"]

//! Process-plan work orders: parts, steps, and recorded batches.

pub mod apps;
pub mod create_modals;
pub mod crumbs;
pub mod entities;
pub mod forms;
pub mod graph;
pub mod handlers;
pub mod keys;
pub mod line_editors;
pub mod logic;
pub mod measure;
pub mod migrations;
pub mod plan_lines;
pub mod routes;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};
use lariv_core::{
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
    /// Register work order migrations, routes, templates, and the dashboard tile.
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
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
