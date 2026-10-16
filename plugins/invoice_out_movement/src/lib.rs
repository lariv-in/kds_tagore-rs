#![feature(impl_trait_in_assoc_type)]
#![recursion_limit = "1024"]

//! Deployment link from a finance invoice to one inventory out movement.
//!
//! The foreign key lives on `draft_invoices`, `posted_invoices`, and
//! `cancelled_invoices`. Posting and cancelling copy it with a database
//! trigger, because the shared invoice models do not know this column.
//! Draft forms and the invoice PDF context pick it up from the invoice addon
//! registries.

pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod templates;

use async_trait::async_trait;
use lariv_plugin_finance_invoices::draft_form_addon::{
    DraftInvoiceFormAddon, register_draft_invoice_form_addon,
};
use lariv_plugin_finance_invoices::invoice_pdf_addon::{
    InvoicePdfContextAddon, register_invoice_pdf_context_addon,
};
use lariv_plugin_inventory::entities::stock_movement_out::{self, Entity as OutEntity};
use lariv_plugin_inventory::routes::MovementDetailRouteTag;
use maud::{Markup, html};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, Statement, Value,
};
use serde_json::{Value as Json, json};

use lariv_core::components::{FieldText, InputForeignKey, field_text, input_foreign_key, label};
use lariv_core::html_form::UrlencodedFields;

/// Column on the invoice header tables.
pub const COLUMN: &str = "stock_movement_out_id";

/// Form and query field name for [`COLUMN`].
pub const FIELD: &str = COLUMN;

/// PDF context key for the foreign key.
pub const PDF_ID_KEY: &str = "StockMovementOutId";

/// PDF context key for the related out movement.
pub const PDF_OBJECT_KEY: &str = "StockMovementOut";

const ADDON_ID: &str = "kds_invoice_stock_movement_out";

/// Plugin identity tag.
pub struct InvoiceOutMovementTag;

