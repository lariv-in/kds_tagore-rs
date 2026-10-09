#![feature(impl_trait_in_assoc_type)]

//! Historical delivery-challan migrations.
//!
//! Challans are stock movements now. These migrations stay registered so
//! existing databases still recognize applied versions. The last one copies
//! challan rows into inventory and drops the delivery tables.

pub mod migrations;

/// Plugin identity tag.
pub struct DeliveryTag;

lariv_core::define_plugin_install! {
    plugin: DeliveryTag;
    /// Register the challan-to-stock-movement migrations.
    steps: [
        migrations(migrations::Hook),
    ]
}
