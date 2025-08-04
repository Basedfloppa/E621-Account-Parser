use yew::{classes, function_component, html, use_effect_with, use_state, Callback, Html, NodeRef, Properties};
use crate::{pages::home::draw_chart, TagCount};

#[derive(Properties, PartialEq)]
pub struct TagChartCardProps {
    pub canvas_ref: NodeRef,
    pub visible: bool,
    pub tag_counts: Vec<TagCount>,
}

#[function_component(TagChartCard)]
pub fn tag_chart_card(props: &TagChartCardProps) -> Html {
    let group_types: Vec<String> = {
        let mut groups: Vec<String> = props.tag_counts.iter()
            .map(|tag| tag.group_type.clone())
            .collect();
        groups.sort();
        groups.dedup();
        groups
    };

    let selected_group = use_state(|| group_types.get(0).cloned().unwrap_or_default());

    let on_tab_click = {
        let selected_group = selected_group.clone();
        Callback::from(move |group: String| {
            selected_group.set(group);
        })
    };

    let current_tags: Vec<TagCount> = props
        .tag_counts
        .iter()
        .filter(|tag| tag.group_type == *selected_group)
        .cloned()
        .collect();

    {
        let canvas_ref = props.canvas_ref.clone();
        let tag_data = current_tags.clone();
        use_effect_with(tag_data.clone(), move |_| {
            if let Some(canvas) = canvas_ref.cast::<web_sys::HtmlCanvasElement>() {
                let desired_bar_height = 40.0;
                let padding = 100.0;
                let total_height = (tag_data.len() as f64 * desired_bar_height + padding) as u32;
                canvas.set_height(total_height);

                draw_chart(&canvas, &tag_data);
            }
        });
    }

    if !props.visible {
        return html! {};
    }

    html! {
        <div class="card mt-4">
            <div class="card-header bg-primary text-white">
                <h5 class="mb-0">{"Tag Analysis"}</h5>
            </div>
            <div class="card-body">
                // Tabs
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

                // Chart container
                <div class="chart-container" style="position: relative; width: 100%;">
                    <canvas
                        ref={props.canvas_ref.clone()}
                        id="tagChart"
                        class="w-100"
                        style="display: block; height: auto;"
                    />
                </div>
            </div>
        </div>
    }
}
