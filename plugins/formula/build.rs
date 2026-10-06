fn main() {
    // Selects `::lariv_core::` paths in lariv-rs-macros (see `plugin_crate`).
    println!("cargo:rustc-env=LARIV_PLUGIN_CRATE=1");
}
