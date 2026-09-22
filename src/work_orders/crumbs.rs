//! Breadcrumbs and sidebar navigation menu for KDS Quotations.

use lariv_rs::components::{
    Crumb, SidebarMenu, SidebarMenuItem, breadcrumbs, sidebar_menu, sidebar_menu_item_pane,
};
use maud::{Markup, html};

use super::routes::{
    ComponentDetailRouteTag, DraftWorkOrdersDefaultRouteTag, InvoiceDetailRouteTag,
    IssuedWorkOrderDetailRouteTag, IssuedWorkOrdersRouteTag, WorkOrderDetailRouteTag,
    WorkOrdersComponentsRouteTag, WorkOrdersDefaultRouteTag, WorkOrdersPrefsGetRouteTag,
};

pub fn work_orders_tab_url(tab: &str) -> String {
    match tab {
        "issued" => IssuedWorkOrdersRouteTag.url(),
        _ => DraftWorkOrdersDefaultRouteTag.url(),
    }
}

pub fn wo_menu(active: &str) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "KDS Quotations",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Quotations",
                url: &WorkOrdersDefaultRouteTag.url(),
                active: active == "invoices",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Work Orders",
                url: &DraftWorkOrdersDefaultRouteTag.url(),
                active: active == "orders" || active == "issued",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Components",
                url: &WorkOrdersComponentsRouteTag.url(),
                active: active == "components",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Preferences",
                url: &WorkOrdersPrefsGetRouteTag.url(),
                active: active == "preferences",
                ..Default::default()
            }))
        },
    })
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

pub fn work_orders_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Work Orders",
        href: None,
    }])
}

pub fn work_order_crumbs(order_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Work Orders",
        &work_orders_tab_url("drafts"),
        order_number,
        &WorkOrderDetailRouteTag::new(id).url(),
        None,
    )
}

pub fn issued_work_orders_list_crumbs() -> Markup {
    work_orders_list_crumbs()
}

pub fn issued_work_order_crumbs(order_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Work Orders",
        &work_orders_tab_url("issued"),
        order_number,
        &IssuedWorkOrderDetailRouteTag::new(id).url(),
        None,
    )
}

pub fn components_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Components",
        href: None,
    }])
}

pub fn component_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Components",
        &WorkOrdersComponentsRouteTag.url(),
        name,
        &ComponentDetailRouteTag::new(id).url(),
        None,
    )
}

pub fn invoices_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Quotations",
        href: None,
    }])
}

pub fn invoice_crumbs(invoice_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Quotations",
        &WorkOrdersDefaultRouteTag.url(),
        invoice_number,
        &InvoiceDetailRouteTag::new(id).url(),
        None,
    )
}

pub fn work_orders_prefs_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Preferences",
        href: None,
    }])
}

pub fn work_order_edit_crumbs(order_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Work Orders",
        &work_orders_tab_url("drafts"),
        order_number,
        &WorkOrderDetailRouteTag::new(id).url(),
        Some("Edit"),
    )
}

pub fn component_edit_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Components",
        &WorkOrdersComponentsRouteTag.url(),
        name,
        &ComponentDetailRouteTag::new(id).url(),
        Some("Edit"),
    )
}

pub fn invoice_edit_crumbs(invoice_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Quotations",
        &WorkOrdersDefaultRouteTag.url(),
        invoice_number,
        &InvoiceDetailRouteTag::new(id).url(),
        Some("Edit"),
    )
}
