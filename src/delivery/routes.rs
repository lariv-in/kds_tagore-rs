use super::{handlers, keys::*};

lariv_rs::define_plugin_routes! {
    plugin: super::DeliveryTag;
    prefix: "/dashboard";
    routes: [
        get DeliveryDefaultRouteTag, "/delivery", handlers::challan_list, fragment(DeliveryChallanTableKey);
        get DeliveryChallanCreateGetRouteTag, "/delivery/create", handlers::challan_create_get, modal;
        post DeliveryChallanCreatePostRouteTag, "/delivery/create", handlers::challan_create_post;
        get DeliveryChallanDetailRouteTag, "/delivery/{id}", handlers::challan_detail;
        get DeliveryChallanEditGetRouteTag, "/delivery/{id}/edit", handlers::challan_edit_get, modal;
        post DeliveryChallanEditPostRouteTag, "/delivery/{id}/edit", handlers::challan_edit_post;
        get DeliveryChallanDeleteGetRouteTag, "/delivery/{id}/delete", handlers::challan_delete_get, modal;
        post DeliveryChallanDeletePostRouteTag, "/delivery/{id}/delete", bare handlers::challan_delete_post, fragment(DeliveryChallanDeleteModalKey);
        get DeliveryChallanPdfModalRouteTag, "/delivery/{id}/pdf", bare handlers::challan_pdf_modal, modal;
        get DeliveryChallanPdfRouteTag, "/delivery/{id}/pdf/file", bare handlers::challan_pdf, file;
        get DeliveryPrefsGetRouteTag, "/delivery/preferences", handlers::preferences_get;
        post DeliveryPrefsPostRouteTag, "/delivery/preferences", handlers::preferences_post;
    ]
}
