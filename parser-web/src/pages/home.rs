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
    let selected_user: UseStateHandle<Option<UserInfo>> = use_state(|| None::<UserInfo>);
    let is_loading: UseStateHandle<bool> = use_state(|| false);
    let tag_counts: UseStateHandle<Vec<TagCount>> = use_state(|| Vec::<TagCount>::new());
    let error: UseStateHandle<Option<String>> = use_state(|| None::<String>);
    let canvas_ref = use_node_ref();

    html! {
        <div>
            <div class="container mt-4">
                <div class="row justify-content-center">
                    <div class="col-lg-8">
                        <div class="card shadow-sm">
                            <div class="card-body">
                                <h1 class="card-title text-center mb-4">{"e621 Tag Analyzer"}</h1>

                                <SavedAccountsSelect
                                    selected_user={selected_user.clone()}
                                    is_loading={is_loading.clone()}
                                />

                                <UserSearchForm
                                    found_user={selected_user.clone()}
                                    error={error.clone()}
                                    api_base={API_BASE}
                                    is_loading={is_loading.clone()}
                                />

                                <UserInfoAlert
                                    user={selected_user.clone()}
                                    error={error.clone()}
                                />

                                <FetchAnalyzeButton
                                    tag_count={tag_counts.clone()}
                                    found_user={selected_user.clone()}
                                    error={error.clone()}
                                    api_base={API_BASE}
                                    is_loading={is_loading.clone()}
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
