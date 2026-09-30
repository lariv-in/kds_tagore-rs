//! Draft quotation email (`.eml`) with the quotation PDF attached.
//!
//! `X-Unsent: 1` asks Thunderbird and Outlook to open the file as a compose
//! window, with the subject, body, recipient, and PDF already filled in.

use std::fmt::Write as _;

use chrono::Utc;
use lariv_rs::plugins::filesystem::state::FilesystemState;
use minijinja::Environment;
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use uuid::Uuid;

use super::pdf::{self, PdfError};
use super::preferences::{load_preferences, quotation_email_body, quotation_email_subject};

pub const DEFAULT_QUOTATION_EMAIL_SUBJECT: &str = "Quotation {{ invoice_number }}";

pub const DEFAULT_QUOTATION_EMAIL_BODY: &str = "\
Dear {{ customer_name }},

Please find attached our quotation {{ invoice_number }} dated {{ date }} for ₹{{ grand_total }}.

Regards,
{{ company_name }}
";

pub const QUOTATION_EMAIL_TEMPLATE_HINT: &str = "\
Jinja2 template. Placeholders: invoice_number, date, customer_name, customer_email, \
grand_total, company_name, company_phone, company_gstin, place_of_supply. \
Leave blank to use the default.";

pub struct QuotationEml {
    pub bytes: Vec<u8>,
    pub filename: String,
}

/// Build a draft message for one quotation, with its PDF attached.
pub async fn build_quotation_eml(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    id: i64,
) -> Result<QuotationEml, PdfError> {
    let parts = pdf::render_quotation_pdf_parts(db, fs, id, "UTC").await?;
    let prefs = load_preferences(db)
        .await
        .map_err(|e| PdfError::Message(e.to_string()))?;
    let ctx = mail_context(&parts.context);
    let subject = single_line(&render_text(quotation_email_subject(&prefs), &ctx)?);
    let subject = if subject.is_empty() {
        format!("Quotation {}", json_str(&ctx, "invoice_number"))
    } else {
        subject
    };
    let body = render_text(quotation_email_body(&prefs), &ctx)?;
    let to = mail_recipient(&ctx);
    let pdf_name = format!("{}.pdf", parts.pdf.filename_base);
    let filename = format!("{}.eml", parts.pdf.filename_base);
    let bytes = assemble_eml(&EmlParts {
        to: &to,
        subject: &subject,
        body: &body,
        pdf_filename: &pdf_name,
        pdf_bytes: &parts.pdf.bytes,
    });
    Ok(QuotationEml { bytes, filename })
}

fn mail_context(pdf_ctx: &Value) -> Value {
    let invoice_number = json_str_owned(pdf_ctx.get("InvoiceNumber"));
    let date = json_str_owned(pdf_ctx.get("Date"));
    let grand_total = json_str_owned(pdf_ctx.get("GrandTotal"));
    let customer_name = json_str_owned(pdf_ctx.pointer("/Customer/Name"));
    let customer_email = json_str_owned(pdf_ctx.pointer("/Customer/Email"));
    let mut ctx = pdf_ctx.clone();
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert("invoice_number".into(), json!(invoice_number));
        obj.insert("date".into(), json!(date));
        obj.insert("grand_total".into(), json!(grand_total));
        obj.insert("customer_name".into(), json!(customer_name));
        obj.insert("customer_email".into(), json!(customer_email));
    }
    ctx
}

fn json_str_owned(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn json_str(ctx: &Value, key: &str) -> String {
    json_str_owned(ctx.get(key))
}

fn render_text(src: &str, ctx: &Value) -> Result<String, PdfError> {
    let mut env = Environment::new();
    env.add_template("mail", src)
        .map_err(|e| PdfError::Message(format!("Email template: {e}")))?;
    let tmpl = env
        .get_template("mail")
        .map_err(|e| PdfError::Message(format!("Email template: {e}")))?;
    tmpl.render(ctx)
        .map_err(|e| PdfError::Message(format!("Email template: {e}")))
}

fn single_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn mail_recipient(ctx: &Value) -> String {
    let raw = json_str(ctx, "customer_email");
    let t = raw.trim();
    if t.is_empty()
        || !t.contains('@')
        || t.chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '<' | '>' | ',' | ';'))
    {
        String::new()
    } else {
        t.to_string()
    }
}

struct EmlParts<'a> {
    to: &'a str,
    subject: &'a str,
    body: &'a str,
    pdf_filename: &'a str,
    pdf_bytes: &'a [u8],
}

