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
    // The icon shows the mode a click switches to, matching the label read by screen readers.
    let icon = match *theme {
        Theme::Light => html! {
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        },
        Theme::Dark => html! {
            <>
                <circle cx="12" cy="12" r="4" />
                <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
            </>
        },
    };
    html! {
        <button type="button" class="theme-toggle" aria-label={label} title={label} {onclick}>
            <svg
                viewBox="0 0 24 24"
                width="20"
                height="20"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
            >
                { icon }
            </svg>
        </button>
    }
}
