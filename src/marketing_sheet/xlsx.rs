//! Parse and build the KDS & Tagore Marketing Sheet workbook.

use std::collections::BTreeMap;
use std::io::Cursor;

use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};
use chrono::{Datelike, NaiveDate};
use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};

pub const SHEET_NAME: &str = "Sheet1";

pub const COL_S_NO: &str = "S no";
pub const COL_LEAD_NAME: &str = "Lead Name";
pub const COL_CONTACT_NO: &str = "Contact No";
pub const COL_CUSTOMER: &str = "Customer";
pub const COL_COMPANY: &str = "Company Name";
pub const COL_LOCATION: &str = "Expected Sales Location";
pub const COL_STATUS: &str = "Current Status";
pub const COL_ORDER_DATE: &str = "Order expected date";
pub const COL_ORDER_TYPE: &str = "Order Type";

const DISCUSSION_PREFIX: &str = "discussion summary";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeadStatus {
    Active,
    Completed,
    Failed,
}

impl LeadStatus {
    pub fn as_sheet_label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
        }
    }

    pub fn parse(raw: &str) -> Self {
        let t = normalize_header(raw);
        if matches!(
            t.as_str(),
            "failed" | "fail" | "notresponding" | "lost" | "dead"
        ) {
            Self::Failed
        } else if matches!(
            t.as_str(),
            "completed" | "converted" | "won" | "closed" | "closedwon"
        ) {
            Self::Completed
        } else {
            Self::Active
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetRow {
    pub serial_no: Option<i64>,
    pub lead_name: String,
    pub contact_no: String,
    pub customer: String,
    pub company_name: String,
    pub expected_sales_location: String,
    pub status: LeadStatus,
    pub order_expected_date: Option<NaiveDate>,
    pub order_type: String,
    pub discussions: Vec<(NaiveDate, String)>,
}

impl SheetRow {
    pub fn is_empty(&self) -> bool {
        self.lead_name.is_empty()
            && self.customer.is_empty()
            && self.company_name.is_empty()
            && self.contact_no.is_empty()
    }
}

#[derive(Clone, Copy)]
enum FixedCol {
    Serial,
    LeadName,
    ContactNo,
    Customer,
    Company,
    Location,
    Status,
    OrderDate,
    OrderType,
}

fn normalize_header(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn parse_discussion_header(header: &str) -> Option<NaiveDate> {
    let trimmed = header.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with(DISCUSSION_PREFIX) {
        return None;
    }
    let start = trimmed.find('(')?;
    let end = trimmed.rfind(')')?;
    if end <= start + 1 {
        return None;
    }
    parse_sheet_date(trimmed[start + 1..end].trim())
}

pub fn parse_sheet_date(raw: &str) -> Option<NaiveDate> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(date) = chrono::NaiveDate::parse_from_str(s, "%d.%m.%y")
        .ok()
        .or_else(|| chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y").ok())
        .or_else(|| lariv_rs::datetime::parse_date(s))
    {
        return Some(date);
    }
    parse_dmy_loose(s)
}

fn parse_dmy_loose(s: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split(['.', '/', '-']).collect();
    if parts.len() != 3 {
        return None;
    }
    let day: u32 = parts[0].trim().parse().ok()?;
    let month: u32 = parts[1].trim().parse().ok()?;
    let year_raw: i32 = parts[2].trim().parse().ok()?;
    let year = if year_raw < 100 {
        2000 + year_raw
    } else {
        year_raw
    };
    NaiveDate::from_ymd_opt(year, month, day)
}

pub fn sheet_date_label(date: NaiveDate) -> String {
    format!("{}.{}.{}", date.day(), date.month(), date.format("%y"))
}

pub fn discussion_header(date: NaiveDate) -> String {
    format!("Discussion Summary({})", sheet_date_label(date))
}

fn excel_serial_to_date(n: f64) -> Option<NaiveDate> {
    if !n.is_finite() || n < 1.0 || n > 100_000.0 {
        return None;
    }
    let days = n.trunc() as i64;
    NaiveDate::from_ymd_opt(1899, 12, 30)?.checked_add_signed(chrono::TimeDelta::days(days))
}

fn format_number(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        let s = format!("{f}");
        if let Some(date) = excel_serial_to_date(f) {
            return lariv_rs::datetime::format_date(date);
        }
        s
    }
}

fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => format_number(*f),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => excel_serial_to_date(dt.as_f64())
            .map(lariv_rs::datetime::format_date)
            .unwrap_or_else(|| format_number(dt.as_f64())),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.trim().to_string(),
        Data::Error(err) => format!("{err:?}"),
    }
}

fn cell_date(cell: &Data) -> Option<NaiveDate> {
    match cell {
        Data::Empty => None,
        Data::DateTime(dt) => excel_serial_to_date(dt.as_f64()),
        Data::Float(f) => excel_serial_to_date(*f),
        Data::Int(i) => excel_serial_to_date(*i as f64),
        Data::String(s) => parse_sheet_date(s),
        Data::DateTimeIso(s) => parse_sheet_date(s),
        _ => parse_sheet_date(&cell_text(cell)),
    }
}

