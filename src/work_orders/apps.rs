use super::routes::WorkOrdersDefaultRouteTag;

lariv_rs::define_register_apps! {
    plugin: super::WorkOrdersTag;
    key: "kds_tagore-quotations";
    name: "KDS Quotations";
    href: WorkOrdersDefaultRouteTag.url();
    icon: "wrench-screwdriver";
    roles: ["superuser"];
}
