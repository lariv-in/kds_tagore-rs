#![recursion_limit = "1024"]

//! Deployment role `hr`: Accounting, Quotations, Delivery Challans, Machinery Schedule, and Tasks.
//! The HR app tile stays hidden for this role.

use lariv_core::{
    apps::{AppsCapability, AppsRegistrar},
};
use lariv_plugin_customer::routes::{CustomerMutate, CustomerView};
use lariv_plugin_finance_accounts::{
            ACCOUNTING_APP_KEY,
            routes::{FinanceAccountsMutate, FinanceAccountsView},
        };
use lariv_plugin_finance_creditnotes::routes::FinanceCreditNotesView;
use lariv_plugin_finance_invoices::routes::{FinanceInvoicesMutate, FinanceInvoicesView};
use lariv_plugin_finance_products::routes::{FinanceProductsMutate, FinanceProductsView};
use lariv_plugin_finance_taxes::routes::{FinanceTaxesMutate, FinanceTaxesView};
use lariv_plugin_hr::roles::Employee;
use lariv_plugin_hr::routes::{
            ApplicantMutate, AttendanceMutate, AttendanceView, EmployeeMutate, ExEmployeeMutate,
            HolidayMutate, HolidayView, HrPeopleView, JobFormMutate, JobFormView,
        };
use lariv_plugin_tasks::routes::{TasksMutate, TasksView};
use lariv_plugin_users::{
            role_authorization::{RoleAuthorizationRegistrar, RoleAuthorizationRegistry},
            role_registry::{Role, RoleRegistrar, RoleRegistry},
            routes::UsersPick,
        };

/// Stored name for [`Hr`].
pub const HR_ROLE: &str = Hr::NAME;

const TASKS_APP_KEY: &str = "p_tasks";

/// Accounting, Quotations, Delivery Challans, Machinery Schedule, and Tasks.
#[derive(Clone, Copy, Debug, Default)]
pub struct Hr;

impl Hr {
    pub const NAME: &'static str = "hr";
    pub const TITLE: &'static str = "HR";
    pub const DESCRIPTION: &'static str =
        "Accounting, Quotations, Delivery Challans, Machinery Schedule, and Tasks.";
}

impl Role for Hr {
    const NAME: &'static str = "hr";
    const TITLE: &'static str = "HR";
    const DESCRIPTION: &'static str =
        "Accounting, Quotations, Delivery Challans, Machinery Schedule, and Tasks.";
}

pub struct HrRoleTag;

lariv_core::define_plugin_install! {
    plugin: HrRoleTag;
    steps: [
        cap_hook(lariv_plugin_users::role_registry::RoleRegistryTag, lariv_plugin_users::role_registry::RoleRegistryCap, CatalogHook),
        apps(AppsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, RoleHook),
    ]
}

/// Registers [`Hr`] on the compile-time role catalog.
#[derive(Clone, Copy, Default)]
pub struct CatalogHook;

impl RoleRegistrar for CatalogHook {
    fn register_roles(self, registry: RoleRegistry) -> RoleRegistry {
        registry.register::<Hr>()
    }
}

#[derive(Clone, Copy, Default)]
pub struct AppsHook;

impl AppsRegistrar for AppsHook {
    fn register_apps(self, apps: AppsCapability) -> AppsCapability {
        let apps = grant_tile(apps, ACCOUNTING_APP_KEY, Hr::NAME);
        let apps = grant_tile(apps, ACCOUNTING_APP_KEY, Employee::NAME);
        grant_tile(apps, TASKS_APP_KEY, Hr::NAME)
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
            .patch::<HrPeopleView>(allow_hr)
            .patch::<ApplicantMutate>(allow_hr)
            .patch::<EmployeeMutate>(allow_hr)
            .patch::<ExEmployeeMutate>(allow_hr)
            .patch::<JobFormView>(allow_hr)
            .patch::<JobFormMutate>(allow_hr)
            .patch::<HolidayView>(allow_hr)
            .patch::<HolidayMutate>(allow_hr)
            .patch::<AttendanceView>(allow_hr)
            .patch::<AttendanceMutate>(allow_hr)
            .patch::<FinanceAccountsView>(allow_hr)
            .patch::<FinanceAccountsMutate>(allow_hr)
            .patch::<FinanceAccountsView>(allow_employee)
            .patch::<FinanceAccountsMutate>(allow_employee)
            // FinanceAccountsPreferencesMutate stays superuser-only.
            .patch::<FinanceInvoicesView>(allow_hr)
            .patch::<FinanceInvoicesMutate>(allow_hr)
            .patch::<FinanceInvoicesView>(allow_employee)
            .patch::<FinanceInvoicesMutate>(allow_employee)
            .patch::<FinanceProductsView>(allow_hr)
            .patch::<FinanceProductsMutate>(allow_hr)
            .patch::<FinanceProductsView>(allow_employee)
            .patch::<FinanceProductsMutate>(allow_employee)
            .patch::<FinanceTaxesView>(allow_hr)
            .patch::<FinanceTaxesMutate>(allow_hr)
            .patch::<FinanceTaxesView>(allow_employee)
            .patch::<FinanceTaxesMutate>(allow_employee)
            .patch::<FinanceCreditNotesView>(allow_hr)
            .patch::<FinanceCreditNotesView>(allow_employee)
            .patch::<CustomerView>(allow_hr)
            .patch::<CustomerMutate>(allow_hr)
            .patch::<CustomerView>(allow_employee)
            .patch::<CustomerMutate>(allow_employee)
            .patch::<TasksView>(allow_hr)
            .patch::<TasksMutate>(allow_hr)
            .patch::<UsersPick>(allow_hr)
    }
}

fn allow_hr(roles: &mut Vec<String>) {
    allow_named(roles, Hr::NAME);
}

fn allow_employee(roles: &mut Vec<String>) {
    allow_named(roles, Employee::NAME);
}

fn allow_named(roles: &mut Vec<String>, role: &str) {
    if !roles.iter().any(|existing| existing == role) {
        roles.push(role.into());
    }
}
