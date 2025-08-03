use yew::{function_component, html, Html, NodeRef, Properties};

#[derive(Properties, PartialEq)]
pub struct TagChartProps {
    pub canvas_ref: NodeRef,
    pub visible: bool,
}

#[function_component(TagChartCard)]
pub fn tag_chart_card(props: &TagChartProps) -> Html {
    if !props.visible {
        return html! {};
    }

    html! {
        <div class="card mt-4">
            <div class="card-header bg-primary text-white">
                <h5 class="mb-0">{"Tag Analysis"}</h5>
            </div>
            <div class="card-body">
                <div class="chart-container" style="position: relative; height: 60vh;">
                    <canvas ref={props.canvas_ref.clone()} id="tagChart"></canvas>
                </div>
            </div>
        </div>
    }
}
