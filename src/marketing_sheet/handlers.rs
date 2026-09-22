use axum::{
    body::Body,
    extract::Multipart,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use lariv_rs::{
    components::{SharedChromeFolder, SlotCtx},
    html_form::{CsrfToken, HtmlForm},
    http::Cap,
    plugins::{
        crm::state::CrmState,
        users::middleware::{RequireAuth, RequireStaff},
    },
    web::{Htmx, html_built_page_or_app_layout},
};

use super::forms::ImportForm;
use super::import::{export_rows, import_rows};
use super::templates::MarketingSheetPage;
use super::xlsx;

const MAX_UPLOAD_BYTES: usize = lariv_rs::http::REQUEST_BODY_LIMIT_BYTES;

fn render_page(
    page: &MarketingSheetPage,
    htmx: &Htmx,
    chrome: &SharedChromeFolder,
    ctx: &lariv_rs::plugins::users::state::AuthContext,
) -> maud::Markup {
    html_built_page_or_app_layout(page, htmx, chrome, &SlotCtx::from_auth(ctx))
}

async fn page_from_db(
    db: &sea_orm::DatabaseConnection,
    tz: &str,
    error: String,
    result: Option<super::import::ImportReport>,
) -> MarketingSheetPage {
    let rows = export_rows(db, tz).await.unwrap_or_default();
    MarketingSheetPage {
        error,
        result,
        rows,
    }
}

pub async fn page(
    Cap(crm): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    if !ctx.user.is_superuser {
        return RedirectForbidden.into_response();
    }
    let page = page_from_db(&crm.db, &ctx.timezone, String::new(), None).await;
    render_page(&page, &htmx, &chrome, &ctx).into_response()
}

struct RedirectForbidden;

impl IntoResponse for RedirectForbidden {
    fn into_response(self) -> Response {
        axum::response::Redirect::to("/crm/leads").into_response()
    }
}

pub async fn import_post(
    Cap(crm): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    if !ctx.user.is_superuser {
        return RedirectForbidden.into_response();
    }

    let parsed_form = match ImportForm::from_multipart(multipart, &csrf).await {
        Ok(form) => form,
        Err(err) => {
            let page = page_from_db(&crm.db, &ctx.timezone, err.to_string(), None).await;
            return (
                StatusCode::BAD_REQUEST,
                render_page(&page, &htmx, &chrome, &ctx),
            )
                .into_response();
        }
    };
    let bytes = match parsed_form.file.into_bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            let page = page_from_db(&crm.db, &ctx.timezone, err.to_string(), None).await;
            return (
                StatusCode::BAD_REQUEST,
                render_page(&page, &htmx, &chrome, &ctx),
            )
                .into_response();
        }
    };
    if bytes.len() > MAX_UPLOAD_BYTES {
        let page = page_from_db(
            &crm.db,
            &ctx.timezone,
            "xlsx file too large (max 50 MiB)".into(),
            None,
        )
        .await;
        return (
            StatusCode::BAD_REQUEST,
            render_page(&page, &htmx, &chrome, &ctx),
        )
            .into_response();
    }
    if bytes.is_empty() {
        let page = page_from_db(&crm.db, &ctx.timezone, "empty file".into(), None).await;
        return (
            StatusCode::BAD_REQUEST,
            render_page(&page, &htmx, &chrome, &ctx),
        )
            .into_response();
    }

    let rows = match xlsx::parse_workbook(&bytes) {
        Ok(rows) => rows,
        Err(err) => {
            let page = page_from_db(&crm.db, &ctx.timezone, err, None).await;
            return (
                StatusCode::BAD_REQUEST,
                render_page(&page, &htmx, &chrome, &ctx),
            )
                .into_response();
        }
    };

    match import_rows(&crm.db, &ctx, &rows).await {
        Ok(report) => {
            let page = page_from_db(&crm.db, &ctx.timezone, String::new(), Some(report)).await;
            render_page(&page, &htmx, &chrome, &ctx).into_response()
        }
        Err(err) => {
            tracing::error!(error = %err, "marketing sheet import failed");
            let page = page_from_db(&crm.db, &ctx.timezone, err, None).await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                render_page(&page, &htmx, &chrome, &ctx),
            )
                .into_response()
        }
    }
}

pub async fn export_post(Cap(crm): Cap<CrmState>, RequireStaff(ctx): RequireStaff) -> Response {
    if !ctx.user.is_superuser {
        return RedirectForbidden.into_response();
    }
    let rows = match export_rows(&crm.db, &ctx.timezone).await {
        Ok(rows) => rows,
        Err(err) => {
            tracing::error!(error = %err, "marketing sheet export failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, err).into_response();
        }
    };
    let bytes = match xlsx::build_workbook(&rows) {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!(error = %err, "marketing sheet workbook failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, err).into_response();
        }
    };
    let filename = format!(
        "KDS_Tagore_Marketing_Sheet_{}.xlsx",
        Utc::now().format("%Y%m%d_%H%M%S")
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(bytes))
        .unwrap()
        .into_response()
}
