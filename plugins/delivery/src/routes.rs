use super::{handlers, keys::*};

use kds_plugin_hr_role::Hr;
use lariv_plugin_hr::roles::Employee;

/// Delivery challan routes. Allowlist is [`Hr`]; superuser always passes.
pub struct DeliveryAccess;

/// Delivery preferences page. Allowlist is [`Employee`]; [`Hr`] is excluded. Superuser always passes.
pub struct DeliveryPreferencesView;

/// Delivery preferences save. Empty allowlist: superuser only.
pub struct DeliveryPreferencesMutate;

lariv_core::define_plugin_routes! {
    plugin: super::DeliveryTag;
    prefix: "/dashboard";
    routes: [
        get DeliveryDefaultRouteTag, "/delivery", handlers::challan_list, fragment(DeliveryChallanTableKey), authorize(DeliveryAccess, [Hr, Employee]);
        get DeliveryChallanCreateGetRouteTag, "/delivery/create", handlers::challan_create_get, modal, authorize(DeliveryAccess, [Hr, Employee]);
        post DeliveryChallanCreatePostRouteTag, "/delivery/create", handlers::challan_create_post, authorize(DeliveryAccess, [Hr, Employee]);
        get DeliveryChallanDetailRouteTag, "/delivery/{id}", handlers::challan_detail, authorize(DeliveryAccess, [Hr, Employee]);
        get DeliveryChallanEditGetRouteTag, "/delivery/{id}/edit", handlers::challan_edit_get, modal, authorize(DeliveryAccess, [Hr, Employee]);
        post DeliveryChallanEditPostRouteTag, "/delivery/{id}/edit", handlers::challan_edit_post, authorize(DeliveryAccess, [Hr, Employee]);
        get DeliveryChallanDeleteGetRouteTag, "/delivery/{id}/delete", handlers::challan_delete_get, modal, authorize(DeliveryAccess, [Hr, Employee]);
        post DeliveryChallanDeletePostRouteTag, "/delivery/{id}/delete", bare handlers::challan_delete_post, fragment(DeliveryChallanDeleteModalKey), authorize(DeliveryAccess, [Hr, Employee]);
        get DeliveryChallanPdfModalRouteTag, "/delivery/{id}/pdf", bare handlers::challan_pdf_modal, modal, authorize(DeliveryAccess, [Hr, Employee]);
        get DeliveryChallanPdfRouteTag, "/delivery/{id}/pdf/file", bare handlers::challan_pdf, file, authorize(DeliveryAccess, [Hr, Employee]);
        get DeliveryPrefsGetRouteTag, "/delivery/preferences", handlers::preferences_get, authorize(DeliveryPreferencesView, [Employee]);
        post DeliveryPrefsPostRouteTag, "/delivery/preferences", handlers::preferences_post, authorize(DeliveryPreferencesMutate, []);
    ]
}
