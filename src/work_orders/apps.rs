use super::routes::WorkOrdersDefaultRouteTag;

lariv_rs::define_register_apps! {
    plugin: super::WorkOrdersTag;
    key: "kds_tagore-work-orders";
    name: "Work Orders";
    href: WorkOrdersDefaultRouteTag.url();
    icon: "wrench-screwdriver";
    roles: ["superuser"];
}
