// src/pages/home.rs
use reqwasm::http::Request;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlInputElement};
use yew::prelude::*;

mod home_components;

use home_components::*;

const API_BASE: &str = "http://localhost:8000";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TagCount {
    pub name: String,
    pub group_type: String,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UserInfo {
    pub id: i64,
    pub name: String,
}

#[function_component(HomePage)]
pub fn home_page() -> Html {
    // State management
    let user_query = use_state(|| String::new());
    let found_user = use_state(|| None::<UserInfo>);
    let is_loading = use_state(|| false);
    let tag_counts = use_state(|| Vec::<TagCount>::new());
    let error = use_state(|| None::<String>);
    let canvas_ref = use_node_ref();

    // Load saved accounts from localStorage
    let saved_accounts =
        use_state(
            || match web_sys::window().and_then(|w| w.local_storage().ok()?) {
                Some(storage) => match storage.get_item("e621_accounts") {
                    Ok(Some(accounts_json)) => {
                        serde_json::from_str::<Vec<UserInfo>>(&accounts_json).unwrap_or_else(|_| vec![])
                    }
                    _ => vec![],
                },
                _ => vec![],
            },
        );

    // Handler for user query input
    let on_user_input = {
        let user_query = user_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            user_query.set(input.value());
        })
    };

    // Handler for selecting a saved account
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
                    }));
                    user_query.set(account.name.clone());
                }
            }
        })
    };

    // Handler for clearing selection
    let clear_selection = {
        let found_user = found_user.clone();
        let user_query = user_query.clone();

        Callback::from(move |_| {
            found_user.set(None);
            user_query.set(String::new());
        })
    };

    // Handler for fetching user data
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

    // Handler for fetching tag data
    let fetch_tag_data = {
        let found_user = found_user.clone();
        let is_loading = is_loading.clone();
        let tag_counts = tag_counts.clone();
        let error = error.clone();
        let canvas_ref = canvas_ref.clone();

        Callback::from(move |_| {
            if found_user.is_none() {
                error.set(Some("No user selected".to_string()));
                return;
            }

            let user_id = found_user.as_ref().unwrap().id;
            is_loading.set(true);
            error.set(None);

            let tag_counts = tag_counts.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();
            let canvas_ref = canvas_ref.clone();

            wasm_bindgen_futures::spawn_local(async move {
                // First trigger data processing
                match Request::post(&format!("{}/process/{}", API_BASE, user_id))
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
                            is_loading.set(false);
                            return;
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Processing error: {}", e)));
                        is_loading.set(false);
                        return;
                    }
                }

                // Then fetch tag counts
                match Request::get(&format!("{}/account/{}/tag_counts", API_BASE, user_id))
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Vec<TagCount>>().await {
                                Ok(counts) => {
                                    tag_counts.set(counts);
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

                // Draw chart after state update
                if let Some(canvas) = canvas_ref.cast::<web_sys::HtmlCanvasElement>() {
                    draw_chart(&canvas, &*tag_counts);
                }
            });
        })
    };

    // Render UI
    html! {
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
                                on_click={fetch_tag_data}
                                is_loading={*is_loading}
                                is_disabled={found_user.is_none()}
                            />

                            <TagChartCard
                                canvas_ref={canvas_ref.clone()}
                                visible={!tag_counts.is_empty()}
                            />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

// Helper function to draw the bar chart
fn draw_chart(canvas: &web_sys::HtmlCanvasElement, tag_counts: &[TagCount]) {
    // Group counts by tag type
    let mut groups: HashMap<String, i64> = HashMap::new();
    for tag in tag_counts {
        *groups.entry(tag.group_type.clone()).or_insert(0) += tag.count;
    }

    // Sort groups by count descending
    let mut sorted_groups: Vec<_> = groups.into_iter().collect();
    sorted_groups.sort_by(|a, b| b.1.cmp(&a.1));

    // Prepare data for chart
    let labels: Vec<String> = sorted_groups.iter().map(|(k, _)| k.clone()).collect();
    let data: Vec<i64> = sorted_groups.iter().map(|(_, v)| *v).collect();

    // Get canvas context
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();

    // Clear canvas
    ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    // Chart dimensions
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    let padding = 50.0;
    let chart_width = width - padding * 2.0;
    let chart_height = height - padding * 2.0;

    // Find max value for scaling
    let max_value = data.iter().max().copied().unwrap_or(1) as f64;

    // Draw chart title
    ctx.set_font("bold 16px Arial");
    ctx.set_text_align("center");
    ctx.fill_text("Tag Count by Type", width / 2.0, padding / 2.0)
        .unwrap();

    // Draw axes
    ctx.begin_path();
    ctx.move_to(padding, padding);
    ctx.line_to(padding, height - padding);
    ctx.line_to(width - padding, height - padding);
    ctx.stroke();

    // Draw bars
    let bar_width = chart_width / data.len() as f64;
    let colors = [
        "#4e73df", "#1cc88a", "#36b9cc", "#f6c23e", "#e74a3b", "#858796", "#5a5c69",
    ];

    for (i, (label, value)) in sorted_groups.iter().enumerate() {
        let x = padding + i as f64 * bar_width;
        let bar_height = (*value as f64 / max_value) * chart_height;
        let y = height - padding - bar_height;

        // Draw bar
        ctx.set_fill_style_str(&colors[i % colors.len()]);
        ctx.fill_rect(x + 5.0, y, bar_width - 10.0, bar_height);

        // Draw value label
        ctx.set_fill_style_str(&"#000");
        ctx.set_font("12px Arial");
        ctx.set_text_align("center");
        ctx.fill_text(&value.to_string(), x + bar_width / 2.0, y - 10.0)
            .ok();

        // Draw category label
        ctx.save();
        ctx.translate(x + bar_width / 2.0, height - padding / 2.0)
            .unwrap();
        ctx.rotate(-std::f64::consts::PI / 4.0).unwrap();
        ctx.set_font("10px Arial");
        ctx.set_text_align("right");
        ctx.fill_text(label, 0.0, 0.0).ok();
        ctx.restore();
    }

    // Draw Y-axis labels
    ctx.set_fill_style_str(&"#000");
    ctx.set_font("12px Arial");
    ctx.set_text_align("right");
    ctx.set_text_baseline("middle");

    for i in 0..=5 {
        let value = max_value * (1.0 - i as f64 / 5.0);
        let y = padding + chart_height * (i as f64 / 5.0);
        ctx.fill_text(&format!("{:.0}", value), padding - 5.0, y)
            .ok();
    }
}
