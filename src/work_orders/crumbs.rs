//! Breadcrumbs and sidebar navigation menu for Work Orders.

use lariv_rs::components::{
    Crumb, SidebarMenu, SidebarMenuItem, breadcrumbs, sidebar_menu, sidebar_menu_item_pane,
};
use maud::{Markup, html};

use super::routes::{
    ComponentDetailRouteTag, InvoiceDetailRouteTag, MachineDetailRouteTag, MaterialDetailRouteTag,
    ShapeDetailRouteTag, WorkOrderDetailRouteTag, WorkOrdersComponentsRouteTag,
    WorkOrdersDefaultRouteTag, WorkOrdersInvoicesRouteTag, WorkOrdersMachinesRouteTag,
    WorkOrdersMaterialsRouteTag, WorkOrdersRatesRouteTag, WorkOrdersShapesRouteTag,
};

pub fn wo_menu(active: &str) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Work Orders",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Draft Work Orders",
                url: &WorkOrdersDefaultRouteTag.url(),
                active: active == "orders",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Components",
                url: &WorkOrdersComponentsRouteTag.url(),
                active: active == "components",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Shapes",
                url: &WorkOrdersShapesRouteTag.url(),
                active: active == "shapes",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Materials",
                url: &WorkOrdersMaterialsRouteTag.url(),
                active: active == "materials",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Material Rates",
                url: &WorkOrdersRatesRouteTag.url(),
                active: active == "rates",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Machines",
                url: &WorkOrdersMachinesRouteTag.url(),
                active: active == "machines",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Proforma Invoices",
                url: &WorkOrdersInvoicesRouteTag.url(),
                active: active == "invoices",
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
        label: "Draft Work Orders",
        href: None,
    }])
}

pub fn work_order_crumbs(order_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Draft Work Orders",
        &WorkOrdersDefaultRouteTag.url(),
        order_number,
        &WorkOrderDetailRouteTag::new(id).url(),
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

pub fn shapes_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Shapes",
        href: None,
    }])
}

pub fn shape_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Shapes",
        &WorkOrdersShapesRouteTag.url(),
        name,
        &ShapeDetailRouteTag::new(id).url(),
        None,
    )
}

pub fn materials_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Materials",
        href: None,
    }])
}

pub fn material_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Materials",
        &WorkOrdersMaterialsRouteTag.url(),
        name,
        &MaterialDetailRouteTag::new(id).url(),
        None,
    )
}

pub fn rates_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Material Rates",
        href: None,
    }])
}

pub fn machines_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Machines",
        href: None,
    }])
}

pub fn machine_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Machines",
        &WorkOrdersMachinesRouteTag.url(),
        name,
        &MachineDetailRouteTag::new(id).url(),
        None,
    )
}

pub fn invoices_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Proforma Invoices",
        href: None,
    }])
}

pub fn invoice_crumbs(invoice_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Proforma Invoices",
        &WorkOrdersInvoicesRouteTag.url(),
        invoice_number,
        &InvoiceDetailRouteTag::new(id).url(),
        None,
    )
}

pub fn work_order_edit_crumbs(order_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Work Orders",
        &WorkOrdersDefaultRouteTag.url(),
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

pub fn shape_edit_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Shapes",
        &WorkOrdersShapesRouteTag.url(),
        name,
        &ShapeDetailRouteTag::new(id).url(),
        Some("Edit"),
    )
}

pub fn material_edit_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Materials",
        &WorkOrdersMaterialsRouteTag.url(),
        name,
        &MaterialDetailRouteTag::new(id).url(),
        Some("Edit"),
    )
}

pub fn machine_edit_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Machines",
        &WorkOrdersMachinesRouteTag.url(),
        name,
        &MachineDetailRouteTag::new(id).url(),
        Some("Edit"),
    )
}

pub fn invoice_edit_crumbs(invoice_number: &str, id: i64) -> Markup {
    entity_crumbs(
        "Proforma Invoices",
        &WorkOrdersInvoicesRouteTag.url(),
        invoice_number,
        &InvoiceDetailRouteTag::new(id).url(),
        Some("Edit"),
    )
}

