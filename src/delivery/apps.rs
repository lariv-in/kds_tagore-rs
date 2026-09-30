use super::routes::DeliveryDefaultRouteTag;

lariv_rs::define_register_apps! {
    plugin: super::DeliveryTag;
    key: "kds_tagore-delivery";
    name: "Delivery";
    href: DeliveryDefaultRouteTag.url();
    icon: "truck";
    roles: ["superuser"];
}
