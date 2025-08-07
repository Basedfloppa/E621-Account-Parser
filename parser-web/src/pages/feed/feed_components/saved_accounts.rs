use yew::prelude::*;

use crate::pages::UserInfo;

#[derive(Properties, PartialEq, Clone)]
pub struct SavedAccountsSelectProps {
    pub saved_accounts: Vec<UserInfo>,
    /// Controlled value: the currently selected user id as string ("" means none)
    pub value: String,
    pub on_select: Callback<Event>,
    pub on_clear: Callback<MouseEvent>,
    pub is_loading: bool,
}

#[function_component(SavedAccountsSelect)]
pub fn saved_accounts_select(props: &SavedAccountsSelectProps) -> Html {
    html! {
        <div class="mb-3 d-flex align-items-center gap-2">
            <select
                class="form-select"
                value={props.value.clone()}
                onchange={props.on_select.clone()}
                disabled={props.is_loading}
            >
                <option value="">{ "Select an account" }</option>
                {
                    props.saved_accounts.iter().map(|u| {
                        html! {
                            <option key={u.id.to_string()} value={u.id.to_string()}>
                                { &u.name }
                            </option>
                        }
                    }).collect::<Html>()
                }
            </select>

            <button
                class="btn btn-outline-secondary"
                onclick={props.on_clear.clone()}
                disabled={props.value.is_empty() || props.is_loading}
            >
                { "Clear" }
            </button>
        </div>
    }
}
