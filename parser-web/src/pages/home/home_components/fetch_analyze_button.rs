use yew::{function_component, html, Callback, Html, MouseEvent, Properties};

#[derive(Properties, PartialEq)]
pub struct AnalyzeButtonProps {
    pub on_analyze: Callback<MouseEvent>,
    pub on_fetch: Callback<MouseEvent>,
    pub is_loading: bool,
    pub is_disabled: bool,
}

#[function_component(FetchAnalyzeButton)]
pub fn fetch_analyze_button(props: &AnalyzeButtonProps) -> Html {
    html! {
        <div class="d-grid gap-2 mb-4">
            <button
                class="btn btn-warning"
                onclick={props.on_analyze.clone()}
                disabled={props.is_loading || props.is_disabled}
            >
                {if props.is_loading {
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
                onclick={props.on_fetch.clone()}
                disabled={props.is_loading || props.is_disabled}
            >
                {if props.is_loading {
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