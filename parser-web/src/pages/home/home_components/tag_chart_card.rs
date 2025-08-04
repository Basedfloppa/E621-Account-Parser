use crate::TagCount;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, MutationObserver, MutationObserverInit, js_sys,
};
use yew::use_memo;
use yew::{
    Callback, Html, NodeRef, Properties, classes, function_component, html, use_effect, use_state,
};

#[derive(Properties, PartialEq)]
pub struct TagChartCardProps {
    pub canvas_ref: NodeRef,
    pub visible: bool,
    pub tag_counts: Vec<TagCount>,
}

#[function_component(TagChartCard)]
pub fn tag_chart_card(props: &TagChartCardProps) -> Html {
    let theme_trigger = use_state(|| 0);
    let selected_group = use_state(|| String::new());
    let current_tags = use_memo(
        (selected_group.clone(), props.tag_counts.clone()),
        |(group, tags)| {
            tags.iter()
                .filter(|tag| tag.group_type == *group.clone())
                .cloned()
                .collect::<Vec<TagCount>>()
        },
    );
    let current_tags = (*current_tags).clone();
    let group_types = use_memo(props.tag_counts.clone(), |tag_counts: &Vec<TagCount>| {
        let mut groups: Vec<String> = tag_counts
            .iter()
            .map(|tag| tag.group_type.clone())
            .collect();
        groups.sort();
        groups.dedup();
        groups
    });

    {
        let selected_group = selected_group.clone();
        let group_types = group_types.clone();

        use_effect(move || {
            let current = &*selected_group;

            if current.is_empty() || !group_types.contains(current) {
                if let Some(first) = group_types.get(0) {
                    selected_group.set(first.clone());
                }
            }

            || ()
        });
    }

    {
        let theme_version = theme_trigger.clone();

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
                        if attr_name == Some("data-bs-theme".to_string())
                            || attr_name == Some("class".to_string())
                        {
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

            move || {
                observer.disconnect();
                callback.forget();
            }
        });
    }

    {
        let canvas_ref = props.canvas_ref.clone();
        let current_tags = current_tags.clone();
        let theme_trigger = *theme_trigger; // primitive, copied

        use_effect(move || {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                resize_canvas(&canvas, current_tags.len());
                draw_chart(&canvas, &current_tags);
            }
            || ()
        });
    }

    let on_tab_click = {
        let selected_group = selected_group.clone();
        Callback::from(move |group: String| {
            selected_group.set(group);
        })
    };

    if !props.visible {
        return html! {};
    }

    html! {
        <div class="card mt-4">
            <div class="card-header bg-primary text-white">
                <h5 class="mb-0">{"Tag Analysis"}</h5>
            </div>
            <div class="card-body">
                <ul class="nav nav-tabs mb-3">
                    {
                        for group_types.iter().map(|group| {
                            let is_active = *group == *selected_group;
                            let group_clone = group.clone();
                            html! {
                                <li class="nav-item">
                                    <button
                                        class={classes!("nav-link", if is_active { "active" } else { "" })}
                                        onclick={on_tab_click.reform(move |_| group_clone.clone())}
                                    >
                                        { group }
                                    </button>
                                </li>
                            }
                        })
                    }
                </ul>

                <div class="chart-container" style="max-height: 80vh; max-width: 100%;">
                    <canvas
                        ref={props.canvas_ref.clone()}
                        style="display: block; width: 100%"
                    />
                </div>
            </div>
        </div>
    }
}

