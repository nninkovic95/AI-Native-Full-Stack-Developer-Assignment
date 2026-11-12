use shared::{DocumentSummary, User};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    api::{self, FailureKind},
    Route,
};

#[function_component(ListPage)]
pub fn list_page() -> Html {
    let navigator = use_navigator().expect("router");
    let user = use_state(|| None::<User>);
    let docs = use_state(|| None::<Vec<DocumentSummary>>);
    let error = use_state(|| None::<String>);

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

    let on_new = {
        let (error, navigator) = (error.clone(), navigator.clone());
        Callback::from(move |_| {
            let (error, navigator) = (error.clone(), navigator.clone());
            spawn_local(async move {
                match api::create_document().await {
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
                <button class="primary" onclick={on_new}>{ "New document" }</button>
            </div>
            if let Some(msg) = &*error {
                <p class="error" role="alert">{ msg }</p>
            }
            { match &*docs {
                None if error.is_none() => html! { <p class="muted">{ "Loading…" }</p> },
                None => html! {},
                Some(d) if d.is_empty() => html! { <p class="muted">{ "No documents yet. Create one to get started." }</p> },
                Some(d) => html! {
                    <table class="doclist">
                        <thead><tr><th>{ "Title" }</th><th>{ "Owner" }</th><th>{ "Last edited" }</th></tr></thead>
                        <tbody>
                            { for d.iter().map(|doc| html! {
                                <tr key={doc.id.to_string()}>
                                    <td><Link<Route> to={Route::Editor { id: doc.id }}>{ &doc.title }</Link<Route>></td>
                                    <td>{ &doc.owner.name }</td>
                                    <td>{ doc.updated_at.format("%Y-%m-%d %H:%M UTC").to_string() }</td>
                                </tr>
                            }) }
                        </tbody>
                    </table>
                },
            } }
        </main>
    }
}
