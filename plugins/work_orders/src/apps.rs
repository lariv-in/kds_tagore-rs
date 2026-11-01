use super::routes::WorkOrderListRouteTag;

lariv_core::define_register_apps! {
    plugin: super::WorkOrdersTag;
    key: "kds_tagore-quotations";
    name: "Work orders";
    href: WorkOrderListRouteTag.url();
    icon: "wrench-screwdriver";
    roles: [kds_plugin_accountant_role::Accountant];
}