lariv_core::define_plugin_install! {
    plugin: InvoiceOutMovementTag;
    /// Add the out-movement foreign key and register it on the invoice addons.
    steps: [
        migrations(migrations::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
    ]
}

/// Register the PDF-context and draft-form addons. Idempotent.
pub fn register_addons() {
    register_invoice_pdf_context_addon(&ADDON);
    register_draft_invoice_form_addon(&ADDON);
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
        draft_invoice_id: i64,
    ) -> Result<Json, String> {
        let movement_id = load_draft_movement_id(db, draft_invoice_id).await?;
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

#[async_trait]
impl DraftInvoiceFormAddon for Addon {
    fn id(&self) -> &'static str {
        ADDON_ID
    }

    async fn render_inputs(
        &self,
        db: &DatabaseConnection,
        draft_id: Option<i64>,
        posted: Option<&UrlencodedFields>,
    ) -> Markup {
        let (value, display) = field_state(db, draft_id, posted).await;
        let url = routes::MovementOutFkSelectRouteTag.url();
        input_foreign_key(InputForeignKey {
            label: "Out movement",
            name: FIELD,
            value: &value,
            display: &display,
            placeholder: "Select out movement…",
            url: &url,
            uid: "invoice-stock-movement-out",
            ..Default::default()
        })
    }

    async fn render_detail(&self, db: &DatabaseConnection, draft_id: i64) -> Markup {
        let movement = match load_draft_movement_id(db, draft_id).await {
            Ok(Some(id)) => OutEntity::find_by_id(id).one(db).await.ok().flatten(),
            _ => None,
        };
        let body = match movement {
            Some(movement) => {
                let href = MovementDetailRouteTag::new("out".to_string(), movement.id).url();
                html! {
                    a class="link" href=(href) { (movement_label(&movement)) }
                }
            }
            None => field_text(FieldText {
                value: "—",
                classes: "",
            }),
        };
        label("Out movement", body)
    }

    async fn save(
        &self,
        db: &DatabaseConnection,
        draft_id: i64,
        fields: &UrlencodedFields,
    ) -> Result<(), String> {
        let Some(raw) = fields.get_first(FIELD) else {
            return Ok(());
        };
        let movement_id = parse_movement_id(raw)?;
        if let Some(id) = movement_id {
            let found = OutEntity::find_by_id(id)
                .one(db)
                .await
                .map_err(|err| err.to_string())?;
            if found.is_none() {
                return Err(format!("out movement {id} was not found"));
            }
        }
        write_movement_id(db, draft_id, movement_id).await
    }

    fn bulk_has_values(&self, fields: &UrlencodedFields) -> bool {
        fields
            .get_first(FIELD)
            .is_some_and(|value| !value.trim().is_empty())
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

fn movement_label(movement: &stock_movement_out::Model) -> String {
    let number = movement.number.trim();
    if number.is_empty() {
        format!("#{}", movement.id)
    } else {
        number.to_string()
    }
}

fn parse_movement_id(raw: &str) -> Result<Option<i64>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let id: i64 = raw
        .parse()
        .map_err(|_| format!("out movement id {raw:?} is not a number"))?;
    if id <= 0 {
        return Err(format!("out movement id {id} is not valid"));
    }
    Ok(Some(id))
}

async fn field_state(
    db: &DatabaseConnection,
    draft_id: Option<i64>,
    posted: Option<&UrlencodedFields>,
) -> (String, String) {
    if let Some(fields) = posted
        && let Some(raw) = fields.get_first(FIELD)
    {
        let raw = raw.trim();
        if raw.is_empty() {
            return (String::new(), String::new());
        }
        let display = match raw.parse::<i64>() {
            Ok(id) => movement_display(db, id).await,
            Err(_) => raw.to_string(),
        };
        return (raw.to_string(), display);
    }
    let Some(draft_id) = draft_id else {
        return (String::new(), String::new());
    };
    let Ok(Some(id)) = load_draft_movement_id(db, draft_id).await else {
        return (String::new(), String::new());
    };
    let display = movement_display(db, id).await;
    (id.to_string(), display)
}

async fn movement_display(db: &DatabaseConnection, id: i64) -> String {
    match OutEntity::find_by_id(id).one(db).await {
        Ok(Some(movement)) => movement_label(&movement),
        _ => format!("#{id}"),
    }
}

async fn load_draft_movement_id(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<Option<i64>, String> {
    let backend = db.get_database_backend();
    let sql = format!(
        "SELECT {COLUMN} AS {COLUMN} FROM draft_invoices WHERE id = {}",
        placeholder(backend, 1)
    );
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            sql,
            [draft_id.into()],
        ))
        .await
        .map_err(|err| err.to_string())?;
    let Some(row) = row else {
        return Ok(None);
    };
    row.try_get("", COLUMN).map_err(|err| err.to_string())
}

async fn write_movement_id(
    db: &DatabaseConnection,
    draft_id: i64,
    movement_id: Option<i64>,
) -> Result<(), String> {
    let backend = db.get_database_backend();
    let movement_value: Value = movement_id.into();
    let draft_value: Value = draft_id.into();
    exec(
        db,
        backend,
        &format!(
            "UPDATE draft_invoices SET {COLUMN} = {}, updated_at = CURRENT_TIMESTAMP WHERE id = {}",
            placeholder(backend, 1),
            placeholder(backend, 2)
        ),
        vec![movement_value.clone(), draft_value.clone()],
    )
    .await?;
    exec(
        db,
        backend,
        &format!(
            "UPDATE posted_invoices SET {COLUMN} = {}, updated_at = CURRENT_TIMESTAMP WHERE draft_invoice_id = {}",
            placeholder(backend, 1),
            placeholder(backend, 2)
        ),
        vec![movement_value.clone(), draft_value.clone()],
    )
    .await?;
    exec(
        db,
        backend,
        &format!(
            "UPDATE cancelled_invoices SET {COLUMN} = {}, updated_at = CURRENT_TIMESTAMP \
             WHERE posted_invoice_id IN (SELECT id FROM posted_invoices WHERE draft_invoice_id = {})",
            placeholder(backend, 1),
            placeholder(backend, 2)
        ),
        vec![movement_value, draft_value],
    )
    .await
}

async fn exec(
    db: &DatabaseConnection,
    backend: DatabaseBackend,
    sql: &str,
    values: Vec<Value>,
) -> Result<(), String> {
    db.execute_raw(Statement::from_sql_and_values(backend, sql, values))
        .await
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn placeholder(backend: DatabaseBackend, index: usize) -> String {
    match backend {
        DatabaseBackend::Postgres => format!("${index}"),
        _ => "?".to_string(),
    }
}
