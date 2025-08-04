use reqwasm::http::Request;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{js_sys, CanvasRenderingContext2d, HtmlInputElement, MutationObserver, MutationObserverInit};
use yew::prelude::*;
use wasm_bindgen::{JsValue};

mod home_components;

use home_components::*;

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
    // State management
    let user_query = use_state(|| String::new());
    let found_user = use_state(|| None::<UserInfo>);
    let is_loading = use_state(|| false);
    let tag_counts = use_state(|| Vec::<TagCount>::new());
    let error = use_state(|| None::<String>);
    let canvas_ref = use_node_ref();
    let theme_version = use_state(|| 0);

    // Load saved accounts from localStorage
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
                        api_key: account.api_key.clone(),
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
            });
        })
    };

    // Setup theme change observer
    {
        let theme_version = theme_version.clone();
        
        use_effect(move || {
            let document = web_sys::window().unwrap().document().unwrap();
            let target = document.document_element().unwrap();
            
            let callback = Closure::<dyn FnMut(js_sys::Array, _)>::new(
                move |mutations: js_sys::Array, _: web_sys::MutationObserver| {
                    for i in 0..mutations.length() {
                        let mutation = mutations.get(i);
                        let mutation = match mutation.dyn_into::<web_sys::MutationRecord>() {
                            Ok(m) => m,
                            Err(_) => continue,
                        };
                        
                        let attr_name = mutation.attribute_name();
                        if attr_name == Some("data-bs-theme".to_string()) || attr_name == Some("class".to_string()) {
                            theme_version.set(*theme_version + 1);
                            break;
                        }
                    }
                },
            );
            
            let observer = MutationObserver::new(callback.as_ref().unchecked_ref())
                .expect("Failed to create observer");
            
            let options = MutationObserverInit::new();
            options.set_attributes(true);
            let _ = observer.observe_with_options(&target, &options);
            
            // Return cleanup closure
            move || {
                observer.disconnect();
                // Keep callback alive for observer lifetime
                callback.forget();
            }
        });
    }

    {
        let canvas_ref = canvas_ref.clone();
        let tag_counts = (*tag_counts).clone();
        let theme_ver = *theme_version;
        
        use_effect_with((tag_counts, theme_ver), move |(tag_counts, _)| {
            if let Some(canvas) = canvas_ref.cast::<web_sys::HtmlCanvasElement>() {
                draw_chart(&canvas, tag_counts);
            }
        });
    }

    // Render UI
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
                                    on_click={fetch_tag_data}
                                    is_loading={*is_loading}
                                    is_disabled={found_user.is_none()}
                                />
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <TagChartCard
                canvas_ref={canvas_ref.clone()}
                visible={!tag_counts.is_empty()}
                tag_counts={(*tag_counts).clone()}
            />
        </div>
    }
}

fn draw_chart(canvas: &web_sys::HtmlCanvasElement, tag_counts: &[TagCount]) {
    let window = web_sys::window().expect("no global window exists");
    let device_pixel_ratio = window.device_pixel_ratio();

    // Get logical size (CSS pixels)
    let logical_width = canvas.client_width() as f64;
    let logical_height = canvas.client_height() as f64;

    // Set physical size (scaled by device pixel ratio)
    canvas.set_width((logical_width * device_pixel_ratio) as u32);
    canvas.set_height((logical_height * device_pixel_ratio) as u32);

    // Maintain CSS size
    canvas
        .style()
        .set_property("width", &format!("{}px", logical_width))
        .unwrap();
    canvas
        .style()
        .set_property("height", &format!("{}px", logical_height))
        .unwrap();

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();

    // Scale context for high DPI displays
    ctx.scale(device_pixel_ratio, device_pixel_ratio)
        .expect("Failed to scale context");

    // Clear canvas in logical pixels
    ctx.clear_rect(0.0, 0.0, logical_width, logical_height);

    if tag_counts.is_empty() {
        return;
    }

    // Measure text to determine dynamic padding
    ctx.set_font("bold 12px Arial");
    let mut max_left_text_width: f64 = 0.0;
    let mut max_right_text_width: f64 = 0.0;

    for tag in tag_counts.iter() {
        let name_measure = ctx.measure_text(&tag.name).unwrap();
        let count_measure = ctx.measure_text(&tag.count.to_string()).unwrap();

        max_left_text_width = max_left_text_width.max(name_measure.width());
        max_right_text_width = max_right_text_width.max(count_measure.width());
    }

    let left_padding = max_left_text_width + 20.0; // 10px margin on each side
    let right_padding = max_right_text_width + 20.0;
    let top_padding = 30.0;
    let bottom_padding = 30.0;

    let chart_width = logical_width - left_padding - right_padding;
    let chart_height = logical_height - top_padding - bottom_padding;

    if chart_width <= 0.0 || chart_height <= 0.0 {
        return; // Not enough space to render
    }

    let mut sorted_tags = tag_counts.to_vec();
    sorted_tags.sort_by(|a, b| b.count.cmp(&a.count));

    let bar_height = (chart_height / sorted_tags.len() as f64).min(30.0);
    let max_value = sorted_tags.iter().map(|tag| tag.count).max().unwrap_or(1) as f64;

    // Get colors from CSS variables
    let colors = [
        get_css_variable_value("--bs-primary").unwrap_or("#0d6efd".to_string()),
        get_css_variable_value("--bs-success").unwrap_or("#198754".to_string()),
        get_css_variable_value("--bs-info").unwrap_or("#0dcaf0".to_string()),
        get_css_variable_value("--bs-warning").unwrap_or("#ffc107".to_string()),
        get_css_variable_value("--bs-danger").unwrap_or("#dc3545".to_string()),
        get_css_variable_value("--bs-secondary").unwrap_or("#6c757d".to_string()),
        get_css_variable_value("--bs-dark").unwrap_or("#212529".to_string()),
    ];

    let text_color = get_css_variable_value("--bs-body-color").unwrap_or("#212529".to_string());

    // Draw bars and text
    for (i, tag) in sorted_tags.iter().enumerate() {
        let y = top_padding + i as f64 * bar_height;
        let bar_length = (tag.count as f64 / max_value) * chart_width;

        // Draw bar
        ctx.set_fill_style_str(&colors[i % colors.len()]);
        ctx.fill_rect(
            left_padding,
            y + (bar_height - 20.0) / 2.0, // Center vertically
            bar_length,
            20.0, // Fixed bar height
        );

        // Draw tag name
        ctx.set_fill_style_str(&text_color);
        ctx.set_text_align("right");
        ctx.set_text_baseline("middle");
        ctx.fill_text(&tag.name, left_padding - 10.0, y + bar_height / 2.0)
            .unwrap();

        // Draw count
        ctx.set_text_align("left");
        ctx.fill_text(
            &tag.count.to_string(),
            left_padding + bar_length + 10.0,
            y + bar_height / 2.0,
        )
        .unwrap();
    }

    // Draw axis labels
    ctx.set_font("bold 14px Arial");
    ctx.set_text_align("center");
    ctx.fill_text("Tags", left_padding - 20.0, 15.0).unwrap();
    ctx.fill_text("Count", logical_width - right_padding + 20.0, 15.0)
        .unwrap();
}

// Helper function to get CSS variable values
fn get_css_variable_value(var_name: &str) -> Option<String> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let root = document.document_element()?;

    let computed_style = window.get_computed_style(&root).ok()??;

    computed_style
        .get_property_value(var_name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|v| !v.is_empty())
}
