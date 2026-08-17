//! Idempotent seed for the KDS Tagore public homepage.

use chrono::Utc;
use lariv_rs::plugins::filesystem::node::{self, NodeFile};
use lariv_rs::plugins::website::{
    entities::db_route::{self, Column as DbRouteColumn, Entity as DbRouteEntity},
    render,
    state::WebsiteState,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

const HOMEPAGE_HTML: &str = include_str!("../assets/homepage.html");
const ROUTE_PATH: &str = "/";
const PAGE_NAME: &str = "index.html";
const THEME: &str = "p_website.kds";

pub async fn ensure_homepage(state: &WebsiteState) -> anyhow::Result<()> {
    ensure_homepage_state(&state.db, state.store.as_ref()).await
}

async fn ensure_homepage_state(
    db: &DatabaseConnection,
    store: &lariv_rs::plugins::filesystem::storage::DynFilestore,
) -> anyhow::Result<()> {
    let page = ensure_page_vnode(db, store).await?;
    ensure_db_route(db, page.id).await?;
    Ok(())
}

async fn ensure_page_vnode(
    db: &DatabaseConnection,
    store: &lariv_rs::plugins::filesystem::storage::DynFilestore,
) -> anyhow::Result<lariv_rs::plugins::filesystem::entities::VNode> {
    let segments = ["website".into(), "pages".into()];
    let parent_id = node::ensure_directory_path(db, store, None, &segments)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let parent = match parent_id {
        Some(id) => node::get_by_id(db, id).await.ok().flatten(),
        None => None,
    };

    if let Some(existing) = node::find_child(db, parent_id, PAGE_NAME, false).await? {
        let path = existing.file_path.as_deref().unwrap_or("");
        let mut download = store.open(path, &existing.name).await?;
        let mut current = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut download.reader, &mut current).await?;
        if current != HOMEPAGE_HTML {
            render::replace_vnode_content(db, store, existing.clone(), HOMEPAGE_HTML.as_bytes())
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        return Ok(existing);
    }

    node::create(
        db,
        store,
        PAGE_NAME.into(),
        false,
        Some(NodeFile::Bytes {
            filename: PAGE_NAME.into(),
            data: HOMEPAGE_HTML.as_bytes().to_vec(),
        }),
        parent.as_ref(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))
}

async fn ensure_db_route(db: &DatabaseConnection, page_id: i64) -> anyhow::Result<()> {
    if let Some(existing) = DbRouteEntity::find()
        .filter(DbRouteColumn::Path.eq(ROUTE_PATH))
        .one(db)
        .await?
    {
        let mut am: db_route::ActiveModel = existing.into();
        am.page_id = Set(page_id);
        am.is_active = Set(true);
        am.theme = Set(THEME.into());
        am.updated_at = Set(Some(Utc::now()));
        am.update(db).await?;
        return Ok(());
    }

    let now = Utc::now();
    db_route::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        path: Set(ROUTE_PATH.into()),
        page_id: Set(page_id),
        is_active: Set(true),
        theme: Set(THEME.into()),
        grapes_project: Set(None),
    }
    .insert(db)
    .await?;
    Ok(())
}
