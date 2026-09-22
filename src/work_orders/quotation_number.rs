//! Quotation number formatting — same placeholders as finance invoice numbers.

use chrono::{DateTime, NaiveDate, Utc};
use lariv_rs::plugins::finance_common::fiscal_year::FiscalYear;
use lariv_rs::plugins::finance_invoices::logic::invoice_number::format_posted_invoice_number;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

use super::entities::quotation::{self, Entity as QuotationEntity};
use super::preferences::{load_preferences, quotation_number_format};

/// Default format when the preference is blank.
pub const DEFAULT_QUOTATION_NUMBER_FORMAT: &str = "QT-{{YYYY}}-{{POSTED_SEQ}}";

/// Tooltip copy for the quotation number format preference field.
pub const QUOTATION_NUMBER_FORMAT_HINT: &str = "\
Applied when creating or saving a quotation that has no quotation number (blank). \
If a number is already entered, that value is kept unchanged.

This is not a template engine — only these literal placeholders are replaced:
• {{FISCAL_CODE}} — Indian fiscal year code for the quotation date (Apr–Mar, e.g. 24-25)
• {{YY}} — two-digit year of the quotation date (e.g. 26)
• {{YYYY}} — four-digit year (e.g. 2026)
• {{POSTED_SEQ}} — next quotations row id (MAX(id)+1 among live rows), not a per-year sequence counter
• {{FISCAL_POSTED_SEQ}} — count of quotations whose date falls in the same Indian fiscal year as this quotation, plus one (resets each Apr–Mar FY)

Leave blank to default to QT-{{YYYY}}-{{POSTED_SEQ}}.
Example: QT/{{FISCAL_CODE}}/{{FISCAL_POSTED_SEQ}}";

fn quotation_datetime(date: NaiveDate) -> DateTime<Utc> {
    date.and_time(chrono::NaiveTime::MIN).and_utc()
}

pub async fn next_quotation_seq(db: &DatabaseConnection) -> Result<i64, sea_orm::DbErr> {
    let max_id = QuotationEntity::find()
        .order_by_desc(quotation::Column::Id)
        .one(db)
        .await?
        .map(|q| q.id)
        .unwrap_or(0);
    Ok(max_id + 1)
}

/// Next sequence among quotations whose date falls in the same Indian fiscal year
/// as `date` (`COUNT(*) + 1`).
pub async fn next_fiscal_quotation_seq(
    db: &DatabaseConnection,
    date: NaiveDate,
) -> Result<i64, sea_orm::DbErr> {
    let (start, end) = FiscalYear::for_datetime(quotation_datetime(date)).datetime_range();
    let count = QuotationEntity::find()
        .filter(quotation::Column::Date.gte(start.date_naive()))
        .filter(quotation::Column::Date.lt(end.date_naive()))
        .count(db)
        .await?;
    Ok(count as i64 + 1)
}

pub fn format_quotation_number(
    format: &str,
    quotation_date: NaiveDate,
    seq: i64,
    fiscal_seq: i64,
) -> String {
    let format = if format.trim().is_empty() {
        DEFAULT_QUOTATION_NUMBER_FORMAT
    } else {
        format.trim()
    };
    format_posted_invoice_number(format, quotation_datetime(quotation_date), seq, fiscal_seq)
}

/// Use `supplied` when non-blank; otherwise evaluate the preference format.
pub async fn resolve_quotation_number(
    db: &DatabaseConnection,
    supplied: &str,
    date: NaiveDate,
) -> Result<String, sea_orm::DbErr> {
    let t = supplied.trim();
    if !t.is_empty() {
        return Ok(t.to_string());
    }
    let prefs = load_preferences(db).await?;
    let seq = next_quotation_seq(db).await?;
    let fiscal_seq = next_fiscal_quotation_seq(db, date).await?;
    Ok(format_quotation_number(
        quotation_number_format(&prefs),
        date,
        seq,
        fiscal_seq,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiscal_posted_seq_placeholder() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 8).unwrap();
        assert_eq!(
            format_quotation_number("QT/{{FISCAL_CODE}}/{{FISCAL_POSTED_SEQ}}", date, 99, 7),
            "QT/25-26/7"
        );
    }

    #[test]
    fn posted_seq_unchanged_when_fiscal_placeholder_absent() {
        let date = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        assert_eq!(
            format_quotation_number("QT-{{YYYY}}-{{POSTED_SEQ}}", date, 42, 1),
            "QT-2026-42"
        );
    }

    #[test]
    fn empty_format_uses_default() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
        assert_eq!(format_quotation_number("", date, 3, 1), "QT-2026-3");
    }
}