fn draw_chart(canvas: &web_sys::HtmlCanvasElement, tag_counts: &[TagCount]) {
    let window = web_sys::window().expect("no global window exists");
    let device_pixel_ratio = window.device_pixel_ratio();

    let logical_width = canvas.client_width() as f64;
    if logical_width == 0.0 || tag_counts.is_empty() {
        return;
    }

    let bar_spacing = 30.0;
    let top_padding = 30.0;
    let bottom_padding = 30.0;

    let ideal_height = top_padding + bottom_padding + (bar_spacing * tag_counts.len() as f64);
    let max_logical_height = 2048.0;
    let logical_height = ideal_height.min(max_logical_height);

    let physical_width = (logical_width * device_pixel_ratio).round();
    let physical_height = (logical_height * device_pixel_ratio).round();

    let prev_width = canvas.width();
    let prev_height = canvas.height();

    if prev_width != physical_width as u32 {
        canvas.set_width(physical_width as u32);
    }
    if prev_height != physical_height as u32 {
        canvas.set_height(physical_height as u32);
    }

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();

    if physical_height <= 32767.0 && physical_width <= 32767.0 {
        ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap();
        ctx.scale(device_pixel_ratio, device_pixel_ratio)
            .expect("Failed to scale context");
    }

    ctx.clear_rect(0.0, 0.0, logical_width, logical_height);

    if tag_counts.is_empty() {
        return;
    }

    ctx.set_font("bold 12px Arial");
    let mut max_left_text_width: f64 = 0.0;
    let mut max_right_text_width: f64 = 0.0;

    for tag in tag_counts {
        let name_w: f64 = ctx.measure_text(&tag.name).unwrap().width();
        let count_w: f64 = ctx.measure_text(&tag.count.to_string()).unwrap().width();
        max_left_text_width = max_left_text_width.max(name_w);
        max_right_text_width = max_right_text_width.max(count_w);
    }

    let left_padding = max_left_text_width + 20.0;
    let right_padding = max_right_text_width + 20.0;

    let chart_width = logical_width - left_padding - right_padding;
    if logical_width == 0.0 || tag_counts.is_empty() {
        return;
    }

    let chart_height = logical_height - top_padding - bottom_padding;
    let bar_height = bar_spacing.min(chart_height / tag_counts.len() as f64);
    let max_value = tag_counts.iter().map(|t| t.count).max().unwrap_or(1) as f64;

    let colors = [
        get_css_variable_value("--bs-primary").unwrap_or("#0d6efd".into()),
        get_css_variable_value("--bs-success").unwrap_or("#198754".into()),
        get_css_variable_value("--bs-info").unwrap_or("#0dcaf0".into()),
        get_css_variable_value("--bs-warning").unwrap_or("#ffc107".into()),
        get_css_variable_value("--bs-danger").unwrap_or("#dc3545".into()),
        get_css_variable_value("--bs-secondary").unwrap_or("#6c757d".into()),
        get_css_variable_value("--bs-dark").unwrap_or("#212529".into()),
    ];
    let text_color = get_css_variable_value("--bs-body-color").unwrap_or("#212529".into());

    for (i, tag) in tag_counts.iter().enumerate() {
        let y = top_padding + i as f64 * bar_spacing;
        if y > logical_height - bottom_padding {
            break;
        }

        let bar_length = (tag.count as f64 / max_value) * chart_width;

        ctx.set_fill_style_str(&colors[i % colors.len()]);
        ctx.fill_rect(
            left_padding,
            y + (bar_height - 20.0) / 2.0,
            bar_length,
            20.0,
        );

        ctx.set_fill_style_str(&text_color);
        ctx.set_text_align("right");
        ctx.set_text_baseline("middle");
        ctx.fill_text(&tag.name, left_padding - 10.0, y + bar_height / 2.0)
            .unwrap_or(());

        ctx.set_text_align("left");
        ctx.fill_text(
            &tag.count.to_string(),
            left_padding + bar_length + 10.0,
            y + bar_height / 2.0,
        )
        .unwrap_or(());
    }

    ctx.set_font("bold 14px Arial");
    ctx.set_text_align("center");
    ctx.fill_text("Tags", left_padding - 20.0, 15.0)
        .unwrap_or(());
    ctx.fill_text("Count", logical_width - right_padding + 20.0, 15.0)
        .unwrap_or(());
}

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

fn resize_canvas(canvas: &HtmlCanvasElement, tag_count_len: usize) {
    let window = web_sys::window().expect("no global window exists");
    let device_pixel_ratio = window.device_pixel_ratio();

    let logical_width = canvas.client_width() as f64;
    if logical_width == 0.0 || tag_count_len == 0 {
        return;
    }

    let bar_spacing = 30.0;
    let top_padding = 30.0;
    let bottom_padding = 30.0;

    let ideal_height = top_padding + bottom_padding + (bar_spacing * tag_count_len as f64);
    let max_logical_height = 2048.0;
    let logical_height = ideal_height.min(max_logical_height);

    let physical_width = (logical_width * device_pixel_ratio).round() as u32;
    let physical_height = (logical_height * device_pixel_ratio).round() as u32;

    if canvas.width() != physical_width {
        canvas.set_width(physical_width);
    }
    if canvas.height() != physical_height {
        canvas.set_height(physical_height);
    }
}
