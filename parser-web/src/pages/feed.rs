use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Request, RequestInit, RequestMode, Response};
use yew::prelude::*;

use crate::components::*;
use crate::models::*;
use crate::pages::UserInfo;

const API_BASE: &str = "http://localhost:8080";

#[function_component(FeedPage)]
pub fn feed_page() -> Html {
    let posts = use_state(|| Vec::<Post>::new());
    let page = use_state(|| 1usize);
    let is_loading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let has_more = use_state(|| true);
    let selected_user = use_state(|| Option::<UserInfo>::None);

    let fetch_page = {
        let posts = posts.clone();
        let page = page.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();
        let has_more = has_more.clone();
        let selected_user = selected_user.clone();

        Callback::from(move |_| {
            if *is_loading { return; }

            let Some(user) = (*selected_user).clone() else {
                error.set(Some("Select an account to load the feed.".to_string()));
                return;
            };

            let url = format!("{API_BASE}/recommendations/{}?page={}", user.id, *page);

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
                        let incoming = new_items.len();

                        use std::collections::HashSet;
                        let mut merged = (*posts).clone();
                        let mut seen: HashSet<i64> = merged.iter().map(|p| p.id).collect();
                        new_items.retain(|p| seen.insert(p.id));

                        let added = new_items.len();
                        if added > 0 {
                            merged.extend(new_items);
                            posts.set(merged);
                            page.set(*page + 1);
                        }

                        if incoming < crate::models::API_PAGE_SIZE || added == 0 {
                            has_more.set(false);
                        }
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                is_loading.set(false);
            });
        })
    };

    {
        let posts = posts.clone();
        let page = page.clone();
        let has_more = has_more.clone();
        let error = error.clone();
        let is_loading = is_loading.clone();
        let fetch_page = fetch_page.clone();

        use_effect_with((*selected_user).clone(), move |selected: &Option<UserInfo>| {
            if let Some(u) = selected {
                posts.set(Vec::new());
                page.set(1);
                has_more.set(true);
                error.set(None);
                is_loading.set(false);
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
                        html! { <span class="text-muted small" aria-live="polite">{ format!("Loaded {} posts", posts.len()) }</span> }
                    } else { html!{} }
                }
            </div>

            {
                if let Some(err) = &*error {
                    html! {
                        <div class="alert alert-danger d-flex justify-content-between align-items-center" role="alert" aria-live="polite">
                            <span>{ err }</span>
                            <button
                                class="btn btn-sm btn-outline-light"
                                type="button"
                                onclick={{
                                    let fetch_page = fetch_page.clone();
                                    Callback::from(move |_| fetch_page.emit(()))
                                }}
                            >
                                { "Retry" }
                            </button>
                        </div>
                    }
                } else { html!{} }
            }

            {
                if selected_user.is_some() && !*is_loading && error.is_none() && posts.is_empty() {
                    html! {
                        <div class="text-center text-muted my-5" aria-live="polite">
                            { "No posts yet." }
                        </div>
                    }
                } else { html!{} }
            }

            <div class="row g-3" aria-busy={(*is_loading).to_string()}>
                {
                    posts.iter().map(|post| {
                        let p = post.clone();
                        html! {
                            <div key={p.id} class="col-12 col-sm-6 col-md-4 col-lg-3 d-flex">
                                <PostCard post={p} clickable={true} on_click={on_post_click.clone()} />
                            </div>
                        }
                    }).collect::<Html>()
                }
            </div>

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

            <div class="d-flex justify-content-center my-4">
                {
                    if *has_more {
                        html! {
                            <button
                                class="btn btn-outline-primary"
                                type="button"
                                onclick={{
                                    let fetch_page = fetch_page.clone();
                                    Callback::from(move |_| fetch_page.emit(()))
                                }}
                                disabled={*is_loading || selected_user.is_none()}
                                aria-busy={(*is_loading).to_string()}
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

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch promise rejected: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast Response".to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP error {} {}", resp.status(), resp.status_text()));
    }

    let text_promise = resp
        .text()
        .map_err(|e| format!("Failed to read response text: {:?}", e))?;
    let text_js = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Text promise rejected: {:?}", e))?;
    let text = text_js
        .as_string()
        .ok_or("Response text not a string".to_string())?;

    serde_json::from_str::<T>(&text).map_err(|e| format!("JSON parse error: {e}"))
}
