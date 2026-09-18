//! Verify dashboard app tiles for the KDS Tagore plugin stack.

#![recursion_limit = "512"]

use std::path::PathBuf;

use kds_tagore_rs::{machinery_schedule, marketing_sheet, website_seed, work_orders};
use lariv_rs::app::App;
use lariv_rs::apps::AppsTag;
use lariv_rs::plugins::{
    contacts, crm, customer, dashboard, filesystem, finance_accounts, finance_creditnotes,
    finance_customer, finance_indian, finance_invoices, finance_products, finance_taxes, forms, hr,
    llm_assistant, users, website,
};
use lariv_rs::traits::get::GetByTag;

const STACK_SIZE: usize = 32 * 1024 * 1024;

const MINIMAL_DB_TOML: &str = r#"database_url = "sqlite::memory:"
[users]
adminEmail = "admin@test.local"
adminPassword = "adminadmin"
"#;

fn temp_config(body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kds-tagore-apps-{}-{}.toml",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, body).expect("write temp config");
    path
}

#[test]
fn kds_tagore_registers_forms_app_tile() {
    std::thread::Builder::new()
        .name("kds-tagore-apps".into())
        .stack_size(STACK_SIZE)
        .spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");
            rt.block_on(async {
                let app = App::new_web_app();
                let app = users::install(app);
                let app = forms::install(app);
                let app = filesystem::install(app);
                let app = llm_assistant::install(app);
                let app = machinery_schedule::install(app);
                let app = work_orders::install(app);
                let app = finance_accounts::install(app);
                let app = customer::install(app);
                let app = contacts::install(app);
                let app = crm::install(app);
                let app = hr::install(app);
                let app = marketing_sheet::install(app);
                let app = finance_customer::install(app);
                let app = finance_creditnotes::install(app);
                let app = finance_taxes::install(app);
                let app = finance_products::install(app);
                let app = finance_invoices::install(app);
                let app = finance_indian::install(app);
                let app = dashboard::install(app);
                let app = website::install(app);
                let app = website_seed::install(app);

                let path = temp_config(MINIMAL_DB_TOML);
                let app = app.load_config(&path).await.expect("load_config");
                std::fs::remove_file(&path).ok();
                let mounted = app.mount();

                let catalog = mounted.get_capability_output::<AppsTag, _>();
                let keys: Vec<_> = catalog.apps().iter().map(|t| t.key.as_str()).collect();
                assert!(
                    keys.iter().any(|k| *k == "p_forms"),
                    "expected Forms tile in apps catalog, got: {keys:?}"
                );

                let visible = catalog.visible_apps("superuser", true, true);
                let visible_keys: Vec<_> = visible.iter().map(|t| t.key.as_str()).collect();
                assert!(
                    visible_keys.iter().any(|k| *k == "p_forms"),
                    "expected Forms tile visible to superuser, got: {visible_keys:?}"
                );
            });
        })
        .expect("spawn kds-tagore-apps thread")
        .join()
        .expect("kds-tagore-apps thread");
}
