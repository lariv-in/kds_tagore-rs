use super::{handlers, keys::*};

use kds_plugin_hr_role::Hr;

/// Delivery challan routes. Allowlist is [`Hr`]; superuser always passes.
pub struct DeliveryAccess;

/// Delivery preferences page. Empty allowlist: superuser only.
pub struct DeliveryPreferencesView;

/// Delivery preferences save. Empty allowlist: superuser only.
pub struct DeliveryPreferencesMutate;

lariv_core::define_plugin_routes! {
    plugin: super::DeliveryTag;
    prefix: "/dashboard";
    routes: [
        get DeliveryDefaultRouteTag, "/delivery", handlers::challan_list, fragment(DeliveryChallanTableKey), authorize(DeliveryAccess, [Hr]);
        get DeliveryChallanCreateGetRouteTag, "/delivery/create", handlers::challan_create_get, modal, authorize(DeliveryAccess, [Hr]);
        post DeliveryChallanCreatePostRouteTag, "/delivery/create", handlers::challan_create_post, authorize(DeliveryAccess, [Hr]);
        get DeliveryChallanDetailRouteTag, "/delivery/{id}", handlers::challan_detail, authorize(DeliveryAccess, [Hr]);
        get DeliveryChallanEditGetRouteTag, "/delivery/{id}/edit", handlers::challan_edit_get, modal, authorize(DeliveryAccess, [Hr]);
        post DeliveryChallanEditPostRouteTag, "/delivery/{id}/edit", handlers::challan_edit_post, authorize(DeliveryAccess, [Hr]);
        get DeliveryChallanDeleteGetRouteTag, "/delivery/{id}/delete", handlers::challan_delete_get, modal, authorize(DeliveryAccess, [Hr]);
        post DeliveryChallanDeletePostRouteTag, "/delivery/{id}/delete", bare handlers::challan_delete_post, fragment(DeliveryChallanDeleteModalKey), authorize(DeliveryAccess, [Hr]);
        get DeliveryChallanPdfModalRouteTag, "/delivery/{id}/pdf", bare handlers::challan_pdf_modal, modal, authorize(DeliveryAccess, [Hr]);
        get DeliveryChallanPdfRouteTag, "/delivery/{id}/pdf/file", bare handlers::challan_pdf, file, authorize(DeliveryAccess, [Hr]);
        get DeliveryPrefsGetRouteTag, "/delivery/preferences", handlers::preferences_get, authorize(DeliveryPreferencesView, []);
        post DeliveryPrefsPostRouteTag, "/delivery/preferences", handlers::preferences_post, authorize(DeliveryPreferencesMutate, []);
    ]
}
