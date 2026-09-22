//! Upsert marketing-sheet rows into CRM companies, contacts, leads, and updates.

use chrono::{NaiveDate, TimeZone, Utc};
use lariv_rs::datetime::{format_date_in_tz, parse_timezone};
use lariv_rs::plugins::contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};
use lariv_rs::plugins::crm::entities::{
    converted_lead::{self, Entity as ConvertedLeadEntity},
    failed_lead::{self, Entity as FailedLeadEntity},
    lead::{self, Entity as LeadEntity},
    lead_update::{self, Entity as LeadUpdateEntity},
};
use lariv_rs::plugins::crm::logic::lead::{LeadInput, create_lead, update_lead};
use lariv_rs::plugins::crm::logic::lead_conversion::{convert_lead, unconvert_lead};
use lariv_rs::plugins::crm::logic::lead_fail::{fail_lead, reactivate_lead};
use lariv_rs::plugins::users::entities::user::Entity as UserEntity;
use lariv_rs::plugins::users::state::AuthContext;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};

use super::xlsx::{LeadStatus, SheetRow};

#[derive(Clone, Debug, Default)]
pub struct ImportReport {
    pub created: usize,
    pub updated: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeadState {
    Active,
    Completed,
    Failed,
}

pub async fn import_rows(
    db: &DatabaseConnection,
    auth: &AuthContext,
    rows: &[SheetRow],
) -> Result<ImportReport, String> {
    let mut report = ImportReport::default();
    for (idx, row) in rows.iter().enumerate() {
        match import_one(db, auth, row).await {
            Ok((created, warnings)) => {
                if created {
                    report.created += 1;
                } else {
                    report.updated += 1;
                }
                report.warnings.extend(warnings);
            }
            Err(err) => {
                report.warnings.push(format!(
                    "Row {}: {err}",
                    row.serial_no.unwrap_or((idx + 2) as i64)
                ));
            }
        }
    }
    Ok(report)
}

async fn import_one(
    db: &DatabaseConnection,
    auth: &AuthContext,
    row: &SheetRow,
) -> Result<(bool, Vec<String>), String> {
    let company_name = first_nonempty(&[&row.company_name, &row.customer, &row.lead_name])
        .ok_or_else(|| "company, customer, and lead name are empty".to_string())?;
    let customer_name = first_nonempty(&[&row.customer, &row.company_name, &row.lead_name])
        .unwrap_or_else(|| company_name.clone());

    let company = upsert_company(db, &company_name, &row.expected_sales_location).await?;
    let contact = upsert_contact(db, company.id, &customer_name, &row.contact_no).await?;
    let mut warnings = Vec::new();
    let assigned_to_id = match_user(db, &row.lead_name).await;
    if assigned_to_id.is_none() && !row.lead_name.trim().is_empty() {
        warnings.push(format!(
            "No user matched salesperson '{}'; lead left unassigned",
            row.lead_name.trim()
        ));
    }

    let existing = match row.serial_no {
        Some(id) if id > 0 => LeadEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?,
        _ => None,
    };
    let created = existing.is_none();
    let input = LeadInput {
        contact_id: contact.id,
        source: None,
        notes: nonempty_opt(&row.order_type),
        assigned_to_id,
        order_expected_date: row.order_expected_date,
        tag_ids: match &existing {
            Some(lead) => load_tag_ids(db, lead.id).await?,
            None => Vec::new(),
        },
    };
    let lead = if let Some(existing) = existing {
        update_lead(db, existing.id, input).await?
    } else {
        create_lead(db, input).await?
    };

    apply_status(db, auth, lead.id, row.status).await?;
    sync_discussions(db, auth, lead.id, assigned_to_id, &row.discussions).await?;
    Ok((created, warnings))
}

async fn load_tag_ids(db: &DatabaseConnection, lead_id: i64) -> Result<Vec<i64>, String> {
    use lariv_rs::plugins::crm::entities::lead_tag_link::{self, Entity as LinkEntity};
    Ok(LinkEntity::find()
        .filter(lead_tag_link::Column::LeadId.eq(lead_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|link| link.lead_tag_id)
        .collect())
}

async fn upsert_company(
    db: &DatabaseConnection,
    name: &str,
    location: &str,
) -> Result<company::Model, String> {
    let existing = CompanyEntity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(found) = existing.into_iter().find(|c| names_match(&c.name, name)) {
        if !location.is_empty() && found.city.as_deref() != Some(location) {
            let mut am: company::ActiveModel = found.into();
            am.city = Set(Some(location.to_string()));
            am.updated_at = Set(Some(Utc::now()));
            return am.update(db).await.map_err(|e| e.to_string());
        }
        return Ok(found);
    }
    let now = Utc::now();
    company::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name.to_string()),
        address_line_1: Set(None),
        address_line_2: Set(None),
        city: Set(nonempty_opt(location)),
        pincode: Set(None),
        state: Set(None),
        website: Set(None),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

async fn upsert_contact(
    db: &DatabaseConnection,
    company_id: i64,
    name: &str,
    phone: &str,
) -> Result<contact::Model, String> {
    let existing = ContactEntity::find()
        .filter(contact::Column::CompanyId.eq(company_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(found) = existing.into_iter().find(|c| names_match(&c.name, name)) {
        if !phone.is_empty() && found.phone.as_deref() != Some(phone) {
            let mut am: contact::ActiveModel = found.into();
            am.phone = Set(Some(phone.to_string()));
            am.updated_at = Set(Some(now_utc()));
            return am.update(db).await.map_err(|e| e.to_string());
        }
        return Ok(found);
    }
    let now = Utc::now();
    contact::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        company_id: Set(Some(company_id)),
        name: Set(name.to_string()),
        email: Set(None),
        phone: Set(nonempty_opt(phone)),
        is_primary: Set(true),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

fn now_utc() -> chrono::DateTime<Utc> {
    Utc::now()
}

async fn match_user(db: &DatabaseConnection, name: &str) -> Option<i64> {
    let needle = name.trim();
    if needle.is_empty() {
        return None;
    }
    let users = UserEntity::find().all(db).await.unwrap_or_default();
    if let Some(exact) = users.iter().find(|u| names_match(&u.name, needle)) {
        return Some(exact.id);
    }
    let prefix = needle.split_whitespace().next().unwrap_or(needle);
    let matches: Vec<_> = users
        .iter()
        .filter(|u| {
            u.name
                .split_whitespace()
                .next()
                .is_some_and(|first| names_match(first, prefix))
        })
        .collect();
    if matches.len() == 1 {
        Some(matches[0].id)
    } else {
        None
    }
}

async fn current_state(db: &DatabaseConnection, lead_id: i64) -> Result<LeadState, String> {
    if FailedLeadEntity::find()
        .filter(failed_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Ok(LeadState::Failed);
    }
    if ConvertedLeadEntity::find()
        .filter(converted_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Ok(LeadState::Completed);
    }
    Ok(LeadState::Active)
}

async fn apply_status(
    db: &DatabaseConnection,
    auth: &AuthContext,
    lead_id: i64,
    status: LeadStatus,
) -> Result<(), String> {
    let wanted = match status {
        LeadStatus::Active => LeadState::Active,
        LeadStatus::Completed => LeadState::Completed,
        LeadStatus::Failed => LeadState::Failed,
    };
    let current = current_state(db, lead_id).await?;
    if current == wanted {
        return Ok(());
    }
    match (current, wanted) {
        (LeadState::Active, LeadState::Completed) => {
            convert_lead(db, lead_id, auth).await?;
        }
        (LeadState::Active, LeadState::Failed) => {
            fail_lead(db, lead_id, auth, None).await?;
        }
        (LeadState::Completed, LeadState::Active) => {
            let converted = ConvertedLeadEntity::find()
                .filter(converted_lead::Column::LeadId.eq(lead_id))
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "converted lead not found".to_string())?;
            unconvert_lead(db, converted.id, auth).await?;
        }
        (LeadState::Completed, LeadState::Failed) => {
            fail_lead(db, lead_id, auth, None).await?;
        }
        (LeadState::Failed, LeadState::Active) => {
            let failed = FailedLeadEntity::find()
                .filter(failed_lead::Column::LeadId.eq(lead_id))
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "failed lead not found".to_string())?;
            reactivate_lead(db, failed.id, auth).await?;
        }
        (LeadState::Failed, LeadState::Completed) => {
            let failed = FailedLeadEntity::find()
                .filter(failed_lead::Column::LeadId.eq(lead_id))
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "failed lead not found".to_string())?;
            reactivate_lead(db, failed.id, auth).await?;
            convert_lead(db, lead_id, auth).await?;
        }
        (a, b) if a == b => {}
        _ => {}
    }
    Ok(())
}

async fn sync_discussions(
    db: &DatabaseConnection,
    auth: &AuthContext,
    lead_id: i64,
    assigned_to_id: Option<i64>,
    discussions: &[(NaiveDate, String)],
) -> Result<(), String> {
    let existing = LeadUpdateEntity::find()
        .filter(lead_update::Column::LeadId.eq(lead_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let created_by = assigned_to_id.unwrap_or(auth.user.id);
    for (date, text) in discussions {
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let datetime = date_at_noon_tz(*date, &auth.timezone);
        if let Some(found) = existing
            .iter()
            .find(|u| update_date(u.datetime, &auth.timezone) == *date)
        {
            if found.description != text {
                let mut am: lead_update::ActiveModel = found.clone().into();
                am.description = Set(text.to_string());
                am.datetime = Set(datetime);
                am.updated_at = Set(Some(Utc::now()));
                am.update(db).await.map_err(|e| e.to_string())?;
            }
            continue;
        }
        let now = Utc::now();
        lead_update::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            lead_id: Set(lead_id),
            created_by_id: Set(created_by),
            datetime: Set(datetime),
            description: Set(text.to_string()),
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn date_at_noon_tz(date: NaiveDate, tz: &str) -> chrono::DateTime<Utc> {
    let zone = parse_timezone(tz);
    zone.from_local_datetime(&date.and_hms_opt(12, 0, 0).unwrap_or_default())
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| date.and_hms_opt(12, 0, 0).unwrap().and_utc())
}

fn update_date(dt: chrono::DateTime<Utc>, tz: &str) -> NaiveDate {
    lariv_rs::datetime::parse_date(&format_date_in_tz(dt, tz)).unwrap_or_else(|| dt.date_naive())
}

fn names_match(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn nonempty_opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn first_nonempty(values: &[&str]) -> Option<String> {
    values
        .iter()
        .map(|s| s.trim())
        .find(|s| !s.is_empty())
        .map(ToString::to_string)
}

pub async fn export_rows(db: &DatabaseConnection, tz: &str) -> Result<Vec<SheetRow>, String> {
    let leads = LeadEntity::find()
        .order_by_asc(lead::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mut rows = Vec::with_capacity(leads.len());
    for lead in leads {
        let contact = ContactEntity::find_by_id(lead.contact_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?;
        let company = match contact.as_ref().and_then(|c| c.company_id) {
            Some(id) => CompanyEntity::find_by_id(id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?,
            None => None,
        };
        let salesperson = match lead.assigned_to_id {
            Some(id) => UserEntity::find_by_id(id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .map(|u| u.name)
                .unwrap_or_default(),
            None => String::new(),
        };
        let status = match current_state(db, lead.id).await? {
            LeadState::Active => LeadStatus::Active,
            LeadState::Completed => LeadStatus::Completed,
            LeadState::Failed => LeadStatus::Failed,
        };
        let updates = LeadUpdateEntity::find()
            .filter(lead_update::Column::LeadId.eq(lead.id))
            .order_by_asc(lead_update::Column::Datetime)
            .all(db)
            .await
            .map_err(|e| e.to_string())?;
        let mut discussions: Vec<(NaiveDate, String)> = Vec::new();
        for update in updates {
            let date = update_date(update.datetime, tz);
            if let Some(existing) = discussions.iter_mut().find(|(d, _)| *d == date) {
                if !existing.1.is_empty() {
                    existing.1.push('\n');
                }
                existing.1.push_str(&update.description);
            } else {
                discussions.push((date, update.description));
            }
        }
        rows.push(SheetRow {
            serial_no: Some(lead.id),
            lead_name: salesperson,
            contact_no: contact
                .as_ref()
                .and_then(|c| c.phone.clone())
                .unwrap_or_default(),
            customer: contact.map(|c| c.name).unwrap_or_default(),
            company_name: company.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
            expected_sales_location: company.and_then(|c| c.city).unwrap_or_default(),
            status,
            order_expected_date: lead.order_expected_date,
            order_type: lead.notes.unwrap_or_default(),
            discussions,
        });
    }
    Ok(rows)
}
