//! Theme toggle: writes `data-theme` on the root element and remembers the choice in localStorage.
//! The inline script in `index.html` applies a saved choice before the first paint.

use crate::logic::theme::Theme;
use yew::prelude::*;

const STORAGE_KEY: &str = "theme";

fn system_prefers_dark() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
        .is_some_and(|m| m.matches())
}

fn saved_choice() -> Option<String> {
    // Storage can be unavailable (blocked cookies, private mode); the system preference then applies.
    web_sys::window()?.local_storage().ok()??.get_item(STORAGE_KEY).ok()?
}

fn apply(theme: Theme) {
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = root.set_attribute("data-theme", theme.as_str());
    }
    if let Some(Some(storage)) = web_sys::window().map(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(STORAGE_KEY, theme.as_str());
    }
}

#[function_component(ThemeToggle)]
pub fn theme_toggle() -> Html {
    let theme = use_state(|| Theme::from_stored(saved_choice().as_deref(), system_prefers_dark()));
    let onclick = {
        let theme = theme.clone();
        Callback::from(move |_| {
            let next = theme.toggled();
            apply(next);
            theme.set(next);
        })
    };
    let label = theme.toggle_label();
    html! {
        <button type="button" class="theme-toggle" aria-label={label} {onclick}>{ label }</button>
    }
}
