//! Breadcrumb trails for Machinery Schedule pages.

use lariv_rs::components::{Crumb, breadcrumbs};
use lariv_rs::http::RouteQueryBuilder;
use maud::Markup;

use super::routes::{
    CompletedJobDetailRouteTag, JobDefaultRouteTag, JobDetailRouteTag, MachineDefaultRouteTag,
    MachineDetailRouteTag,
};

pub fn jobs_tab_url(tab: &str) -> String {
    RouteQueryBuilder::new(JobDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn entity_crumbs(
    list_label: &'static str,
    list_url: &str,
    name: &str,
    detail_url: &str,
    action: Option<&str>,
) -> Markup {
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: list_label,
                href: Some(list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: list_label,
                href: Some(list_url),
            },
            Crumb {
                label: name,
                href: Some(detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

pub fn jobs_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Jobs",
        href: None,
    }])
}

pub fn job_crumbs(name: &str, job_id: i64, action: Option<&str>) -> Markup {
    entity_crumbs(
        "Jobs",
        &jobs_tab_url("jobs"),
        name,
        &JobDetailRouteTag::new(job_id).url(),
        action,
    )
}

pub fn completed_job_crumbs(name: &str, completed_id: i64, action: Option<&str>) -> Markup {
    entity_crumbs(
        "Jobs",
        &jobs_tab_url("completed"),
        name,
        &CompletedJobDetailRouteTag::new(completed_id).url(),
        action,
    )
}

pub fn machines_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Machines",
        href: None,
    }])
}

pub fn machine_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    let list_url = MachineDefaultRouteTag.url();
    entity_crumbs(
        "Machines",
        &list_url,
        name,
        &MachineDetailRouteTag::new(id).url(),
        action,
    )
}
