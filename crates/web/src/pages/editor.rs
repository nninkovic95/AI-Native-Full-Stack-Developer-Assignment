use std::cell::RefCell;
use std::rc::Rc;

use gloo_timers::future::TimeoutFuture;
use shared::Document;
use uuid::Uuid;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlElement, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    api::{self, FailureKind},
    editor_js,
    logic::{
        api_shapes::{content_update, title_update},
        save_state::{SaveEvent, SaveState},
        sharing::owner_label,
        validate::normalize_title,
    },
    pages::share_panel::SharePanel,
    Route,
};

const AUTOSAVE_DELAY_MS: u32 = 800;

/// Everything an autosave needs, cheap to clone into callbacks and async tasks.
#[derive(Clone)]
struct Saver {
    id: Uuid,
    quill: Rc<RefCell<Option<JsValue>>>,
    state: Rc<RefCell<SaveState>>,
    label: UseStateHandle<String>,
    navigator: Navigator,
}

impl Saver {
    fn apply(&self, event: SaveEvent) {
        let next = self.state.borrow().clone().next(event);
        self.label.set(next.label());
        *self.state.borrow_mut() = next;
    }

    /// Saves while edits are pending. A save already in flight picks up later edits itself.
    async fn flush(self) {
        if matches!(*self.state.borrow(), SaveState::Saving { .. }) {
            return;
        }
        while self.state.borrow().needs_save() {
            let delta = match self.quill.borrow().as_ref() {
                Some(q) => editor_js::get_contents(q),
                None => return,
            };
            let body = match content_update(&delta) {
                Ok(b) => b,
                Err(msg) => return self.apply(SaveEvent::SaveErr(msg)),
            };
            self.apply(SaveEvent::SaveStarted);
            match api::update_document(self.id, &body).await {
                Ok(_) => self.apply(SaveEvent::SaveOk),
                Err(f) if f.kind == FailureKind::Unauthorized => {
                    self.navigator.push(&Route::Login);
                    return;
                }
                Err(f) => return self.apply(SaveEvent::SaveErr(f.message)),
            }
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct EditorProps {
    pub id: Uuid,
}

#[function_component(EditorPage)]
pub fn editor_page(props: &EditorProps) -> Html {
    let id = props.id;
    let navigator = use_navigator().expect("router");

    let doc = use_state(|| None::<Rc<Document>>);
    let load_error = use_state(|| None::<String>);
    let title = use_state(String::new);
    let title_error = use_state(|| None::<String>);
    let last_saved_title = use_mut_ref(String::new);

    let share_open = use_state(|| false);
    let label = use_state(String::new);
    let editor_ref = use_node_ref();
    let quill = use_mut_ref(|| None::<JsValue>);
    let on_change_closure = use_mut_ref(|| None::<Closure<dyn FnMut()>>);
    let generation = use_mut_ref(|| 0u32);
    let state = use_mut_ref(|| SaveState::Idle);

    let saver = Saver { id, quill: quill.clone(), state, label: label.clone(), navigator: navigator.clone() };

    // Load the document.
    {
        let (doc, load_error, title, last_saved_title, navigator) =
            (doc.clone(), load_error.clone(), title.clone(), last_saved_title.clone(), navigator.clone());
        use_effect_with(id, move |id| {
            let id = *id;
            spawn_local(async move {
                match api::get_document(id).await {
                    Ok(d) => {
                        title.set(d.title.clone());
                        *last_saved_title.borrow_mut() = d.title.clone();
                        doc.set(Some(Rc::new(d)));
                    }
                    Err(f) if f.kind == FailureKind::Unauthorized => navigator.push(&Route::Login),
                    Err(f) => load_error.set(Some(f.message)),
                }
            });
        });
    }

    // Create Quill once the document is loaded and the container is in the DOM.
    {
        let saver = saver.clone();
        let (editor_ref, doc, generation) = (editor_ref.clone(), doc.clone(), generation.clone());
        let (quill, on_change_closure) = (quill.clone(), on_change_closure.clone());
        use_effect_with(doc.is_some(), move |loaded| {
            if *loaded && quill.borrow().is_none() {
                if let (Some(container), Some(d)) = (editor_ref.cast::<HtmlElement>(), doc.as_ref()) {
                    let on_change = {
                        let saver = saver.clone();
                        Closure::<dyn FnMut()>::new(move || {
                            saver.apply(SaveEvent::Changed);
                            let ticket = {
                                let mut g = generation.borrow_mut();
                                *g += 1;
                                *g
                            };
                            let (saver, generation) = (saver.clone(), generation.clone());
                            spawn_local(async move {
                                TimeoutFuture::new(AUTOSAVE_DELAY_MS).await;
                                // Only the last edit of a burst saves.
                                if *generation.borrow() == ticket {
                                    saver.flush().await;
                                }
                            });
                        })
                    };
                    let q = editor_js::create_editor(&container, on_change.as_ref().unchecked_ref());
                    editor_js::set_contents(&q, &d.content.to_string());
                    *quill.borrow_mut() = Some(q);
                    *on_change_closure.borrow_mut() = Some(on_change);
                }
            }
            // Leaving the page within the debounce window must not drop the last edit.
            let saver = saver.clone();
            move || {
                if saver.state.borrow().needs_save() {
                    spawn_local(saver.flush());
                }
            }
        });
    }

    let commit_title = {
        let (title, title_error, last_saved_title, navigator) =
            (title.clone(), title_error.clone(), last_saved_title.clone(), navigator.clone());
        Callback::from(move |_: ()| {
            let normalized = normalize_title(&title);
            title.set(normalized.clone());
            if normalized == *last_saved_title.borrow() {
                return;
            }
            let (title_error, last_saved_title, navigator) =
                (title_error.clone(), last_saved_title.clone(), navigator.clone());
            spawn_local(async move {
                match api::update_document(id, &title_update(&normalized)).await {
                    Ok(_) => {
                        *last_saved_title.borrow_mut() = normalized;
                        title_error.set(None);
                    }
                    Err(f) if f.kind == FailureKind::Unauthorized => navigator.push(&Route::Login),
                    Err(f) => title_error.set(Some(f.message)),
                }
            });
        })
    };

    let on_title_input = {
        let title = title.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            title.set(input.value());
        })
    };
    let on_title_blur = {
        let commit = commit_title.clone();
        Callback::from(move |_: FocusEvent| commit.emit(()))
    };
    let on_title_key = Callback::from(move |e: KeyboardEvent| {
        if e.key() == "Enter" {
            e.prevent_default();
            // Blurring runs the blur handler, which saves.
            let input: HtmlInputElement = e.target_unchecked_into();
            let _ = input.blur();
        }
    });

    let toggle_share = {
        let share_open = share_open.clone();
        Callback::from(move |_| share_open.set(!*share_open))
    };
    let close_share = {
        let share_open = share_open.clone();
        Callback::from(move |_: ()| share_open.set(false))
    };

    html! {
        <main class="page editor-page">
            <div class="editor-header">
                <Link<Route> to={Route::List}>{ "← All documents" }</Link<Route>>
                <div class="editor-header-right">
                    if let Some(d) = &*doc {
                        <span class="ownership">{ owner_label(d.is_owner, &d.owner.name) }</span>
                        <button onclick={toggle_share} aria-expanded={share_open.to_string()}>{ "Share" }</button>
                    }
                    <span class="status" aria-live="polite">{ (*label).clone() }</span>
                </div>
            </div>
            // Rendered outside the editor wrapper so opening it never touches Quill's DOM.
            if let (true, Some(d)) = (*share_open, &*doc) {
                <SharePanel id={d.id} owner={d.owner.clone()} is_owner={d.is_owner} on_close={close_share} />
            }
            if let Some(msg) = &*load_error {
                <p class="error" role="alert">{ msg }</p>
            } else if doc.is_none() {
                <p class="muted">{ "Loading…" }</p>
            } else {
                <input
                    class="doc-title"
                    type="text"
                    aria-label="Document title"
                    value={(*title).clone()}
                    oninput={on_title_input}
                    onblur={on_title_blur}
                    onkeydown={on_title_key}
                />
                if let Some(msg) = &*title_error {
                    <p class="error" role="alert">{ format!("Rename failed: {msg}") }</p>
                }
                // Quill adds its toolbar next to the container, so the wrapper must hold nothing else from Yew.
                <div class="editor-wrap"><div ref={editor_ref}></div></div>
            }
        </main>
    }
}
