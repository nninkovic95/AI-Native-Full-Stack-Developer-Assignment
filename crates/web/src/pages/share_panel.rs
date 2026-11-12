use shared::User;
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    api::{self, FailureKind},
    logic::sharing::validate_share_email,
    Route,
};

#[derive(Properties, PartialEq)]
pub struct SharePanelProps {
    pub id: Uuid,
    pub owner: User,
    pub is_owner: bool,
    pub on_close: Callback<()>,
}

/// Lists who can open the document and, for the owner, lets them add people by email.
#[function_component(SharePanel)]
pub fn share_panel(props: &SharePanelProps) -> Html {
    let navigator = use_navigator().expect("router");
    let people = use_state(|| None::<Vec<User>>);
    let email = use_state(String::new);
    let error = use_state(|| None::<String>);
    let busy = use_state(|| false);

    {
        let (people, error, navigator) = (people.clone(), error.clone(), navigator.clone());
        use_effect_with(props.id, move |id| {
            let id = *id;
            spawn_local(async move {
                match api::list_shares(id).await {
                    Ok(list) => people.set(Some(list)),
                    Err(f) if f.kind == FailureKind::Unauthorized => navigator.push(&Route::Login),
                    Err(f) => error.set(Some(f.message)),
                }
            });
        });
    }

    let on_input = {
        let email = email.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            email.set(input.value());
        })
    };

    let on_submit = {
        let (email, error, busy, people, navigator) =
            (email.clone(), error.clone(), busy.clone(), people.clone(), navigator.clone());
        let (id, own_email) = (props.id, props.owner.email.clone());
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if *busy {
                return;
            }
            let checked = match validate_share_email(&email, &own_email) {
                Ok(v) => v,
                Err(msg) => {
                    error.set(Some(msg));
                    return;
                }
            };
            error.set(None);
            busy.set(true);
            let (email, error, busy, people, navigator) =
                (email.clone(), error.clone(), busy.clone(), people.clone(), navigator.clone());
            spawn_local(async move {
                let result = api::share_document(id, &checked).await;
                busy.set(false);
                match result {
                    Ok(user) => {
                        let mut list = (*people).clone().unwrap_or_default();
                        list.push(user);
                        list.sort_by(|a, b| a.name.cmp(&b.name));
                        people.set(Some(list));
                        email.set(String::new());
                    }
                    Err(f) if f.kind == FailureKind::Unauthorized => navigator.push(&Route::Login),
                    Err(f) => error.set(Some(f.message)),
                }
            });
        })
    };

    let on_close = {
        let cb = props.on_close.clone();
        Callback::from(move |_| cb.emit(()))
    };

    html! {
        <section class="share-panel" aria-label="Share this document">
            <div class="share-header">
                <h2>{ "People with access" }</h2>
                <button onclick={on_close}>{ "Close" }</button>
            </div>
            <ul class="share-list">
                <li>
                    <span>{ format!("{} ({})", props.owner.name, props.owner.email) }</span>
                    <span class="badge">{ "Owner" }</span>
                </li>
                { for people.iter().flatten().map(|u| html! {
                    <li key={u.id.to_string()}><span>{ format!("{} ({})", u.name, u.email) }</span></li>
                }) }
            </ul>
            if props.is_owner {
                <form class="share-form" onsubmit={on_submit}>
                    <input
                        type="text"
                        aria-label="Email address to share with"
                        placeholder="Email address"
                        value={(*email).clone()}
                        oninput={on_input}
                    />
                    <button class="primary" type="submit" disabled={*busy}>
                        { if *busy { "Sharing…" } else { "Share" } }
                    </button>
                </form>
                <p class="hint">{ "Demo accounts you can share with: alice@ajaia.test, bob@ajaia.test, carol@ajaia.test" }</p>
            } else {
                <p class="hint">{ "Only the owner can share this document" }</p>
            }
            if let Some(msg) = &*error {
                <p class="error" role="alert">{ msg }</p>
            }
        </section>
    }
}
