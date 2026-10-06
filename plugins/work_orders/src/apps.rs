use super::routes::WorkOrdersDefaultRouteTag;

lariv_core::define_register_apps! {
    plugin: super::WorkOrdersTag;
    key: "kds_tagore-quotations";
    name: "KDS Quotations";
    href: WorkOrdersDefaultRouteTag.url();
    icon: "wrench-screwdriver";
    roles: [kds_plugin_hr_role::Hr, lariv_plugin_hr::roles::Employee];
}
