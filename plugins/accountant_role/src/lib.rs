#![feature(impl_trait_in_assoc_type)]
#![recursion_limit = "1024"]

//! Deployment role `accountant`: Accounting, Quotations, Machinery Schedule, Tasks, and HR.
//! HR is read-only holidays, punch in and punch out, the accountant's own leave, and overtime.

use lariv_core::apps::{AppsCapability, AppsRegistrar};
use lariv_plugin_contacts::routes::{ContactsMutate, ContactsView};
use lariv_plugin_finance_accounts::{
    ACCOUNTING_APP_KEY,
    routes::{FinanceAccountsMutate, FinanceAccountsView},
};
use lariv_plugin_finance_creditnotes::routes::FinanceCreditNotesView;
use lariv_plugin_finance_invoices::routes::{FinanceInvoicesMutate, FinanceInvoicesView};
use lariv_plugin_finance_products::routes::{FinanceProductsMutate, FinanceProductsView};
use lariv_plugin_finance_taxes::routes::{FinanceTaxesMutate, FinanceTaxesView};
use lariv_plugin_hr::routes::{AttendanceView, HolidayView, LeaveView, OvertimeView};
use lariv_plugin_tasks::routes::{TasksMutate, TasksView};
use lariv_plugin_users::{
    role_authorization::{RoleAuthorizationRegistrar, RoleAuthorizationRegistry},
    role_registry::{Role, RoleRegistrar, RoleRegistry},
    routes::UsersPick,
};

/// Stored name for [`Accountant`].
pub const ACCOUNTANT_ROLE: &str = Accountant::NAME;

const TASKS_APP_KEY: &str = "p_tasks";
const HR_APP_KEY: &str = "p_hr";

/// Accounting, Quotations, Machinery Schedule, Tasks, and HR.
#[derive(Clone, Copy, Debug, Default)]
pub struct Accountant;

impl Accountant {
    pub const NAME: &'static str = "accountant";
    pub const TITLE: &'static str = "Accountant";
    pub const DESCRIPTION: &'static str =
        "Accounting, Quotations, Machinery Schedule, Tasks, and HR.";
}

impl Role for Accountant {
    const NAME: &'static str = "accountant";
    const TITLE: &'static str = "Accountant";
    const DESCRIPTION: &'static str =
        "Accounting, Quotations, Machinery Schedule, Tasks, and HR.";
}

pub mod migrations;

pub struct AccountantRoleTag;

lariv_core::define_plugin_install! {
    plugin: AccountantRoleTag;
    steps: [
        migrations(migrations::Hook),
        cap_hook(lariv_plugin_users::role_registry::RoleRegistryTag, lariv_plugin_users::role_registry::RoleRegistryCap, CatalogHook),
        apps(AppsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, RoleHook),
    ]
}

/// Registers [`Accountant`] on the compile-time role catalog.
#[derive(Clone, Copy, Default)]
pub struct CatalogHook;

impl RoleRegistrar for CatalogHook {
    fn register_roles(self, registry: RoleRegistry) -> RoleRegistry {
        registry.register::<Accountant>()
    }
}

#[derive(Clone, Copy, Default)]
pub struct AppsHook;

impl AppsRegistrar for AppsHook {
    fn register_apps(self, apps: AppsCapability) -> AppsCapability {
        let apps = grant_tile(apps, ACCOUNTING_APP_KEY, Accountant::NAME);
        let apps = grant_tile(apps, TASKS_APP_KEY, Accountant::NAME);
        grant_tile(apps, HR_APP_KEY, Accountant::NAME)
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
            .patch::<HolidayView>(allow_accountant)
            .patch::<AttendanceView>(allow_accountant)
            .patch::<LeaveView>(allow_accountant)
            .patch::<OvertimeView>(allow_accountant)
            .patch::<FinanceAccountsView>(allow_accountant)
            .patch::<FinanceAccountsMutate>(allow_accountant)
            // FinanceAccountsPreferencesMutate stays superuser-only.
            .patch::<FinanceInvoicesView>(allow_accountant)
            .patch::<FinanceInvoicesMutate>(allow_accountant)
            .patch::<FinanceProductsView>(allow_accountant)
            .patch::<FinanceProductsMutate>(allow_accountant)
            .patch::<FinanceTaxesView>(allow_accountant)
            .patch::<FinanceTaxesMutate>(allow_accountant)
            .patch::<FinanceCreditNotesView>(allow_accountant)
            .patch::<ContactsView>(allow_accountant)
            .patch::<ContactsMutate>(allow_accountant)
            .patch::<TasksView>(allow_accountant)
            .patch::<TasksMutate>(allow_accountant)
            .patch::<UsersPick>(allow_accountant)
    }
}

fn allow_accountant(roles: &mut Vec<String>) {
    allow_named(roles, Accountant::NAME);
}

fn allow_named(roles: &mut Vec<String>, role: &str) {
    if !roles.iter().any(|existing| existing == role) {
        roles.push(role.into());
    }
}
