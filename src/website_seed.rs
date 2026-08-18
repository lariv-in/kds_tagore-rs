//! Idempotent seed for the KDS Tagore public homepage and static media.

use chrono::Utc;
use lariv_rs::plugins::filesystem::node::{self, NodeFile};
use lariv_rs::plugins::filesystem::storage::DynFilestore;
use lariv_rs::plugins::website::{
    builder_assets::public_asset_url,
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
    store: &DynFilestore,
) -> anyhow::Result<()> {
    let media_urls = ensure_static_assets(db, store).await?;
    let html = homepage_html_with_media_urls(&media_urls);
    let page = ensure_page_vnode(db, store, html.as_bytes()).await?;
    ensure_db_route(db, ROUTE_PATH, page.id, THEME).await?;
    tracing::info!(page_id = page.id, "kds website: homepage route ready");
    Ok(())
}

fn homepage_html_with_media_urls(urls: &[(String, String)]) -> String {
    let mut html = HOMEPAGE_HTML.to_string();
    for (name, url) in urls {
        html = html.replace(&format!("/static/{name}"), url);
    }
    html
}

async fn ensure_page_vnode(
    db: &DatabaseConnection,
    store: &DynFilestore,
    html: &[u8],
) -> anyhow::Result<lariv_rs::plugins::filesystem::entities::VNode> {
    let segments = ["website".into(), "pages".into()];
    let parent_id = node::ensure_directory_path(db, store, None, &segments)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let parent = match parent_id {
        Some(id) => node::get_by_id(db, id).await.ok().flatten(),
        None => None,
    };

    ensure_file_vnode(db, store, parent_id, parent.as_ref(), PAGE_NAME, html).await
}

/// Seeds blobs + `/static/{name}` aliases. Returns `(filename, /media/{id}/)` pairs
/// so the homepage can use the website plugin's public asset route instead of the
/// catch-all (which production proxies often intercept for `/static/`).
async fn ensure_static_assets(
    db: &DatabaseConnection,
    store: &DynFilestore,
) -> anyhow::Result<Vec<(String, String)>> {
    let segments = ["website".into(), "static".into()];
    let parent_id = node::ensure_directory_path(db, store, None, &segments)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let parent = match parent_id {
        Some(id) => node::get_by_id(db, id).await.ok().flatten(),
        None => None,
    };

    let mut urls = Vec::with_capacity(STATIC_ASSETS.len());
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
        let media_url = public_asset_url(vnode.id);
        tracing::info!(
            name = asset.name,
            vnode_id = vnode.id,
            media_url = %media_url,
            bytes = asset.bytes.len(),
            "kds website: static asset ready"
        );
        ensure_db_route(db, &format!("/static/{}", asset.name), vnode.id, "").await?;
        urls.push((asset.name.to_string(), media_url));
    }
    Ok(urls)
}

async fn ensure_file_vnode(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
    parent: Option<&lariv_rs::plugins::filesystem::entities::VNode>,
    name: &str,
    bytes: &[u8],
) -> anyhow::Result<lariv_rs::plugins::filesystem::entities::VNode> {
    if let Some(existing) = node::find_child(db, parent_id, name, false).await? {
        if vnode_bytes_match(store, &existing, bytes).await? {
            return Ok(existing);
        }
        tracing::warn!(
            name,
            vnode_id = existing.id,
            stored_path = existing.file_path.as_deref().unwrap_or(""),
            "kds website: rewriting vnode blob"
        );
        return render::replace_vnode_content(db, store, existing, bytes)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"));
    }

    tracing::info!(name, "kds website: creating vnode");
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

async fn vnode_bytes_match(
    store: &DynFilestore,
    existing: &lariv_rs::plugins::filesystem::entities::VNode,
    bytes: &[u8],
) -> anyhow::Result<bool> {
    let path = existing.file_path.as_deref().unwrap_or("");
    let mut download = match store.open(path, &existing.name).await {
        Ok(d) => d,
        Err(e) if e.is_missing() => {
            tracing::warn!(
                name = %existing.name,
                vnode_id = existing.id,
                stored_path = path,
                "kds website: blob missing from store"
            );
            return Ok(false);
        }
        Err(e) => return Err(anyhow::anyhow!("{e}")),
    };
    let mut current = Vec::new();
    download.reader.read_to_end(&mut current).await?;
    Ok(current == bytes)
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
        tracing::info!(path, page_id, "kds website: updated db route");
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
    tracing::info!(path, page_id, "kds website: created db route");
    Ok(())
}
