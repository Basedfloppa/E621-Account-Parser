use reqwasm::http::Request;
use web_sys::HtmlInputElement;
use yew::{function_component, html, use_state, Callback, Html, InputEvent, Properties, TargetCast, UseStateHandle};

use crate::pages::UserInfo;

#[derive(Properties, PartialEq)]
pub struct UserSearchProps {
    pub found_user: UseStateHandle<Option<UserInfo>>,
    pub is_loading: UseStateHandle<bool>,
    pub api_base: String,
    pub error: UseStateHandle<Option<String>>,
}

#[function_component(UserSearchForm)]
pub fn user_search_form(props: &UserSearchProps) -> Html {
    let user_query = use_state(|| "".to_string());

    let on_input = {
        let user_query = user_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            user_query.set(input.value());
        })
    };

    let fetch_user = {
        let api_base = props.api_base.clone();
        let user_query = user_query.clone();
        let found_user = props.found_user.clone();
        let is_loading = props.is_loading.clone();
        let error = props.error.clone();

        Callback::from(move |_| {
            let query = user_query.to_string();
            if query.is_empty() {
                error.set(Some("Please enter a username or ID".to_string()));
                return;
            }

            is_loading.set(true);
            error.set(None);

            // Determine if query is numeric ID
            let is_id = query.parse::<i64>().is_ok();
            let url = if is_id {
                format!("{}/user/id/{}", api_base, query)
            } else {
                format!("{}/user/name/{}", api_base, query)
            };

            let found_user = found_user.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match Request::get(&url).send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<UserInfo>().await {
                                Ok(user) => {
                                    found_user.set(Some(user));
                                    error.set(None);
                                }
                                Err(e) => {
                                    error.set(Some(format!("Failed to parse user data: {}", e)));
                                }
                            }
                        } else {
                            let status = response.status();
                            let text = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Unknown error".into());
                            error.set(Some(format!("Error {}: {}", status, text)));
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Network error: {}", e)));
                    }
                }
                is_loading.set(false);
            });
        })
    };

    html! {
        <div class="mb-3">
            <label class="form-label">{"Search by Username or ID"}</label>
            <div class="input-group">
                <input
                    type="text"
                    class="form-control"
                    value={(*user_query).clone()}
                    oninput={on_input.clone()}
                    placeholder="Enter username or ID"
                    disabled={*props.is_loading}
                />
                <button
                    class="btn btn-primary"
                    type="button"
                    onclick={fetch_user.clone()}
                    disabled={*props.is_loading}
                >
                    {"Search"}
                </button>
            </div>
        </div>
    }
}
