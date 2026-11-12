use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{api, logic::validate::validate_login, Route};

#[function_component(LoginPage)]
pub fn login_page() -> Html {
    let navigator = use_navigator().expect("router");
    let email = use_state(String::new);
    let password = use_state(String::new);
    let error = use_state(|| None::<String>);
    let busy = use_state(|| false);

    let oninput = |state: UseStateHandle<String>| {
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            state.set(input.value());
        })
    };

    let onsubmit = {
        let (email, password, error, busy) = (email.clone(), password.clone(), error.clone(), busy.clone());
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if let Err(msg) = validate_login(&email, &password) {
                error.set(Some(msg));
                return;
            }
            busy.set(true);
            error.set(None);
            let (email, password, error, busy, navigator) =
                ((*email).clone(), (*password).clone(), error.clone(), busy.clone(), navigator.clone());
            spawn_local(async move {
                match api::login(email.trim(), &password).await {
                    Ok(_) => navigator.push(&Route::List),
                    Err(f) => {
                        error.set(Some(f.message));
                        busy.set(false);
                    }
                }
            });
        })
    };

    html! {
        <main class="page login">
            <h1>{ "Ajaia Docs" }</h1>
            <form class="card" {onsubmit}>
                <h2>{ "Sign in" }</h2>
                <label>{ "Email" }
                    <input type="email" autocomplete="username" value={(*email).clone()} oninput={oninput(email.clone())} />
                </label>
                <label>{ "Password" }
                    <input type="password" autocomplete="current-password" value={(*password).clone()} oninput={oninput(password.clone())} />
                </label>
                if let Some(msg) = &*error {
                    <p class="error" role="alert">{ msg }</p>
                }
                <button type="submit" class="primary" disabled={*busy}>{ if *busy { "Signing in…" } else { "Sign in" } }</button>
                <p class="hint">
                    { "Demo accounts: alice@ajaia.test, bob@ajaia.test, carol@ajaia.test. Password: " }
                    <code>{ "ajaia-demo" }</code>
                </p>
            </form>
        </main>
    }
}
