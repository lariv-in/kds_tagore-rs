//! Idempotent seed for the KDS Tagore public homepage, static media, and Custom theme.
//!
//! Registered as a [`lariv_rs::hooks::RunSeed`] hook so it runs only for `seed`, not `serve`.

use chrono::Utc;
use lariv_rs::app::MountedApp;
use lariv_rs::hooks::RunSeed;
use lariv_rs::plugin_install::define_plugin_install;
use lariv_rs::plugins::filesystem::node::{self, NodeFile};
use lariv_rs::plugins::filesystem::storage::DynFilestore;
use lariv_rs::plugins::website::{
    WebsiteTag,
    builder_assets::public_asset_url,
    entities::{
        WebsitePreferences,
        db_route::{self, Column as DbRouteColumn, Entity as DbRouteEntity},
    },
    preferences::{self, CUSTOM_THEME_ID},
    render,
    state::WebsiteState,
};
use lariv_rs::traits::get::GetByTag;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use tokio::io::AsyncReadExt;

/// Hook identity for the deployment-local website seed (distinct from [`WebsiteTag`] state).
pub struct KdsWebsiteSeedTag;

define_plugin_install! {
    plugin: KdsWebsiteSeedTag;
    /// Queue homepage/media seed for the `seed` CLI command.
    steps: [seeds(SeedsHook)]
}

/// Runs [`ensure_homepage`] when seed hooks execute.
#[derive(Clone, Copy, Default)]
pub struct SeedsHook;

#[async_trait::async_trait]
impl<M, WebsiteIdx> RunSeed<M, WebsiteIdx> for SeedsHook
where
    M: GetByTag<WebsiteTag, WebsiteIdx, Value = WebsiteState> + Sync,
{
    async fn run_seed(app: &MountedApp<M>) -> anyhow::Result<()> {
        tracing::info!("kds website: seeding homepage and media");
        ensure_homepage(app.get_capability_output::<WebsiteTag, WebsiteIdx>()).await?;
        tracing::info!("kds website: seed complete");
        Ok(())
    }
}

const HOMEPAGE_HTML: &str = include_str!("../assets/homepage.html");
const THEME_CSS: &[u8] = include_bytes!("../assets/theme/kds.css");
const THEME_JS: &[u8] = include_bytes!("../assets/theme/kds.js");
const ROUTE_PATH: &str = "/";
const PAGE_NAME: &str = "index.html";
const THEME_CSS_NAME: &str = "kds.css";
const THEME_JS_NAME: &str = "kds.js";
const THEME: &str = CUSTOM_THEME_ID;

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
    StaticAsset {
        name: "laser_cutting_machine.png",
        bytes: include_bytes!("../assets/static/laser_cutting_machine.png"),
    },
    StaticAsset {
        name: "bending_machine.png",
        bytes: include_bytes!("../assets/static/bending_machine.png"),
    },
];

pub async fn ensure_homepage(state: &WebsiteState) -> anyhow::Result<()> {
    ensure_homepage_state(&state.db, state.store.as_ref()).await
}

async fn ensure_homepage_state(
    db: &DatabaseConnection,
    store: &DynFilestore,
) -> anyhow::Result<()> {
    ensure_custom_theme(db, store).await?;
    let media_urls = ensure_static_assets(db, store).await?;
    let html = homepage_html_with_media_urls(&media_urls);
    let (page, page_rewritten) = ensure_page_vnode(db, store, html.as_bytes()).await?;
    ensure_db_route(db, ROUTE_PATH, page.id, THEME, page_rewritten).await?;
    tracing::info!(page_id = page.id, "kds website: homepage route ready");
    Ok(())
}

/// Seeds theme CSS/JS under `website/themes/` and points Custom theme preferences at them.
async fn ensure_custom_theme(db: &DatabaseConnection, store: &DynFilestore) -> anyhow::Result<()> {
    let segments = ["website".into(), "themes".into()];
    let parent_id = node::ensure_directory_path(db, store, None, &segments)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let parent = match parent_id {
        Some(id) => match node::get_by_id(db, id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "get node by id for website themes parent");
                None
            }
        },
        None => None,
    };

    let css = ensure_file_vnode(
        db,
        store,
        parent_id,
        parent.as_ref(),
        THEME_CSS_NAME,
        THEME_CSS,
    )
    .await?
    .0;
    let js = ensure_file_vnode(
        db,
        store,
        parent_id,
        parent.as_ref(),
        THEME_JS_NAME,
        THEME_JS,
    )
    .await?
    .0;

    preferences::save_preferences(
        db,
        WebsitePreferences {
            id: 1,
            created_at: None,
            updated_at: None,
            custom_theme_css_vnode_id: Some(css.id),
            custom_theme_js_vnode_id: Some(js.id),
        },
    )
    .await?;

    tracing::info!(
        css_vnode_id = css.id,
        js_vnode_id = js.id,
        "kds website: custom theme preferences ready"
    );
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
) -> anyhow::Result<(lariv_rs::plugins::filesystem::entities::VNode, bool)> {
    let segments = ["website".into(), "pages".into()];
    let parent_id = node::ensure_directory_path(db, store, None, &segments)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let parent = match parent_id {
        Some(id) => match node::get_by_id(db, id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "get node by id for website page parent");
                None
            }
        },
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
        Some(id) => match node::get_by_id(db, id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "get node by id for website static parent");
                None
            }
        },
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
        .await?
        .0;
        let media_url = public_asset_url(vnode.id);
        tracing::info!(
            name = asset.name,
            vnode_id = vnode.id,
            media_url = %media_url,
            bytes = asset.bytes.len(),
            "kds website: static asset ready"
        );
        ensure_db_route(db, &format!("/static/{}", asset.name), vnode.id, "", false).await?;
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
) -> anyhow::Result<(lariv_rs::plugins::filesystem::entities::VNode, bool)> {
    if let Some(existing) = node::find_child(db, parent_id, name, false).await? {
        if vnode_bytes_match(store, &existing, bytes).await? {
            return Ok((existing, false));
        }
        tracing::warn!(
            name,
            vnode_id = existing.id,
            stored_path = existing.file_path.as_deref().unwrap_or(""),
            "kds website: rewriting vnode blob"
        );
        let updated = render::replace_vnode_content(db, store, existing, bytes)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        return Ok((updated, true));
    }

    tracing::info!(name, "kds website: creating vnode");
    let created = node::create(
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
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok((created, true))
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
    reset_grapes_project: bool,
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
        if reset_grapes_project {
            // Drop stale GrapesJS project JSON so the builder reloads from seeded HTML.
            am.grapes_project = Set(None);
        }
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
