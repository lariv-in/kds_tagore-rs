//! Breadcrumbs and the work-order sidebar.

use lariv_core::components::{
    Crumb, SidebarMenu, SidebarMenuItem, breadcrumbs, sidebar_menu, sidebar_menu_item_pane,
};
use maud::Markup;

use super::routes::{WorkOrderDetailRouteTag, WorkOrderListRouteTag};

pub fn menu() -> Markup {
    let url = WorkOrderListRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: "Work orders",
        children: sidebar_menu_item_pane(SidebarMenuItem {
            title: "Work orders",
            url: &url,
            active: true,
            ..Default::default()
        }),
    })
}

pub fn list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Work orders",
        href: None,
    }])
}

pub fn detail_crumbs(title: &str) -> Markup {
    let list_url = WorkOrderListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Work orders",
            href: Some(&list_url),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}

pub fn detail_url(id: i64) -> String {
    WorkOrderDetailRouteTag::new(id).url()
}
