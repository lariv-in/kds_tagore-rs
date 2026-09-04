#![recursion_limit = "512"]

use kds_tagore_rs::{machinery_schedule, website_seed};
use lariv_rs::app::App;
use lariv_rs::plugins::{
    crm, customer, dashboard, filesystem, finance_accounts, finance_creditnotes, finance_customer,
    finance_indian, finance_invoices, finance_products, finance_taxes, users, website,
};
use tracing_subscriber::EnvFilter;

#[lariv_rs::main(
    stack_size = 64 * 1024 * 1024,
    thread_name = "kds-tagore-server"
)]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("info".parse().expect("directive")),
        )
        .init();

    let app = App::new_web_app();
    let app = users::install(app);
    let app = filesystem::install(app);
    let app = machinery_schedule::install(app);
    let app = finance_accounts::install(app);
    let app = customer::install(app);
    let app = crm::install(app);
    let app = finance_customer::install(app);
    let app = finance_creditnotes::install(app);
    let app = finance_taxes::install(app);
    let app = finance_products::install(app);
    let app = finance_invoices::install(app);
    let app = finance_indian::install(app);
    let app = dashboard::install(app);
    // After dashboard so website can own `/` (CMS home) over the auth redirect.
    let app = website::install(app);
    let app = website_seed::install(app);

    let app = app.load_config("config.toml").await?;
    let app = app.mount();
    app.run_migrations().await?;
    app.run().await?;
    Ok(())
}
