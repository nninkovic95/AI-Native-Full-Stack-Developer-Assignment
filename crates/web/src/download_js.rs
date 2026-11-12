//! Binding to `js/download.js`, which hands a text file to the browser as a download.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/download.js")]
extern "C" {
    #[wasm_bindgen(js_name = downloadText)]
    pub fn download_text(file_name: &str, text: &str);
}
