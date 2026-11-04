#![feature(impl_trait_in_assoc_type)]
#![recursion_limit = "1024"]

//! Deployment link from a posted finance invoice to one inventory out movement.
//!
//! The foreign key lives on `posted_invoices` and `cancelled_invoices`.
//! Cancelling copies posted → cancelled with a database trigger. Quotations and
//! draft invoices do not store an out movement.

pub mod migrations;

use async_trait::async_trait;
use lariv_plugin_finance_invoices::invoice_addon_key::InvoiceAddonKey;
use lariv_plugin_finance_invoices::invoice_pdf_addon::{
    InvoicePdfContextAddon, register_invoice_pdf_context_addon,
};
use lariv_plugin_inventory::entities::stock_movement_out::{self, Entity as OutEntity};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, Statement};
use serde_json::{Value as Json, json};

/// Column on the posted and cancelled invoice header tables.
pub const COLUMN: &str = "stock_movement_out_id";

/// PDF context key for the foreign key.
pub const PDF_ID_KEY: &str = "StockMovementOutId";

/// PDF context key for the related out movement.
pub const PDF_OBJECT_KEY: &str = "StockMovementOut";

const ADDON_ID: &str = "kds_invoice_stock_movement_out";

/// Plugin identity tag.
pub struct InvoiceOutMovementTag;

lariv_core::define_plugin_install! {
    plugin: InvoiceOutMovementTag;
    /// Add the out-movement foreign key and register it on the invoice PDF addon.
    steps: [
        migrations(migrations::Hook),
    ]
}

/// Register the PDF-context addon. Idempotent.
pub fn register_addons() {
    register_invoice_pdf_context_addon(&ADDON);
}

struct Addon;

static ADDON: Addon = Addon;

#[async_trait]
impl InvoicePdfContextAddon for Addon {
    fn id(&self) -> &'static str {
        ADDON_ID
    }

    async fn extra_context(
        &self,
        db: &DatabaseConnection,
        key: InvoiceAddonKey,
    ) -> Result<Json, String> {
        let movement_id = match key {
            InvoiceAddonKey::Quotation(_) | InvoiceAddonKey::Draft(_) => None,
            InvoiceAddonKey::Posted(id) => {
                load_header_movement_id(db, "posted_invoices", id).await?
            }
        };
        let movement = match movement_id {
            Some(id) => OutEntity::find_by_id(id)
                .one(db)
                .await
                .map_err(|err| err.to_string())?,
            None => None,
        };
        Ok(context_value(movement_id, movement.as_ref()))
    }

    fn sample_extra_context(&self) -> Json {
        json!({
            PDF_ID_KEY: 1,
            PDF_OBJECT_KEY: {
                "ID": 1,
                "Number": "OUT-1",
                "Datetime": "2026-04-01T10:00:00Z",
                "VehicleType": "Truck",
                "VehicleNumber": "MH12AB1234",
                "EwayBill": "123456789012"
            }
        })
    }
}

fn context_value(movement_id: Option<i64>, movement: Option<&stock_movement_out::Model>) -> Json {
    let object = movement.map(|movement| {
        json!({
            "ID": movement.id,
            "Number": movement.number,
            "Datetime": movement.datetime.to_rfc3339(),
            "VehicleType": movement.vehicle_type,
            "VehicleNumber": movement.vehicle_number,
            "EwayBill": movement.eway_bill,
        })
    });
    json!({
        PDF_ID_KEY: movement_id,
        PDF_OBJECT_KEY: object,
    })
}

fn assert_header_table(table: &str) {
    assert!(
        matches!(table, "posted_invoices" | "cancelled_invoices"),
        "unexpected invoice header table"
    );
}

async fn load_header_movement_id<C: ConnectionTrait>(
    db: &C,
    table: &str,
    id: i64,
) -> Result<Option<i64>, String> {
    assert_header_table(table);
    let backend = db.get_database_backend();
    let sql = format!(
        "SELECT {COLUMN} AS {COLUMN} FROM {table} WHERE id = {}",
        placeholder(backend, 1)
    );
    let row = db
        .query_one_raw(Statement::from_sql_and_values(backend, sql, [id.into()]))
        .await
        .map_err(|err| err.to_string())?;
    let Some(row) = row else {
        return Ok(None);
    };
    row.try_get("", COLUMN).map_err(|err| err.to_string())
}

fn placeholder(backend: DatabaseBackend, index: usize) -> String {
    match backend {
        DatabaseBackend::Postgres => format!("${index}"),
        _ => "?".to_string(),
    }
}
