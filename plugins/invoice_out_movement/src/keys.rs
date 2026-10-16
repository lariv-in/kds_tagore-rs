lariv_core::swap_key!(
    MovementOutSelectTableKey,
    "invoice-movement-out-select-table"
);
lariv_core::swap_key!(
    MovementOutSelectModalKey,
    "invoice-movement-out-select-modal"
);

lariv_core::impl_picker_modal!(MovementOutSelectModalKey, MovementOutSelectTableKey);
