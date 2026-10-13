#![recursion_limit = "1024"]

//! Deployment role `vmc_programmer`: Holidays, Attendance, Leaves, Overtime, and Tasks.
//! Same grants as `laser_programmer`: read-only holidays, punch in and punch out,
//! the programmer's own leave, and overtime.

use lariv_core::apps::{AppsCapability, AppsRegistrar};
use lariv_plugin_hr::routes::{AttendanceView, HolidayView, LeaveView, OvertimeView};
use lariv_plugin_tasks::routes::{TasksMutate, TasksView};
use lariv_plugin_users::{
    role_authorization::{RoleAuthorizationRegistrar, RoleAuthorizationRegistry},
    role_registry::{Role, RoleRegistrar, RoleRegistry},
    routes::UsersPick,
};

/// Stored name for [`VmcProgrammer`].
pub const VMC_PROGRAMMER_ROLE: &str = VmcProgrammer::NAME;

const TASKS_APP_KEY: &str = "p_tasks";
const HR_APP_KEY: &str = "p_hr";

/// Holidays, Attendance, Leaves, Overtime, and Tasks.
#[derive(Clone, Copy, Debug, Default)]
pub struct VmcProgrammer;

impl VmcProgrammer {
    pub const NAME: &'static str = "vmc_programmer";
    pub const TITLE: &'static str = "VMC Programmer";
    pub const DESCRIPTION: &'static str = "Holidays, Attendance, Leaves, Overtime, and Tasks.";
}

impl Role for VmcProgrammer {
    const NAME: &'static str = "vmc_programmer";
    const TITLE: &'static str = "VMC Programmer";
    const DESCRIPTION: &'static str = "Holidays, Attendance, Leaves, Overtime, and Tasks.";
}

pub struct VmcProgrammerRoleTag;

lariv_core::define_plugin_install! {
    plugin: VmcProgrammerRoleTag;
    steps: [
        cap_hook(lariv_plugin_users::role_registry::RoleRegistryTag, lariv_plugin_users::role_registry::RoleRegistryCap, CatalogHook),
        apps(AppsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, RoleHook),
    ]
}

/// Registers [`VmcProgrammer`] on the compile-time role catalog.
#[derive(Clone, Copy, Default)]
pub struct CatalogHook;

impl RoleRegistrar for CatalogHook {
    fn register_roles(self, registry: RoleRegistry) -> RoleRegistry {
        registry.register::<VmcProgrammer>()
    }
}

#[derive(Clone, Copy, Default)]
pub struct AppsHook;

impl AppsRegistrar for AppsHook {
    fn register_apps(self, apps: AppsCapability) -> AppsCapability {
        let apps = grant_tile(apps, HR_APP_KEY, VmcProgrammer::NAME);
        grant_tile(apps, TASKS_APP_KEY, VmcProgrammer::NAME)
    }
}

fn grant_tile(apps: AppsCapability, key: &str, role: &str) -> AppsCapability {
    let Some(mut tile) = apps.apps().iter().find(|tile| tile.key == key).cloned() else {
        return apps;
    };
    if !tile.roles.iter().any(|existing| existing == role) {
        tile.roles.push(role.into());
    }
    apps.register(tile)
}

#[derive(Clone, Copy, Default)]
pub struct RoleHook;

impl RoleAuthorizationRegistrar for RoleHook {
    fn register_roles(self, registry: RoleAuthorizationRegistry) -> RoleAuthorizationRegistry {
        registry
            .patch::<HolidayView>(allow_vmc_programmer)
            .patch::<AttendanceView>(allow_vmc_programmer)
            .patch::<LeaveView>(allow_vmc_programmer)
            .patch::<OvertimeView>(allow_vmc_programmer)
            .patch::<TasksView>(allow_vmc_programmer)
            .patch::<TasksMutate>(allow_vmc_programmer)
            // Assignee picker on task forms. Not a dashboard tile.
            .patch::<UsersPick>(allow_vmc_programmer)
    }
}

fn allow_vmc_programmer(roles: &mut Vec<String>) {
    allow_named(roles, VmcProgrammer::NAME);
}

fn allow_named(roles: &mut Vec<String>, role: &str) {
    if !roles.iter().any(|existing| existing == role) {
        roles.push(role.into());
    }
}
