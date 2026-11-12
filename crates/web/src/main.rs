mod api;
mod editor_js;
mod logic;
mod pages;

use uuid::Uuid;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/login")]
    Login,
    #[at("/")]
    List,
    #[at("/doc/:id")]
    Editor { id: Uuid },
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(route: Route) -> Html {
    match route {
        Route::Login => html! { <pages::login::LoginPage /> },
        Route::List => html! { <pages::list::ListPage /> },
        Route::Editor { id } => html! { <pages::editor::EditorPage key={id.to_string()} {id} /> },
        Route::NotFound => html! {
            <main class="page">
                <h1>{ "Page not found" }</h1>
                <Link<Route> to={Route::List}>{ "Back to documents" }</Link<Route>>
            </main>
        },
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
