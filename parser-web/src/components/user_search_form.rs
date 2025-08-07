use yew::{function_component, html, Callback, Html, InputEvent, MouseEvent, Properties};

#[derive(Properties, PartialEq)]
pub struct UserSearchProps {
    pub user_query: String,
    pub on_input: Callback<InputEvent>,
    pub on_search: Callback<MouseEvent>,
    pub is_loading: bool,
}

#[function_component(UserSearchForm)]
pub fn user_search_form(props: &UserSearchProps) -> Html {
    html! {
        <div class="mb-3">
            <label class="form-label">{"Search by Username or ID"}</label>
            <div class="input-group">
                <input
                    type="text"
                    class="form-control"
                    value={props.user_query.clone()}
                    oninput={props.on_input.clone()}
                    placeholder="Enter username or ID"
                    disabled={props.is_loading}
                />
                <button
                    class="btn btn-primary"
                    type="button"
                    onclick={props.on_search.clone()}
                    disabled={props.is_loading}
                >
                    {"Search"}
                </button>
            </div>
        </div>
    }
}
