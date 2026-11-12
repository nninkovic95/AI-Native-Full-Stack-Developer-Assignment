//! Bindings to `js/editor.js`, the Quill glue. The Quill handle is kept as an opaque `JsValue`.

use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

#[wasm_bindgen(module = "/js/editor.js")]
extern "C" {
    #[wasm_bindgen(js_name = createEditor)]
    pub fn create_editor(container: &HtmlElement, on_change: &js_sys::Function) -> JsValue;

    #[wasm_bindgen(js_name = getContents)]
    pub fn get_contents(quill: &JsValue) -> String;

    #[wasm_bindgen(js_name = setContents)]
    pub fn set_contents(quill: &JsValue, json: &str);
}
