//! Breadcrumbs and sidebar for Delivery.

use lariv_core::components::{
    Crumb, SidebarMenu, SidebarMenuItem, breadcrumbs, sidebar_menu, sidebar_menu_item_pane,
};
use maud::{Markup, html};

use super::routes::{
    DeliveryChallanDetailRouteTag, DeliveryDefaultRouteTag, DeliveryPrefsGetRouteTag,
};

fn show_preferences() -> bool {
    lariv_plugin_users::role_authorization::current_auth()
        .is_none_or(|auth| auth.role != kds_plugin_accountant_role::ACCOUNTANT_ROLE)
}

pub fn delivery_menu(active: &str) -> Markup {
    let preferences = show_preferences().then(|| {
        sidebar_menu_item_pane(SidebarMenuItem {
            title: "Preferences",
            url: &DeliveryPrefsGetRouteTag.url(),
            active: active == "preferences",
            ..Default::default()
        })
    });
    sidebar_menu(SidebarMenu {
        title: "Delivery",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Delivery Challans",
                url: &DeliveryDefaultRouteTag.url(),
                active: active == "challans",
                ..Default::default()
            }))
            @if let Some(item) = preferences {
                (item)
            }
        },
    })
}

fn entity_crumbs(name: &str, detail_url: &str, action: Option<&str>) -> Markup {
    let list_url = DeliveryDefaultRouteTag.url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Delivery Challans",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Delivery Challans",
                href: Some(&list_url),
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

pub fn challan_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Delivery Challans",
        href: None,
    }])
}

pub fn challan_crumbs(number: &str, id: i64) -> Markup {
    let label = if number.trim().is_empty() {
        format!("Challan #{id}")
    } else {
        number.to_string()
    };
    entity_crumbs(&label, &DeliveryChallanDetailRouteTag::new(id).url(), None)
}

pub fn prefs_crumbs() -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "Delivery Challans",
            href: Some(&DeliveryDefaultRouteTag.url()),
        },
        Crumb {
            label: "Preferences",
            href: None,
        },
    ])
}
