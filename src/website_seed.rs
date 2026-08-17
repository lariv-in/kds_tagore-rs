//! Idempotent seed for the KDS Tagore public homepage and static media.

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
use tokio::io::AsyncReadExt;

const HOMEPAGE_HTML: &str = include_str!("../assets/homepage.html");
const ROUTE_PATH: &str = "/";
const PAGE_NAME: &str = "index.html";
const THEME: &str = "p_website.kds";

struct StaticAsset {
    name: &'static str,
    bytes: &'static [u8],
}

const STATIC_ASSETS: &[StaticAsset] = &[
    StaticAsset {
        name: "logo.svg",
        bytes: include_bytes!("../assets/static/logo.svg"),
    },
    StaticAsset {
        name: "hero.jpg",
        bytes: include_bytes!("../assets/static/hero.jpg"),
    },
    StaticAsset {
        name: "laser.jpg",
        bytes: include_bytes!("../assets/static/laser.jpg"),
    },
    StaticAsset {
        name: "welding.jpg",
        bytes: include_bytes!("../assets/static/welding.jpg"),
    },
    StaticAsset {
        name: "finishing.jpg",
        bytes: include_bytes!("../assets/static/finishing.jpg"),
    },
];

pub async fn ensure_homepage(state: &WebsiteState) -> anyhow::Result<()> {
    ensure_homepage_state(&state.db, state.store.as_ref()).await
}

async fn ensure_homepage_state(
    db: &DatabaseConnection,
    store: &lariv_rs::plugins::filesystem::storage::DynFilestore,
) -> anyhow::Result<()> {
    let page = ensure_page_vnode(db, store).await?;
    ensure_db_route(db, ROUTE_PATH, page.id, THEME).await?;
    ensure_static_assets(db, store).await?;
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

    ensure_file_vnode(
        db,
        store,
        parent_id,
        parent.as_ref(),
        PAGE_NAME,
        HOMEPAGE_HTML.as_bytes(),
    )
    .await
}

async fn ensure_static_assets(
    db: &DatabaseConnection,
    store: &lariv_rs::plugins::filesystem::storage::DynFilestore,
) -> anyhow::Result<()> {
    let segments = ["website".into(), "static".into()];
    let parent_id = node::ensure_directory_path(db, store, None, &segments)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let parent = match parent_id {
        Some(id) => node::get_by_id(db, id).await.ok().flatten(),
        None => None,
    };

    for asset in STATIC_ASSETS {
        let vnode = ensure_file_vnode(
            db,
            store,
            parent_id,
            parent.as_ref(),
            asset.name,
            asset.bytes,
        )
        .await?;
        ensure_db_route(db, &format!("/static/{}", asset.name), vnode.id, "").await?;
    }
    Ok(())
}

async fn ensure_file_vnode(
    db: &DatabaseConnection,
    store: &lariv_rs::plugins::filesystem::storage::DynFilestore,
    parent_id: Option<i64>,
    parent: Option<&lariv_rs::plugins::filesystem::entities::VNode>,
    name: &str,
    bytes: &[u8],
) -> anyhow::Result<lariv_rs::plugins::filesystem::entities::VNode> {
    if let Some(existing) = node::find_child(db, parent_id, name, false).await? {
        let path = existing.file_path.as_deref().unwrap_or("");
        let mut download = store.open(path, &existing.name).await?;
        let mut current = Vec::new();
        download.reader.read_to_end(&mut current).await?;
        if current != bytes {
            render::replace_vnode_content(db, store, existing.clone(), bytes)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        return Ok(existing);
    }

    node::create(
        db,
        store,
        name.into(),
        false,
        Some(NodeFile::Bytes {
            filename: name.into(),
            data: bytes.to_vec(),
        }),
        parent,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))
}

async fn ensure_db_route(
    db: &DatabaseConnection,
    path: &str,
    page_id: i64,
    theme: &str,
) -> anyhow::Result<()> {
    if let Some(existing) = DbRouteEntity::find()
        .filter(DbRouteColumn::Path.eq(path))
        .one(db)
        .await?
    {
        let mut am: db_route::ActiveModel = existing.into();
        am.page_id = Set(page_id);
        am.is_active = Set(true);
        am.theme = Set(theme.into());
        am.updated_at = Set(Some(Utc::now()));
        am.update(db).await?;
        return Ok(());
    }

    let now = Utc::now();
    db_route::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        path: Set(path.into()),
        page_id: Set(page_id),
        is_active: Set(true),
        theme: Set(theme.into()),
        grapes_project: Set(None),
    }
    .insert(db)
    .await?;
    Ok(())
}
