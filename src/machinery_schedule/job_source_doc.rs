//! Job source-document type capability — install-time registration via `cap_hook`.
//!
//! Machinery Schedule attaches [`JobSourceDocCap`]; Work Orders registers a loader with
//! `cap_hook(JobSourceDocTag, JobSourceDocCap, Hook)`. Handlers extract
//! [`Cap`](lariv_rs::http::Cap)`<`[`JobSourceDocRegistry`]`>`.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use frunk::{HCons, HNil, hlist::HList};
use lariv_rs::{
    app::App,
    capability::{CapHookExt, Capability, HasCapTag},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};
use sea_orm::DatabaseConnection;

/// Capability tag for the job source-document type registry.
pub struct JobSourceDocTag;

/// Loaded backing document for a resolved type/id pair.
pub trait JobSourceDocInstance: Send + Sync {
    fn source_doc_type(&self) -> &str;
    fn source_doc_id(&self) -> i64;
    fn display_name(&self) -> String;
    fn detail_url(&self) -> String;
}

/// Describes how one document kind participates in linking and URLs.
#[async_trait]
pub trait JobSourceDocType: Send + Sync {
    fn source_doc_type(&self) -> &str;
    fn display_name(&self) -> &str;
    fn detail_url(&self, id: i64) -> String;
    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn JobSourceDocInstance>>;
}

/// Folded map of registered job source-document type loaders.
#[derive(Clone, Default)]
pub struct JobSourceDocRegistry {
    types: HashMap<String, Arc<dyn JobSourceDocType>>,
}

impl JobSourceDocRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a loader; duplicate keys are ignored (install-order first wins).
    pub fn register(mut self, loader: Arc<dyn JobSourceDocType>) -> Self {
        let key = loader.source_doc_type().to_string();
        self.types.entry(key).or_insert(loader);
        self
    }

    pub fn get(&self, typ: &str) -> Option<Arc<dyn JobSourceDocType>> {
        self.types.get(typ).cloned()
    }

    pub fn type_display_name(&self, typ: &str) -> String {
        self.get(typ)
            .map(|loader| loader.display_name().to_string())
            .unwrap_or_else(|| humanize_type_name(typ))
    }

    pub fn type_detail_url(&self, typ: &str, id: i64) -> Option<String> {
        self.get(typ).map(|loader| loader.detail_url(id))
    }

    pub async fn resolve_instance(
        &self,
        db: &DatabaseConnection,
        typ: &str,
        id: i64,
    ) -> Result<Arc<dyn JobSourceDocInstance>> {
        if typ.is_empty() {
            return Err(anyhow!("job source document: empty type"));
        }
        let loader = self
            .get(typ)
            .ok_or_else(|| anyhow!("job source document: unknown type {typ:?}"))?;
        let inst = loader.load_from_id(db, id).await?;
        if inst.source_doc_type() != typ {
            return Err(anyhow!(
                "job source document: type mismatch: registry key {typ:?}, instance {:?}",
                inst.source_doc_type()
            ));
        }
        Ok(inst)
    }
}

/// Plugin hook for registering job source-document types at install time.
pub trait JobSourceDocRegistrar: Sized {
    fn register_job_source_docs(self, registry: JobSourceDocRegistry) -> JobSourceDocRegistry;
}

/// Builder-phase job source-document capability.
#[derive(Clone, Default)]
pub struct JobSourceDocCap<Hooks> {
    pub hooks: Hooks,
    pub items: JobSourceDocRegistry,
    _tag: PhantomData<fn() -> JobSourceDocTag>,
}

impl<Hooks> JobSourceDocCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: JobSourceDocRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> JobSourceDocCap<HCons<Tagged<HTag, H>, Hooks>> {
        JobSourceDocCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for JobSourceDocCap<Hooks> {
    type Tag = JobSourceDocTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for JobSourceDocCap<Hooks> {
    type Hooked = JobSourceDocCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the registry (tail first = install order).
pub trait FoldJobSourceDocRegistrarHooks {
    fn fold(self, registry: JobSourceDocRegistry) -> JobSourceDocRegistry;
}

impl FoldJobSourceDocRegistrarHooks for HNil {
    fn fold(self, registry: JobSourceDocRegistry) -> JobSourceDocRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldJobSourceDocRegistrarHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldJobSourceDocRegistrarHooks,
    H: JobSourceDocRegistrar + Copy,
{
    fn fold(self, registry: JobSourceDocRegistry) -> JobSourceDocRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_job_source_docs(registry)
    }
}

impl<Hooks> Capability for JobSourceDocCap<Hooks>
where
    Hooks: FoldJobSourceDocRegistrarHooks,
{
    type Value = JobSourceDocRegistry;
    type Output = Tagged<JobSourceDocTag, JobSourceDocRegistry>;
    type Hooks = Hooks;
    type Items = JobSourceDocRegistry;

    fn mount(self) -> Self::Output {
        let registry = self.hooks.fold(self.items);
        Tagged::new(registry)
    }
}

/// No-op base hook from Machinery Schedule.
#[derive(Clone, Copy, Default)]
pub struct BaseHook;

impl JobSourceDocRegistrar for BaseHook {
    fn register_job_source_docs(self, registry: JobSourceDocRegistry) -> JobSourceDocRegistry {
        registry
    }
}

/// Attach an empty job source-doc capability (prefer `cap_attach` in install steps).
pub fn with_job_source_docs<L, Proof>(app: App<L>) -> App<HCons<JobSourceDocCap<HNil>, L>>
where
    L: HList + CapTagAbsent<JobSourceDocTag, Proof>,
{
    app.add_capability(JobSourceDocCap::<HNil>::new())
}

/// Humanize an unregistered type key's last path segment.
pub fn humanize_type_name(typ: &str) -> String {
    let name = typ.rsplit('.').next().unwrap_or(typ);
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push(' ');
        }
        out.extend(ch.to_lowercase());
    }
    if out.is_empty() {
        return typ.to_string();
    }
    let mut chars = out.chars();
    match chars.next() {
        None => typ.to_string(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Resolved display fields for a job's source document.
#[derive(Clone, Debug, Default)]
pub struct JobSourceDocDisplay {
    pub type_label: String,
    pub instance_name: String,
    pub detail_url: String,
}

impl JobSourceDocDisplay {
    pub fn empty() -> Self {
        Self {
            type_label: "—".into(),
            instance_name: "—".into(),
            detail_url: String::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        (self.instance_name.is_empty() || self.instance_name == "—") && (self.detail_url.is_empty())
    }
}

/// Resolve a job's stored type/id pair through the registry.
pub async fn resolve_job_source_doc(
    db: &DatabaseConnection,
    registry: &JobSourceDocRegistry,
    typ: &str,
    id: i64,
) -> JobSourceDocDisplay {
    if typ.is_empty() || id <= 0 {
        return JobSourceDocDisplay::empty();
    }
    match registry.resolve_instance(db, typ, id).await {
        Ok(inst) => JobSourceDocDisplay {
            type_label: registry.type_display_name(typ),
            instance_name: inst.display_name(),
            detail_url: inst.detail_url(),
        },
        Err(_) => JobSourceDocDisplay {
            type_label: registry.type_display_name(typ),
            instance_name: format!("#{id}"),
            detail_url: registry.type_detail_url(typ, id).unwrap_or_default(),
        },
    }
}
