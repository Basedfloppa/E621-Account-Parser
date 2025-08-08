use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlSelectElement, Request, RequestInit, RequestMode, Response};
use yew::prelude::*;
use yew_hooks::prelude::*;

use crate::components::*;
use crate::pages::UserInfo;
use crate::models::*;

#[function_component(FeedPage)]
pub fn feed_page() -> Html {
    let posts = use_state(|| Vec::<Post>::new());
    let page = use_state(|| 1usize);
    let is_loading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let has_more = use_state(|| true);

    let saved_accounts =
        use_state(
            || match web_sys::window().and_then(|w| w.local_storage().ok()?) {
                Some(storage) => match storage.get_item("e621_accounts") {
                    Ok(Some(accounts_json)) => {
                        serde_json::from_str::<Vec<UserInfo>>(&accounts_json)
                            .unwrap_or_else(|_| vec![])
                    }
                    _ => vec![],
                },
                _ => vec![],
            },
        );
    let selected_user = use_state(|| Option::<UserInfo>::None);

    let fetch_page = {
        let posts = posts.clone();
        let page = page.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();
        let has_more = has_more.clone();
        let selected_user = selected_user.clone();

        Callback::from(move |_| {
            if *is_loading {
                return;
            }

            let current_user = (*selected_user).clone();
            let Some(user) = current_user else {
                error.set(Some("Select an account to load the feed.".to_string()));
                return;
            };
            let url = format!(
                "http://localhost:8080/recomendations/{}?page={}",
                user.id, *page
            );

            is_loading.set(true);
            error.set(None);

            let posts = posts.clone();
            let page = page.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();
            let has_more = has_more.clone();

            spawn_local(async move {
                match fetch_json::<Vec<Post>>(&url).await {
                    Ok(mut new_items) => {
                        if new_items.len() < 320 {
                            has_more.set(false);
                        }
                        let mut merged = (*posts).clone();
                        merged.append(&mut new_items);
                        posts.set(merged);
                        page.set(*page + 1);
                    }
                    Err(e) => error.set(Some(e)),
                }
                is_loading.set(false);
            });
        })
    };

    let on_select = {
        let saved_accounts = saved_accounts.clone();
        let selected_user = selected_user.clone();
        let posts = posts.clone();
        let page = page.clone();
        let has_more = has_more.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();
        let fetch_page_cb = fetch_page.clone();

        Callback::from(move |ev: Event| {
            let target = ev
                .target()
                .and_then(|t| t.dyn_into::<HtmlSelectElement>().ok());
            if let Some(sel) = target {
                let val = sel.value();
                if val.is_empty() {
                    selected_user.set(None);
                    posts.set(Vec::new());
                    page.set(1);
                    has_more.set(true);
                    error.set(None);
                    return;
                }
                if let Ok(id) = val.parse::<i64>() {
                    if let Some(user) = saved_accounts.iter().find(|u| u.id == id).cloned() {
                        selected_user.set(Some(user));
                        posts.set(Vec::new());
                        page.set(1);
                        has_more.set(true);
                        error.set(None);
                        if !*is_loading {
                            fetch_page_cb.emit(());
                        }
                    }
                }
            }
        })
    };

    let on_clear: Callback<MouseEvent> = {
        let selected_user = selected_user.clone();
        let posts = posts.clone();
        let page = page.clone();
        let has_more = has_more.clone();
        let error = error.clone();
        Callback::from(move |_| {
            selected_user.set(None);
            posts.set(Vec::new());
            page.set(1);
            has_more.set(true);
            error.set(None);
        })
    };

    {
        let saved_accounts = saved_accounts.clone();
        let selected_user = selected_user.clone();
        let posts = posts.clone();
        let page = page.clone();
        let has_more = has_more.clone();
        let error = error.clone();
        let fetch_page = fetch_page.clone();

        use_effect_once(move || {
            if selected_user.is_none() && !saved_accounts.is_empty() {
                selected_user.set(Some(saved_accounts[0].clone()));
                posts.set(Vec::new());
                page.set(1);
                has_more.set(true);
                error.set(None);
                fetch_page.emit(());
            }
            || ()
        });
    }

    let on_post_click = Callback::from(|id: i64| {
        let _ = web_sys::window()
            .and_then(|w| w.alert_with_message(&format!("Clicked post #{id}")).ok());
    });

    html! {
        <div class="container my-4">
            <h2 class="mb-3">{ "Latest Posts" }</h2>

            <SavedAccountsSelect
                selected_user={selected_user.clone()}
                is_loading={is_loading.clone()}
            />

            <div class="d-flex align-items-center justify-content-between mb-3">
                {
                    if let Some(u) = &*selected_user {
                        html! { <span class="text-muted small">{ format!("User: {} (ID: {})", u.name, u.id) }</span> }
                    } else {
                        html! { <span class="text-muted small">{ "No user selected" }</span> }
                    }
                }
                {
                    if !(*is_loading) && (*error).is_none() {
                        html! { <span class="text-muted small">{ format!("Loaded {} posts", posts.len()) }</span> }
                    } else { html!{} }
                }
            </div>

            // Error banner
            {
                if let Some(err) = &*error {
                    html! {
                        <div class="alert alert-danger d-flex justify-content-between align-items-center" role="alert">
                            <span>{ err }</span>
                            <button class="btn btn-sm btn-outline-light" onclick={{
                                let fetch_page = fetch_page.clone();
                                Callback::from(move |_| fetch_page.emit(()))
                            }}>
                                { "Retry" }
                            </button>
                        </div>
                    }
                } else { html!{} }
            }

            // Grid
            <div class="row g-3">
                {
                    posts.iter().map(|post| {
                        let p = post.clone();
                        html! {
                            <div class="col-12 col-sm-6 col-md-4 col-lg-3 d-flex">
                                <PostCard post={p} clickable={true} on_click={on_post_click.clone()} />
                            </div>
                        }
                    }).collect::<Html>()
                }
            </div>

            // Initial spinner
            {
                if *is_loading && posts.is_empty() {
                    html! {
                        <div class="d-flex justify-content-center my-4">
                            <div class="spinner-border" role="status">
                                <span class="visually-hidden">{ "Loading..." }</span>
                            </div>
                        </div>
                    }
                } else { html!{} }
            }

            // Load more
            <div class="d-flex justify-content-center my-4">
                {
                    if *has_more {
                        html! {
                            <button
                                class="btn btn-outline-primary"
                                onclick={{
                                    let fetch_page = fetch_page.clone();
                                    Callback::from(move |_| fetch_page.emit(()))
                                }}
                                disabled={*is_loading || selected_user.is_none()}
                            >
                                {
                                    if selected_user.is_none() {
                                        html!{ "Select an account" }
                                    } else if *is_loading {
                                        html!{ "Loading..." }
                                    } else {
                                        html!{ "Load more" }
                                    }
                                }
                            </button>
                        }
                    } else if !posts.is_empty() {
                        html! { <span class="text-muted">{ "You’re all caught up." }</span> }
                    } else {
                        html!{}
                    }
                }
            </div>
        </div>
    }
}

async fn fetch_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let window = web_sys::window().ok_or("No window available".to_string())?;

    let mut opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    // Perform fetch
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch promise rejected: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast Response".to_string())?;

    // HTTP error handling
    if !resp.ok() {
        return Err(format!(
            "HTTP error {} {}",
            resp.status(),
            resp.status_text()
        ));
    }

    // Read text body
    let text_promise = resp
        .text()
        .map_err(|e| format!("Failed to read response text: {:?}", e))?;
    let text_js = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Text promise rejected: {:?}", e))?;
    let text = text_js
        .as_string()
        .ok_or("Response text not a string".to_string())?;

    // Deserialize
    serde_json::from_str::<T>(&text).map_err(|e| format!("JSON parse error: {e}"))
}
