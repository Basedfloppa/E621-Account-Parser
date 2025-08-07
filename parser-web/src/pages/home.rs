use reqwasm::http::Request;
use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::components::*;

const API_BASE: &str = "http://localhost:8080";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TagCount {
    pub name: String,
    pub group_type: String,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UserInfo {
    pub id: i64,
    pub name: String,
    pub api_key: String,
}

#[function_component(HomePage)]
pub fn home_page() -> Html {
    let user_query = use_state(|| String::new());
    let found_user: UseStateHandle<Option<UserInfo>> = use_state(|| None::<UserInfo>);
    let is_loading: UseStateHandle<bool> = use_state(|| false);
    let tag_counts: UseStateHandle<Vec<TagCount>> = use_state(|| Vec::<TagCount>::new());
    let error: UseStateHandle<Option<String>> = use_state(|| None::<String>);
    let canvas_ref = use_node_ref();

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

    let on_user_input = {
        let user_query = user_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            user_query.set(input.value());
        })
    };

    let on_account_select = {
        let saved_accounts = saved_accounts.clone();
        let found_user = found_user.clone();
        let user_query = user_query.clone();

        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let idx = select.selected_index() as usize;

            if idx > 0 {
                if let Some(account) = saved_accounts.get(idx - 1) {
                    found_user.set(Some(UserInfo {
                        id: account.id,
                        name: account.name.clone(),
                        api_key: account.api_key.clone(),
                    }));
                    user_query.set(account.name.clone());
                }
            }
        })
    };

    let clear_selection = {
        let found_user = found_user.clone();
        let user_query = user_query.clone();

        Callback::from(move |_| {
            found_user.set(None);
            user_query.set(String::new());
        })
    };

    let fetch_user = {
        let user_query = user_query.clone();
        let found_user = found_user.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();

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
                format!("{}/user/id/{}", API_BASE, query)
            } else {
                format!("{}/user/name/{}", API_BASE, query)
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
        <div>
            <div class="container mt-4">
                <div class="row justify-content-center">
                    <div class="col-lg-8">
                        <div class="card shadow-sm">
                            <div class="card-body">
                                <h1 class="card-title text-center mb-4">{"e621 Tag Analyzer"}</h1>

                                <SavedAccountsSelect
                                    saved_accounts={(*saved_accounts).clone()}
                                    selected_user={(*found_user).clone()}
                                    on_select={on_account_select}
                                    on_clear={clear_selection}
                                    is_loading={*is_loading}
                                />

                                <UserSearchForm
                                    user_query={(*user_query).clone()}
                                    on_input={on_user_input}
                                    on_search={fetch_user}
                                    is_loading={*is_loading}
                                />

                                <UserInfoAlert
                                    user={(*found_user).clone()}
                                    error={(*error).clone()}
                                />

                                <FetchAnalyzeButton
                                    tag_count={tag_counts.clone()}
                                    found_user={found_user}
                                    error={error}
                                    api_base={API_BASE}
                                    is_loading={is_loading}
                                />
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <TagChartCard
                canvas_ref={canvas_ref.clone()}
                tag_counts={tag_counts.clone()}
            />
        </div>
    }
}
