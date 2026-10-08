//! KDS Tagore bank details as the invoice account default, and on every invoice.

use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use sea_orm_migration::prelude::*;

/// Remittance block copied onto new invoices and written onto existing ones.
pub const KDS_BANK_ACCOUNT: &str = "\
KDS AND TAGORE PVT. LTD.
Account Number: 560005000126
IFSC Code: ICIC0005600
Bank: ICICI Bank
Branch: Pimpri Chinchwad - Tathawade
City / District: Pune
State: Maharashtra";

#[derive(DeriveMigrationName)]
pub struct Migration;

fn ph(backend: DbBackend) -> &'static str {
    match backend {
        DbBackend::Postgres => "$1",
        _ => "?",
    }
}

async fn exec(db: &impl ConnectionTrait, sql: String, value: &str) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    db.execute_raw(Statement::from_sql_and_values(
        backend,
        sql,
        [Value::String(Some(value.to_string()))],
    ))
    .await?;
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let p = ph(db.get_database_backend());
        exec(
            db,
            format!(
                "UPDATE invoice_preferences SET default_bank_account = {p}, updated_at = CURRENT_TIMESTAMP"
            ),
            KDS_BANK_ACCOUNT,
        )
        .await?;
        exec(
            db,
            format!(
                "INSERT INTO invoice_preferences (id, created_at, updated_at, default_bank_account)
                 SELECT 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, {p}
                 WHERE NOT EXISTS (SELECT 1 FROM invoice_preferences)"
            ),
            KDS_BANK_ACCOUNT,
        )
        .await?;
        for table in ["draft_invoices", "posted_invoices", "cancelled_invoices"] {
            exec(
                db,
                format!("UPDATE {table} SET bank_account = {p}"),
                KDS_BANK_ACCOUNT,
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let p = ph(db.get_database_backend());
        exec(
            db,
            format!(
                "UPDATE invoice_preferences SET default_bank_account = NULL, updated_at = CURRENT_TIMESTAMP
                 WHERE default_bank_account = {p}"
            ),
            KDS_BANK_ACCOUNT,
        )
        .await?;
        for table in ["draft_invoices", "posted_invoices", "cancelled_invoices"] {
            exec(
                db,
                format!("UPDATE {table} SET bank_account = NULL WHERE bank_account = {p}"),
                KDS_BANK_ACCOUNT,
            )
            .await?;
        }
        Ok(())
    }
}
