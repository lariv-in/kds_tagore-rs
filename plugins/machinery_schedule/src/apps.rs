use super::routes::JobDefaultRouteTag;

lariv_core::define_register_apps! {
    plugin: super::MachineryScheduleTag;
    key: "kds_tagore-machinery-schedule";
    name: "Machinery Schedule";
    href: JobDefaultRouteTag.url();
    icon: "cog-6-tooth";
    roles: [kds_plugin_hr_role::Hr];
}
