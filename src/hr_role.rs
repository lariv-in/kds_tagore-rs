//! Deployment role `hr`: HR, Accounting, Quotations, Machinery Schedule, and Tasks.

use lariv_rs::{
    apps::{AppsCapability, AppsRegistrar},
    plugins::{
        customer::routes::{CustomerMutate, CustomerView},
        finance_accounts::{
            ACCOUNTING_APP_KEY,
            routes::{FinanceAccountsMutate, FinanceAccountsView},
        },
        finance_creditnotes::routes::FinanceCreditNotesView,
        finance_invoices::routes::{FinanceInvoicesMutate, FinanceInvoicesView},
        finance_products::routes::{FinanceProductsMutate, FinanceProductsView},
        finance_taxes::routes::{FinanceTaxesMutate, FinanceTaxesView},
        hr::routes::{
            ApplicantMutate, AttendanceMutate, AttendanceView, EmployeeMutate, ExEmployeeMutate,
            HolidayMutate, HolidayView, HrPeopleView, JobFormMutate, JobFormView,
        },
        tasks::routes::{TasksMutate, TasksView},
        users::{
            role_authorization::{RoleAuthorizationRegistrar, RoleAuthorizationRegistry},
            role_registry::{Role, RoleRegistrar, RoleRegistry},
            routes::UsersPick,
        },
    },
};

/// Stored name for [`Hr`].
pub const HR_ROLE: &str = Hr::NAME;

const HR_APP_KEY: &str = "p_hr";
const TASKS_APP_KEY: &str = "p_tasks";

/// HR, Accounting, Quotations, Machinery Schedule, and Tasks.
#[derive(Clone, Copy, Debug, Default)]
pub struct Hr;

impl Hr {
    pub const NAME: &'static str = "hr";
    pub const TITLE: &'static str = "HR";
    pub const DESCRIPTION: &'static str =
        "HR, Accounting, Quotations, Machinery Schedule, and Tasks.";
}

impl Role for Hr {
    const NAME: &'static str = "hr";
    const TITLE: &'static str = "HR";
    const DESCRIPTION: &'static str = "HR, Accounting, Quotations, Machinery Schedule, and Tasks.";
}

pub struct HrRoleTag;

lariv_rs::define_plugin_install! {
    plugin: HrRoleTag;
    steps: [
        cap_hook(lariv_rs::plugins::users::role_registry::RoleRegistryTag, lariv_rs::plugins::users::role_registry::RoleRegistryCap, CatalogHook),
        apps(AppsHook),
        cap_hook(lariv_rs::plugins::users::role_authorization::RoleAuthorizationTag, lariv_rs::plugins::users::role_authorization::RoleAuthorizationCap, RoleHook),
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
        let apps = grant_tile(apps, ACCOUNTING_APP_KEY);
        let apps = grant_tile(apps, HR_APP_KEY);
        grant_tile(apps, TASKS_APP_KEY)
    }
}

fn grant_tile(apps: AppsCapability, key: &str) -> AppsCapability {
    let Some(mut tile) = apps.apps().iter().find(|tile| tile.key == key).cloned() else {
        return apps;
    };
    if !tile.roles.iter().any(|role| role == Hr::NAME) {
        tile.roles.push(Hr::NAME.into());
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
            // FinanceAccountsPreferencesMutate stays superuser-only.
            .patch::<FinanceInvoicesView>(allow_hr)
            .patch::<FinanceInvoicesMutate>(allow_hr)
            .patch::<FinanceProductsView>(allow_hr)
            .patch::<FinanceProductsMutate>(allow_hr)
            .patch::<FinanceTaxesView>(allow_hr)
            .patch::<FinanceTaxesMutate>(allow_hr)
            .patch::<FinanceCreditNotesView>(allow_hr)
            .patch::<CustomerView>(allow_hr)
            .patch::<CustomerMutate>(allow_hr)
            .patch::<TasksView>(allow_hr)
            .patch::<TasksMutate>(allow_hr)
            .patch::<UsersPick>(allow_hr)
    }
}

fn allow_hr(roles: &mut Vec<String>) {
    if !roles.iter().any(|role| role == Hr::NAME) {
        roles.push(Hr::NAME.into());
    }
}
