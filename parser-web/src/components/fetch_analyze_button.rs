use reqwasm::http::Request;
use yew::{Callback, Html, Properties, UseStateHandle, function_component, html};

use crate::pages::{TagCount, UserInfo};

#[derive(Properties, PartialEq)]
pub struct AnalyzeButtonProps {
    pub found_user: UseStateHandle<Option<UserInfo>>,
    pub error: UseStateHandle<Option<String>>,
    pub api_base: String,
    pub tag_count: UseStateHandle<Vec<TagCount>>,
    pub is_loading: UseStateHandle<bool>,
}

#[function_component(FetchAnalyzeButton)]
pub fn fetch_analyze_button(props: &AnalyzeButtonProps) -> Html {
    let analyze_tags = {
        let api_base = props.api_base.clone();
        let found_user = props.found_user.clone();
        let is_loading = props.is_loading.clone();
        let error = props.error.clone();

        Callback::from(move |_| {
            let api_base = api_base.clone();
            if found_user.is_none() {
                error.set(Some("No user selected".to_string()));
                return;
            }

            let user_id = found_user.as_ref().unwrap().id;
            is_loading.set(true);
            error.set(None);

            let is_loading = is_loading.clone();
            let error = error.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match Request::post(&format!("{}/process/{}", &api_base, user_id))
                    .send()
                    .await
                {
                    Ok(response) => {
                        if !response.ok() {
                            let status = response.status();
                            let text = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Unknown error".into());
                            error.set(Some(format!("Processing error {}: {}", status, text)));
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Processing error: {}", e)));
                    }
                }
                is_loading.set(false);
            });
        })
    };

    let fetch_tags = {
        let api_base = props.api_base.clone();
        let found_user = props.found_user.clone();
        let is_loading = props.is_loading.clone();
        let tag_count = props.tag_count.clone();
        let error = props.error.clone();

        Callback::from(move |_| {
            let api_base = api_base.clone();

            if found_user.is_none() {
                error.set(Some("No user selected".to_string()));
                return;
            }

            let user_id = found_user.as_ref().unwrap().id;
            is_loading.set(true);
            error.set(None);

            let tag_count = tag_count.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match Request::get(&format!("{}/account/{}/tag_counts", &api_base, user_id))
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Vec<TagCount>>().await {
                                Ok(counts) => {
                                    tag_count.set(counts);
                                    error.set(None);
                                }
                                Err(e) => {
                                    error.set(Some(format!("Failed to parse tag data: {}", e)));
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
        <div class="d-grid gap-2 mb-4">
            <button
                class="btn btn-warning"
                onclick={analyze_tags}
                disabled={*props.is_loading || props.found_user.is_none()}
            >
                {if *props.is_loading {
                    html! {
                        <span>
                            <span class="spinner-border spinner-border-sm me-2" role="status" aria-hidden="true"></span>
                            {"Analyzing..."}
                        </span>
                    }
                } else {
                    html! {"Analyze Tags"}
                }}
            </button>

            <button
                class="btn btn-success"
                onclick={fetch_tags}
                disabled={*props.is_loading || props.found_user.is_none()}
            >
                {if *props.is_loading {
                    html! {
                        <span>
                            <span class="spinner-border spinner-border-sm me-2" role="status" aria-hidden="true"></span>
                            {"Fetching..."}
                        </span>
                    }
                } else {
                    html! {"Fetch Tag Counts"}
                }}
            </button>
        </div>
    }
}