fn assemble_eml(parts: &EmlParts<'_>) -> Vec<u8> {
    let boundary = format!("=_kds_{}", Uuid::new_v4().simple());
    let mut msg = String::new();
    let _ = write!(
        msg,
        "Date: {}\r\n",
        Utc::now().format("%a, %d %b %Y %H:%M:%S +0000")
    );
    if !parts.to.is_empty() {
        let _ = write!(msg, "To: {}\r\n", parts.to);
    }
    let _ = write!(msg, "Subject: {}\r\n", encode_subject(parts.subject));
    let _ = write!(msg, "MIME-Version: 1.0\r\n");
    let _ = write!(msg, "X-Unsent: 1\r\n");
    let _ = write!(
        msg,
        "Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\r\n"
    );
    let _ = write!(msg, "--{boundary}\r\n");
    let _ = write!(msg, "Content-Type: text/plain; charset=\"UTF-8\"\r\n");
    let _ = write!(msg, "Content-Transfer-Encoding: base64\r\n\r\n");
    msg.push_str(&base64_lines(crlf(parts.body).as_bytes()));
    let _ = write!(msg, "--{boundary}\r\n");
    let pdf_name = quoted_filename(parts.pdf_filename);
    let _ = write!(
        msg,
        "Content-Type: application/pdf; name=\"{pdf_name}\"\r\n"
    );
    let _ = write!(msg, "Content-Transfer-Encoding: base64\r\n");
    let _ = write!(
        msg,
        "Content-Disposition: attachment; filename=\"{pdf_name}\"\r\n\r\n"
    );
    msg.push_str(&base64_lines(parts.pdf_bytes));
    let _ = write!(msg, "--{boundary}--\r\n");
    msg.into_bytes()
}

fn crlf(s: &str) -> String {
    let mut out = s.replace("\r\n", "\n").replace('\r', "\n");
    out = out.replace('\n', "\r\n");
    if !out.ends_with("\r\n") {
        out.push_str("\r\n");
    }
    out
}

fn quoted_filename(name: &str) -> String {
    name.replace(['\\', '"', '\r', '\n'], "-")
}

fn encode_subject(subject: &str) -> String {
    if subject.is_ascii() && !subject.chars().any(|c| matches!(c, '"' | '\\')) {
        return subject.to_string();
    }
    let bytes = subject.as_bytes();
    let mut out = String::new();
    // Encoded-words are limited to 75 characters. 45 raw bytes encode to 60.
    for chunk in bytes.chunks(45) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str("=?UTF-8?B?");
        out.push_str(&base64_inline(chunk));
        out.push_str("?=");
    }
    out
}

fn base64_inline(data: &[u8]) -> String {
    let mut s = base64_lines(data);
    s.retain(|c| c != '\r' && c != '\n');
    s
}

fn base64_lines(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut col = 0;
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        let chars = [
            T[((n >> 18) & 63) as usize] as char,
            T[((n >> 12) & 63) as usize] as char,
            if i + 1 < data.len() {
                T[((n >> 6) & 63) as usize] as char
            } else {
                '='
            },
            if i + 2 < data.len() {
                T[(n & 63) as usize] as char
            } else {
                '='
            },
        ];
        for c in chars {
            if col == 76 {
                out.push_str("\r\n");
                col = 0;
            }
            out.push(c);
            col += 1;
        }
        i += 3;
    }
    if col != 0 {
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_subject_renders_invoice_number() {
        let ctx = json!({"invoice_number": "QT-9", "customer_name": "Acme"});
        let rendered = render_text(DEFAULT_QUOTATION_EMAIL_SUBJECT, &ctx).unwrap();
        assert_eq!(rendered, "Quotation QT-9");
    }

    #[test]
    fn eml_is_an_unsent_draft_with_pdf() {
        let eml = assemble_eml(&EmlParts {
            to: "billing@example.com",
            subject: "Quotation QT-9",
            body: "Hello ₹",
            pdf_filename: "QT-9.pdf",
            pdf_bytes: b"%PDF-1.7",
        });
        let text = String::from_utf8(eml).unwrap();
        assert!(text.contains("X-Unsent: 1\r\n"));
        assert!(text.contains("To: billing@example.com\r\n"));
        assert!(text.contains("Subject: Quotation QT-9\r\n"));
        assert!(text.contains("filename=\"QT-9.pdf\""));
        assert!(text.contains(&base64_inline(b"%PDF-1.7")));
        assert!(text.contains(&base64_inline("Hello ₹\r\n".as_bytes())));
    }

    #[test]
    fn non_ascii_subject_is_encoded() {
        let encoded = encode_subject("Quotation ₹ 1");
        assert!(encoded.starts_with("=?UTF-8?B?"));
        assert!(!encoded.contains('₹'));
    }

    #[test]
    fn unsafe_recipient_is_omitted() {
        let ctx = json!({"customer_email": "bad\r\nBcc: x@y"});
        assert!(mail_recipient(&ctx).is_empty());
        let ctx = json!({"customer_email": "ok@example.com"});
        assert_eq!(mail_recipient(&ctx), "ok@example.com");
    }
}
