use super::routes::JobDefaultRouteTag;

lariv_rs::define_register_apps! {
    plugin: super::MachineryScheduleTag;
    key: "kds_tagore-machinery-schedule";
    name: "Machinery Schedule";
    href: JobDefaultRouteTag.url();
    icon: "cog-6-tooth";
    roles: ["superuser"];
}
