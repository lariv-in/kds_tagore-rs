#![allow(
    clippy::redundant_field_names,
    reason = "define_plugin_routes expands path params as field: field"
)]

use super::{handlers, keys::*};
use kds_plugin_accountant_role::Accountant;

/// Work order screens. Allowlist is [`Accountant`]; superuser always passes.
pub struct WorkOrdersAccess;

lariv_core::define_plugin_routes! {
    plugin: super::WorkOrdersTag;
    prefix: "/dashboard";
    routes: [
        get WorkOrderListRouteTag, "/work-orders", handlers::list, fragment(WorkOrderTableKey), authorize(WorkOrdersAccess, [Accountant]);
        get WorkOrderCreateGetRouteTag, "/work-orders/create", handlers::create_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post WorkOrderCreatePostRouteTag, "/work-orders/create", handlers::create_post, authorize(WorkOrdersAccess, [Accountant]);
        get WorkOrderDetailRouteTag, "/work-orders/{id}", handlers::detail, authorize(WorkOrdersAccess, [Accountant]);
        get WorkOrderEditGetRouteTag, "/work-orders/{id}/edit", handlers::edit_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post WorkOrderEditPostRouteTag, "/work-orders/{id}/edit", handlers::edit_post, authorize(WorkOrdersAccess, [Accountant]);
        get WorkOrderDeleteGetRouteTag, "/work-orders/{id}/delete", handlers::delete_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post WorkOrderDeletePostRouteTag, "/work-orders/{id}/delete", bare handlers::delete_post, fragment(WorkOrderDeleteModalKey), authorize(WorkOrdersAccess, [Accountant]);

        get PartCreateGetRouteTag, "/work-orders/{id}/parts/create", handlers::part_create_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post PartCreatePostRouteTag, "/work-orders/{id}/parts/create", handlers::part_create_post, authorize(WorkOrdersAccess, [Accountant]);
        get PartEditGetRouteTag, "/work-orders/parts/{id}/edit", handlers::part_edit_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post PartEditPostRouteTag, "/work-orders/parts/{id}/edit", handlers::part_edit_post, authorize(WorkOrdersAccess, [Accountant]);
        get PartDeleteGetRouteTag, "/work-orders/parts/{id}/delete", handlers::part_delete_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post PartDeletePostRouteTag, "/work-orders/parts/{id}/delete", bare handlers::part_delete_post, fragment(PartDeleteModalKey), authorize(WorkOrdersAccess, [Accountant]);

        get PresuppliedCreateGetRouteTag, "/work-orders/{id}/presupplied/create", handlers::presupplied_create_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post PresuppliedCreatePostRouteTag, "/work-orders/{id}/presupplied/create", handlers::presupplied_create_post, authorize(WorkOrdersAccess, [Accountant]);
        get PresuppliedEditGetRouteTag, "/work-orders/presupplied/{id}/edit", handlers::presupplied_edit_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post PresuppliedEditPostRouteTag, "/work-orders/presupplied/{id}/edit", handlers::presupplied_edit_post, authorize(WorkOrdersAccess, [Accountant]);
        get PresuppliedDeleteGetRouteTag, "/work-orders/presupplied/{id}/delete", handlers::presupplied_delete_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post PresuppliedDeletePostRouteTag, "/work-orders/presupplied/{id}/delete", bare handlers::presupplied_delete_post, fragment(PresuppliedDeleteModalKey), authorize(WorkOrdersAccess, [Accountant]);

        get StepCreateGetRouteTag, "/work-orders/{id}/steps/create", handlers::step_create_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post StepCreatePostRouteTag, "/work-orders/{id}/steps/create", handlers::step_create_post, authorize(WorkOrdersAccess, [Accountant]);
        get StepEditGetRouteTag, "/work-orders/steps/{id}/edit", handlers::step_edit_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post StepEditPostRouteTag, "/work-orders/steps/{id}/edit", handlers::step_edit_post, authorize(WorkOrdersAccess, [Accountant]);
        get StepDeleteGetRouteTag, "/work-orders/steps/{id}/delete", handlers::step_delete_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post StepDeletePostRouteTag, "/work-orders/steps/{id}/delete", bare handlers::step_delete_post, fragment(StepDeleteModalKey), authorize(WorkOrdersAccess, [Accountant]);

        get InputCreateGetRouteTag, "/work-orders/steps/{id}/inputs/create", handlers::input_create_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post InputCreatePostRouteTag, "/work-orders/steps/{id}/inputs/create", handlers::input_create_post, authorize(WorkOrdersAccess, [Accountant]);
        get InputEditGetRouteTag, "/work-orders/inputs/{id}/edit", handlers::input_edit_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post InputEditPostRouteTag, "/work-orders/inputs/{id}/edit", handlers::input_edit_post, authorize(WorkOrdersAccess, [Accountant]);
        get InputDeleteGetRouteTag, "/work-orders/inputs/{id}/delete", handlers::input_delete_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post InputDeletePostRouteTag, "/work-orders/inputs/{id}/delete", bare handlers::input_delete_post, fragment(FlowDeleteModalKey), authorize(WorkOrdersAccess, [Accountant]);

        get OutputCreateGetRouteTag, "/work-orders/steps/{id}/outputs/create", handlers::output_create_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post OutputCreatePostRouteTag, "/work-orders/steps/{id}/outputs/create", handlers::output_create_post, authorize(WorkOrdersAccess, [Accountant]);
        get OutputEditGetRouteTag, "/work-orders/outputs/{id}/edit", handlers::output_edit_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post OutputEditPostRouteTag, "/work-orders/outputs/{id}/edit", handlers::output_edit_post, authorize(WorkOrdersAccess, [Accountant]);
        get OutputDeleteGetRouteTag, "/work-orders/outputs/{id}/delete", handlers::output_delete_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post OutputDeletePostRouteTag, "/work-orders/outputs/{id}/delete", bare handlers::output_delete_post, fragment(FlowDeleteModalKey), authorize(WorkOrdersAccess, [Accountant]);

        get CompleteGetRouteTag, "/work-orders/steps/{id}/complete", handlers::complete_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post CompletePostRouteTag, "/work-orders/steps/{id}/complete", handlers::complete_post, authorize(WorkOrdersAccess, [Accountant]);
        get CompletionDeleteGetRouteTag, "/work-orders/completed/{id}/delete", handlers::completion_delete_get, modal, authorize(WorkOrdersAccess, [Accountant]);
        post CompletionDeletePostRouteTag, "/work-orders/completed/{id}/delete", bare handlers::completion_delete_post, fragment(CompletionDeleteModalKey), authorize(WorkOrdersAccess, [Accountant]);
    ]
}