fn cell_i64(cell: &Data) -> Option<i64> {
    match cell {
        Data::Int(i) => Some(*i),
        Data::Float(f) if f.is_finite() && f.fract() == 0.0 => Some(*f as i64),
        Data::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>()
                    .ok()
                    .filter(|n| n.fract() == 0.0)
                    .map(|n| n as i64)
                    .or_else(|| t.parse().ok())
            }
        }
        _ => None,
    }
}

fn classify_header(header: &str) -> Option<FixedCol> {
    match normalize_header(header).as_str() {
        "sno" | "sno." | "serialno" | "srno" => Some(FixedCol::Serial),
        "leadname" => Some(FixedCol::LeadName),
        "contactno" | "contactnumber" | "phone" => Some(FixedCol::ContactNo),
        "customer" => Some(FixedCol::Customer),
        "companyname" | "company" => Some(FixedCol::Company),
        "expectedsaleslocation" | "location" => Some(FixedCol::Location),
        "currentstatus" | "status" => Some(FixedCol::Status),
        "orderexpecteddate" | "orderexpected" => Some(FixedCol::OrderDate),
        "ordertype" => Some(FixedCol::OrderType),
        _ => None,
    }
}

/// Read the first worksheet of a marketing-sheet workbook.
pub fn parse_workbook(bytes: &[u8]) -> Result<Vec<SheetRow>, String> {
    if bytes.is_empty() {
        return Err("empty file".into());
    }
    let mut workbook: Xlsx<_> =
        open_workbook_from_rs(Cursor::new(bytes)).map_err(|e| format!("open xlsx: {e}"))?;
    let name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| "workbook has no sheets".to_string())?;
    let range = workbook
        .worksheet_range(&name)
        .map_err(|e| format!("read sheet {name}: {e}"))?;
    let mut rows = range.rows();
    let Some(header_row) = rows.next() else {
        return Err("sheet has no header row".into());
    };

    let mut fixed: Vec<(usize, FixedCol)> = Vec::new();
    let mut discussions: Vec<(usize, NaiveDate)> = Vec::new();
    for (idx, cell) in header_row.iter().enumerate() {
        let header = cell_text(cell);
        if header.is_empty() {
            continue;
        }
        if let Some(col) = classify_header(&header) {
            fixed.push((idx, col));
        } else if let Some(date) = parse_discussion_header(&header) {
            discussions.push((idx, date));
        }
    }
    if !fixed
        .iter()
        .any(|(_, c)| matches!(c, FixedCol::LeadName | FixedCol::Customer | FixedCol::Company))
    {
        return Err(
            "sheet is missing Lead Name, Customer, or Company Name columns".into(),
        );
    }

    let mut out = Vec::new();
    for row in rows {
        let mut parsed = SheetRow {
            serial_no: None,
            lead_name: String::new(),
            contact_no: String::new(),
            customer: String::new(),
            company_name: String::new(),
            expected_sales_location: String::new(),
            status: LeadStatus::Active,
            order_expected_date: None,
            order_type: String::new(),
            discussions: Vec::new(),
        };
        for (idx, col) in &fixed {
            let cell = row.get(*idx).unwrap_or(&Data::Empty);
            match col {
                FixedCol::Serial => parsed.serial_no = cell_i64(cell).filter(|n| *n > 0),
                FixedCol::LeadName => parsed.lead_name = cell_text(cell),
                FixedCol::ContactNo => parsed.contact_no = cell_text(cell),
                FixedCol::Customer => parsed.customer = cell_text(cell),
                FixedCol::Company => parsed.company_name = cell_text(cell),
                FixedCol::Location => parsed.expected_sales_location = cell_text(cell),
                FixedCol::Status => parsed.status = LeadStatus::parse(&cell_text(cell)),
                FixedCol::OrderDate => parsed.order_expected_date = cell_date(cell),
                FixedCol::OrderType => parsed.order_type = cell_text(cell),
            }
        }
        for (idx, date) in &discussions {
            let text = cell_text(row.get(*idx).unwrap_or(&Data::Empty));
            if !text.is_empty() {
                parsed.discussions.push((*date, text));
            }
        }
        if parsed.is_empty() {
            continue;
        }
        out.push(parsed);
    }
    Ok(out)
}

fn sanitize_xlsx_string(s: &str) -> String {
    const EXCEL_MAX_CHARS: usize = 32_767;
    let filtered: String = s
        .chars()
        .filter(|c| {
            let u = *c as u32;
            u == 0x9 || u == 0xA || u == 0xD || (0x20..0xFFFE).contains(&u)
        })
        .collect();
    match filtered.char_indices().nth(EXCEL_MAX_CHARS) {
        Some((idx, _)) => filtered[..idx].to_string(),
        None => filtered,
    }
}

