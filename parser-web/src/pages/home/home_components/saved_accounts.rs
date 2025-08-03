use yew::{function_component, html, Callback, Event, Html, MouseEvent, Properties};

use crate::pages::UserInfo;

#[derive(Properties, PartialEq)]
pub struct SavedAccountsProps {
    pub saved_accounts: Vec<UserInfo>,
    pub selected_user: Option<UserInfo>,
    pub on_select: Callback<Event>,
    pub on_clear: Callback<MouseEvent>,
    pub is_loading: bool,
}

#[function_component(SavedAccountsSelect)]
pub fn saved_accounts_select(props: &SavedAccountsProps) -> Html {
    html! {
        <div class="mb-4">
            <label class="form-label">{"Select Saved Account"}</label>
            <div class="input-group">
                <select
                    class="form-select"
                    onchange={props.on_select.clone()}
                    disabled={props.is_loading}
                >
                    <option value="" selected={props.selected_user.is_none()}>
                        {"-- Select Account --"}
                    </option>
                    {for props.saved_accounts.iter().map(|acc| {
                        html! {
                            <option value={acc.id.to_string()}>
                                {format!("{} (ID: {})", acc.name, acc.id)}
                            </option>
                        }
                    })}
                </select>
                <button
                    class="btn btn-outline-secondary"
                    type="button"
                    onclick={props.on_clear.clone()}
                    disabled={props.is_loading}
                >
                    {"Clear"}
                </button>
            </div>
        </div>
    }
}
