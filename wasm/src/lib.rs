//! Wasm bindings for the browser playground (`docs/playground`).

use apidrift::{diff_openapi_to_html, OpenApiInputFormat};
use wasm_bindgen::prelude::*;

/// Build the HTML diff report. All processing runs in the browser; specs are not uploaded.
#[wasm_bindgen(js_name = generateReport)]
pub fn generate_report(
    base: &str,
    current: &str,
    base_is_yaml: bool,
    current_is_yaml: bool,
    include_descriptions: bool,
) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    let base_fmt = if base_is_yaml {
        OpenApiInputFormat::Yaml
    } else {
        OpenApiInputFormat::Json
    };
    let current_fmt = if current_is_yaml {
        OpenApiInputFormat::Yaml
    } else {
        OpenApiInputFormat::Json
    };
    diff_openapi_to_html(base, current, base_fmt, current_fmt, include_descriptions)
        .map(|(html, _)| html)
        .map_err(|e| JsValue::from_str(&e))
}
