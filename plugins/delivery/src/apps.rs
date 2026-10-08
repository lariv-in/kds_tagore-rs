use super::routes::DeliveryDefaultRouteTag;

lariv_core::define_register_apps! {
    plugin: super::DeliveryTag;
    key: "kds_tagore-delivery";
    name: "Delivery";
    href: DeliveryDefaultRouteTag.url();
    icon: "truck";
    roles: [kds_plugin_accountant_role::Accountant];
}
