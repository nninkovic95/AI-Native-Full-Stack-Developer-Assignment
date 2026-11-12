use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <main class="page">
            <h1>{ "Ajaia Docs" }</h1>
            <p>{ "Hello from Yew, served by Axum." }</p>
        </main>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
