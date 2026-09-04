//! Machinery Schedule — machines and Job / CompletedJob type-state.

pub mod apps;
pub mod create_modals;
pub mod crumbs;
pub mod duration;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod scope;
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

use state::MachineryScheduleState;

/// Plugin identity tag.
pub struct MachineryScheduleTag;

define_passthrough_cap!(
    MachineryScheduleStateCap,
    MachineryScheduleTag,
    MachineryScheduleState
);

define_plugin_install! {
    plugin: MachineryScheduleTag;
    /// Register Machinery Schedule migrations, routes, templates, and dashboard tile.
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
    L: HList + CapTagAbsent<MachineryScheduleTag, TagProof>,
{
    type Output = HCons<MachineryScheduleStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(MachineryScheduleState::new(conn)))
    }
}