/// Build a marketing-sheet workbook from CRM-mapped rows.
pub fn build_workbook(rows: &[SheetRow]) -> Result<Vec<u8>, String> {
    let mut dates: Vec<NaiveDate> = rows
        .iter()
        .flat_map(|r| r.discussions.iter().map(|(d, _)| *d))
        .collect();
    dates.sort_unstable();
    dates.dedup();

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name(SHEET_NAME)
        .map_err(|e| e.to_string())?;
    let header_format = Format::new().set_bold();
    let date_format = Format::new().set_num_format("d.m.yy");
    let text_format = Format::new().set_num_format("@");

    let mut headers = vec![
        COL_S_NO,
        COL_LEAD_NAME,
        COL_CONTACT_NO,
        COL_CUSTOMER,
        COL_COMPANY,
        COL_LOCATION,
        COL_STATUS,
        COL_ORDER_DATE,
        COL_ORDER_TYPE,
    ];
    let discussion_headers: Vec<String> = dates.iter().copied().map(discussion_header).collect();
    for h in &discussion_headers {
        headers.push(h.as_str());
    }

    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, *header, &header_format)
            .map_err(|e| e.to_string())?;
    }

    for (row_idx, row) in rows.iter().enumerate() {
        let excel_row = (row_idx + 1) as u32;
        let notes: BTreeMap<NaiveDate, String> = row.discussions.iter().cloned().collect();
        let serial = row.serial_no.unwrap_or((row_idx + 1) as i64);
        worksheet
            .write_number(excel_row, 0, serial as f64)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(excel_row, 1, sanitize_xlsx_string(&row.lead_name))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(
                excel_row,
                2,
                sanitize_xlsx_string(&row.contact_no),
                &text_format,
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(excel_row, 3, sanitize_xlsx_string(&row.customer))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(excel_row, 4, sanitize_xlsx_string(&row.company_name))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(
                excel_row,
                5,
                sanitize_xlsx_string(&row.expected_sales_location),
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(excel_row, 6, row.status.as_sheet_label())
            .map_err(|e| e.to_string())?;
        if let Some(date) = row.order_expected_date {
            let excel = ExcelDateTime::from_ymd(date.year() as u16, date.month() as u8, date.day() as u8)
                .map_err(|e| e.to_string())?;
            worksheet
                .write_with_format(excel_row, 7, &excel, &date_format)
                .map_err(|e| e.to_string())?;
        }
        worksheet
            .write_string(excel_row, 8, sanitize_xlsx_string(&row.order_type))
            .map_err(|e| e.to_string())?;
        for (i, date) in dates.iter().enumerate() {
            if let Some(text) = notes.get(date) {
                worksheet
                    .write_string(excel_row, (9 + i) as u16, sanitize_xlsx_string(text))
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    workbook
        .save_to_buffer()
        .map_err(|e| format!("write workbook: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row() -> SheetRow {
        SheetRow {
            serial_no: Some(4),
            lead_name: "Partho".into(),
            contact_no: "9641552939".into(),
            customer: "Partho Bhattacharya".into(),
            company_name: "EPPS".into(),
            expected_sales_location: "Mumbai".into(),
            status: LeadStatus::Active,
            order_expected_date: NaiveDate::from_ymd_opt(2026, 8, 31),
            order_type: "Structure fabrication".into(),
            discussions: vec![(
                NaiveDate::from_ymd_opt(2026, 8, 17).unwrap(),
                "Quotation to be shared".into(),
            )],
        }
    }

    #[test]
    fn status_maps_sheet_and_crm_labels() {
        assert_eq!(LeadStatus::parse("Discussion wip"), LeadStatus::Active);
        assert_eq!(LeadStatus::parse("Ready to give the order"), LeadStatus::Active);
        assert_eq!(LeadStatus::parse("Not responding"), LeadStatus::Failed);
        assert_eq!(LeadStatus::parse("Completed"), LeadStatus::Completed);
        assert_eq!(LeadStatus::parse("Converted"), LeadStatus::Completed);
        assert_eq!(LeadStatus::parse("Failed"), LeadStatus::Failed);
    }

    #[test]
    fn discussion_header_roundtrip() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        assert_eq!(discussion_header(date), "Discussion Summary(17.8.26)");
        assert_eq!(parse_discussion_header("Discussion Summary(17.8.26)"), Some(date));
    }

    #[test]
    fn workbook_roundtrip_preserves_row() {
        let original = vec![sample_row()];
        let bytes = build_workbook(&original).expect("build");
        let parsed = parse_workbook(&bytes).expect("parse");
        assert_eq!(parsed, original);
    }

    #[test]
    fn parse_real_marketing_sheet_when_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("KDS & Tagore Marketing Sheet (1).xlsx");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read sheet");
        let rows = parse_workbook(&bytes).expect("parse sample");
        assert!(!rows.is_empty());
        assert_eq!(rows[0].lead_name, "Partho");
        assert_eq!(rows[0].company_name, "EPPS");
        assert_eq!(rows[0].contact_no, "9641552939");
        assert!(
            rows[0]
                .discussions
                .iter()
                .any(|(d, _)| *d == NaiveDate::from_ymd_opt(2026, 8, 17).unwrap())
        );
    }
}
