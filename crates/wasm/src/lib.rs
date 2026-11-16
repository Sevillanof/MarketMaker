#![recursion_limit = "256"]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use web::*;
    // initializes logging using the `log` crate
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    // `hydrate_lazy`: el editor (`/editor`) se baja aparte, ver `EditorRoute`.
    leptos::mount::hydrate_lazy(App);
}
