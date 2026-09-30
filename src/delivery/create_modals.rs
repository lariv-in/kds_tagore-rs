//! Typed [`CreateModal`] wiring for the Delivery challan create button.

use super::keys::DeliveryChallanCreateModalKey;
use super::routes::{DeliveryChallanCreateGetRouteTag, DeliveryChallanCreatePostRouteTag};

lariv_rs::impl_create_modal!(
    DeliveryChallanCreateModalKey,
    DeliveryChallanCreateGetRouteTag,
    DeliveryChallanCreatePostRouteTag,
    "delivery.DeliveryChallanForm"
);
