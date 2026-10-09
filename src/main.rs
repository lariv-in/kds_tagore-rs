#![recursion_limit = "4096"]

use kds_tagore_rs::{
    accountant_role, delivery, machinery_schedule, marketing_sheet, website_seed, work_orders,
};
use lariv_rs::app::App;
use lariv_rs::plugins::{
    contacts, crm, customer, dashboard, documents, filesystem, finance_accounts,
    finance_creditnotes, finance_indian, finance_invoices, finance_products,
    finance_taxes, forms, hr, inventory, llm_assistant, otp, pwa, signing, tasks, users, website,
};
use tracing_subscriber::EnvFilter;

#[lariv_rs::main(
    stack_size = 64 * 1024 * 1024,
    thread_name = "kds-tagore-server",
    flavor = "multi_thread",
)]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("info".parse().expect("directive"))
                .add_directive("llm_assistant::imap=info".parse().expect("directive")),
        )
        .init();

    let app = App::new_web_app();
    let app = users::install(app);
    let app = otp::install(app);
    // Before forms: form appearance references filesystem_nodes.
    let app = filesystem::install(app);
    let app = forms::install(app);
    let app = llm_assistant::install(app);
    let app = machinery_schedule::install(app);
    let app = finance_accounts::install(app);
    let app = customer::install(app);
    let app = contacts::install(app);
    // Before CRM so `tasks` can copy `crm_tasks` before CRM drops those tables.
    let app = tasks::install(app);
    let app = crm::install(app);
    // After CRM: inventory stocks reference `crm_companies`.
    let app = inventory::install(app);
    let app = marketing_sheet::install(app);
    let app = finance_creditnotes::install(app);
    let app = finance_taxes::install(app);
    // After finance_taxes: work-order line-tax tables reference `taxes`.
    let app = work_orders::install(app);
    let app = finance_products::install(app);
    // Before invoices drop `customers`. Challans reuse an existing company or
    // contact for each customer, or create one, and leave `legacy_customer_id`
    // so invoices point at that same party.
    let app = delivery::install(app);
    let app = finance_invoices::install(app);
    let app = finance_indian::install(app);
    let app = dashboard::install(app);
    let app = hr::install(app);
    // After dashboard so website can own `/` (CMS home) over the auth redirect.
    let app = website::install(app);
    let app = pwa::install(app);
    let app = website_seed::install(app);
    let app = documents::install(app);
    let app = signing::install(app);
    // After the apps it patches, so `accountant` is appended to allowlists those plugins already registered.
    let app = accountant_role::install(app);

    let app = app.load_config("config.toml").await?;
    let app = app.mount();
    app.run_migrations().await?;
    app.run().await?;
    Ok(())
}
