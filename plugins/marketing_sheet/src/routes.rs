use super::handlers;

lariv_core::define_plugin_routes! {
    plugin: MarketingSheetTag;
    prefix: "/dashboard";
    routes: [
        get MarketingSheetPageRouteTag, "/marketing-sheet", handlers::page;
        post MarketingSheetImportRouteTag, "/marketing-sheet/import", handlers::import_post;
        post MarketingSheetExportRouteTag, "/marketing-sheet/export", bare handlers::export_post, file;
    ]
}
