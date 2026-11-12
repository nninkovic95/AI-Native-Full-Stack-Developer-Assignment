use shared::{DocumentSummary, User};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    api::{self, FailureKind},
    logic::{
        sharing::{owner_label, split_documents},
        validate::{import_accept, import_hint, validate_import},
    },
    Route,
};

#[function_component(ListPage)]
pub fn list_page() -> Html {
    let navigator = use_navigator().expect("router");
    let user = use_state(|| None::<User>);
    let docs = use_state(|| None::<Vec<DocumentSummary>>);
    let error = use_state(|| None::<String>);
    let importing = use_state(|| false);
    let file_input = use_node_ref();

    {
        let (user, docs, error, navigator) = (user.clone(), docs.clone(), error.clone(), navigator.clone());
        use_effect_with((), move |_| {
            spawn_local(async move {
                let result: api::ApiResult<_> = async { Ok((api::me().await?, api::list_documents().await?)) }.await;
                match result {
                    Ok((u, d)) => {
                        user.set(Some(u));
                        docs.set(Some(d));
                    }
                    Err(f) if f.kind == FailureKind::Unauthorized => navigator.push(&Route::Login),
                    Err(f) => error.set(Some(f.message)),
                }
            });
        });
    }

    // A second click while the request is pending would create a second, empty document.
    // The flag is checked synchronously; the state only drives the button's disabled look.
    let create_pending = use_mut_ref(|| false);
    let creating = use_state(|| false);
    let on_new = {
        let (error, navigator) = (error.clone(), navigator.clone());
        let (create_pending, creating) = (create_pending.clone(), creating.clone());
        Callback::from(move |_| {
            if create_pending.replace(true) {
                return;
            }
            creating.set(true);
            let (error, navigator) = (error.clone(), navigator.clone());
            let (create_pending, creating) = (create_pending.clone(), creating.clone());
            spawn_local(async move {
                match api::create_document().await {
                    Ok(doc) => navigator.push(&Route::Editor { id: doc.id }),
                    Err(f) if f.kind == FailureKind::Unauthorized => navigator.push(&Route::Login),
                    Err(f) => {
                        *create_pending.borrow_mut() = false;
                        creating.set(false);
                        error.set(Some(f.message));
                    }
                }
            });
        })
    };

    let on_import_click = {
        let file_input = file_input.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let on_file_chosen = {
        let (error, importing, navigator) = (error.clone(), importing.clone(), navigator.clone());
        Callback::from(move |e: Event| {
            let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) else {
                return;
            };
            let file = input.files().and_then(|files| files.get(0));
            // Clear the value so picking the same file again still fires a change event.
            input.set_value("");
            let Some(file) = file else {
                return;
            };
            if let Err(msg) = validate_import(&file.name(), file.size() as u64) {
                error.set(Some(msg));
                return;
            }
            error.set(None);
            importing.set(true);
            let (error, importing, navigator) = (error.clone(), importing.clone(), navigator.clone());
            spawn_local(async move {
                let result = api::import_document(&file).await;
                importing.set(false);
                match result {
                    Ok(doc) => navigator.push(&Route::Editor { id: doc.id }),
                    Err(f) if f.kind == FailureKind::Unauthorized => navigator.push(&Route::Login),
                    Err(f) => error.set(Some(f.message)),
                }
            });
        })
    };

    let on_sign_out = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            let navigator = navigator.clone();
            spawn_local(async move {
                // Go to the login page even if the request fails; the session is useless either way.
                let _ = api::logout().await;
                navigator.push(&Route::Login);
            });
        })
    };

    html! {
        <main class="page">
            <header class="topbar">
                <h1>{ "Ajaia Docs" }</h1>
                <div class="userbox">
                    if let Some(u) = &*user {
                        <span>{ &u.name }</span>
                    }
                    <button onclick={on_sign_out}>{ "Sign out" }</button>
                </div>
            </header>
            <div class="toolbar">
                <h2>{ "Documents" }</h2>
                <div class="toolbar-actions">
                    <button onclick={on_import_click} disabled={*importing}>
                        { if *importing { "Importing…" } else { "Import file" } }
                    </button>
                    <button class="primary" onclick={on_new} disabled={*creating}>
                        { if *creating { "Creating…" } else { "New document" } }
                    </button>
                </div>
                <input
                    ref={file_input}
                    type="file"
                    class="visually-hidden"
                    accept={import_accept()}
                    onchange={on_file_chosen}
                />
            </div>
            <p class="hint toolbar-hint">{ import_hint() }</p>
            if let Some(msg) = &*error {
                <p class="error" role="alert">{ msg }</p>
            }
            { match &*docs {
                None if error.is_none() => html! { <p class="muted">{ "Loading…" }</p> },
                None => html! {},
                Some(all) => {
                    let (owned, shared_with_me) = split_documents(all.clone());
                    html! {
                        <>
                            <h2 class="section-title">{ "My documents" }</h2>
                            { doc_table(&owned, "No documents yet. Create one to get started.") }
                            <h2 class="section-title">{ "Shared with me" }</h2>
                            { doc_table(&shared_with_me, "Nothing has been shared with you yet") }
                        </>
                    }
                }
            } }
        </main>
    }
}

fn doc_table(docs: &[DocumentSummary], empty_message: &str) -> Html {
    if docs.is_empty() {
        return html! { <p class="muted">{ empty_message }</p> };
    }
    html! {
        <table class="doclist">
            <thead><tr><th>{ "Title" }</th><th>{ "Owner" }</th><th>{ "Last edited" }</th></tr></thead>
            <tbody>
                { for docs.iter().map(|doc| html! {
                    <tr key={doc.id.to_string()}>
                        <td><Link<Route> to={Route::Editor { id: doc.id }}>{ &doc.title }</Link<Route>></td>
                        <td>{ if doc.is_owner { "You".to_string() } else { owner_label(false, &doc.owner.name) } }</td>
                        <td>{ doc.updated_at.format("%Y-%m-%d %H:%M UTC").to_string() }</td>
                    </tr>
                }) }
            </tbody>
        </table>
    }
}
