use super::{handlers, keys::*};

use kds_plugin_accountant_role::Accountant;

/// Delivery challan routes. Allowlist is [`Accountant`]; superuser always passes.
pub struct DeliveryAccess;

/// Delivery preferences page. Empty allowlist: superuser only.
pub struct DeliveryPreferencesView;

/// Delivery preferences save. Empty allowlist: superuser only.
pub struct DeliveryPreferencesMutate;

lariv_core::define_plugin_routes! {
    plugin: super::DeliveryTag;
    prefix: "/dashboard";
    routes: [
        get DeliveryDefaultRouteTag, "/delivery", handlers::challan_list, fragment(DeliveryChallanTableKey), authorize(DeliveryAccess, [Accountant]);
        get DeliveryChallanCreateGetRouteTag, "/delivery/create", handlers::challan_create_get, modal, authorize(DeliveryAccess, [Accountant]);
        post DeliveryChallanCreatePostRouteTag, "/delivery/create", handlers::challan_create_post, authorize(DeliveryAccess, [Accountant]);
        get DeliveryChallanDetailRouteTag, "/delivery/{id}", handlers::challan_detail, authorize(DeliveryAccess, [Accountant]);
        get DeliveryChallanEditGetRouteTag, "/delivery/{id}/edit", handlers::challan_edit_get, modal, authorize(DeliveryAccess, [Accountant]);
        post DeliveryChallanEditPostRouteTag, "/delivery/{id}/edit", handlers::challan_edit_post, authorize(DeliveryAccess, [Accountant]);
        get DeliveryChallanDeleteGetRouteTag, "/delivery/{id}/delete", handlers::challan_delete_get, modal, authorize(DeliveryAccess, [Accountant]);
        post DeliveryChallanDeletePostRouteTag, "/delivery/{id}/delete", bare handlers::challan_delete_post, fragment(DeliveryChallanDeleteModalKey), authorize(DeliveryAccess, [Accountant]);
        get DeliveryChallanPdfModalRouteTag, "/delivery/{id}/pdf", bare handlers::challan_pdf_modal, modal, authorize(DeliveryAccess, [Accountant]);
        get DeliveryChallanPdfRouteTag, "/delivery/{id}/pdf/file", bare handlers::challan_pdf, file, authorize(DeliveryAccess, [Accountant]);
        get DeliveryPrefsGetRouteTag, "/delivery/preferences", handlers::preferences_get, authorize(DeliveryPreferencesView, []);
        post DeliveryPrefsPostRouteTag, "/delivery/preferences", handlers::preferences_post, authorize(DeliveryPreferencesMutate, []);
    ]
}
