use yew::prelude::*;
use reqwasm::http::Request;
use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;

#[derive(Serialize, Deserialize)]
struct AccountData {
    id: i64,
    name: String,
    api_key: String,
}

#[function_component(Account)]
pub fn account_creator() -> Html {
    let id = use_state(|| String::new());
    let name = use_state(|| String::new());
    let api_key = use_state(|| String::new());
    let message = use_state(|| String::new());
    let error = use_state(|| false);
    let loading = use_state(|| false);

    let on_id_change = {
        let id = id.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            id.set(input.value());
        })
    };

    let on_name_change = {
        let name = name.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            name.set(input.value());
        })
    };

    let on_api_key_change = {
        let api_key = api_key.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            api_key.set(input.value());
        })
    };

    let onsubmit = {
        let id = id.clone();
        let name = name.clone();
        let api_key = api_key.clone();
        let message = message.clone();
        let error = error.clone();
        let loading = loading.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            loading.set(true);
            
            // Validate inputs
            if id.is_empty() || name.is_empty() || api_key.is_empty() {
                message.set("All fields are required".to_string());
                error.set(true);
                loading.set(false);
                return;
            }
            
            let account_id = match id.parse::<i64>() {
                Ok(id) => id,
                Err(_) => {
                    message.set("Invalid account ID. Must be a number".to_string());
                    error.set(true);
                    loading.set(false);
                    return;
                }
            };
            
            // Prepare account data
            let account = AccountData {
                id: account_id,
                name: name.to_string(),
                api_key: api_key.to_string(),
            };
            
            // Clone state for async closure
            let message = message.clone();
            let error = error.clone();
            let loading = loading.clone();
            
            // Make API request with reqwasm
            wasm_bindgen_futures::spawn_local(async move {
                let response = Request::post("http://localhost:8080/account")
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&account).unwrap())
                    .send()
                    .await;
                
                loading.set(false);
                
                match response {
                    Ok(resp) => {
                        if resp.status() >= 200 && resp.status() < 300 {
                            message.set("Account created successfully!".to_string());
                            error.set(false);
                        } else {
                            let error_msg = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            message.set(format!("Error: {} (Status: {})", error_msg, resp.status()));
                            error.set(true);
                        }
                    }
                    Err(e) => {
                        message.set(format!("Network error: {}", e));
                        error.set(true);
                    }
                }
            });
        })
    };

    // Determine message class based on error state
    let message_class = if message.is_empty() {
        "d-none"
    } else if *error {
        "alert alert-danger mt-3"
    } else {
        "alert alert-success mt-3"
    };

    html! {
        <div class="container mt-5">
            <div class="row justify-content-center">
                <div class="col-md-6">
                    <div class="card shadow">
                        <div class="card-body">
                            <h1 class="card-title text-center mb-4">{"Create New Account"}</h1>
                            <form onsubmit={onsubmit}>
                                <div class="mb-3">
                                    <label for="account-id" class="form-label">{"Account ID"}</label>
                                    <input
                                        type="number"
                                        class="form-control"
                                        id="account-id"
                                        value={(*id).clone()}
                                        onchange={on_id_change}
                                        placeholder="Enter numeric account ID"
                                        disabled={*loading}
                                    />
                                </div>
                                
                                <div class="mb-3">
                                    <label for="account-name" class="form-label">{"Username"}</label>
                                    <input
                                        type="text"
                                        class="form-control"
                                        id="account-name"
                                        value={(*name).clone()}
                                        onchange={on_name_change}
                                        placeholder="Enter your username"
                                        disabled={*loading}
                                    />
                                </div>
                                
                                <div class="mb-3">
                                    <label for="api-key" class="form-label">{"API Key"}</label>
                                    <input
                                        type="password"
                                        class="form-control"
                                        id="api-key"
                                        value={(*api_key).clone()}
                                        onchange={on_api_key_change}
                                        placeholder="Enter your API key"
                                        disabled={*loading}
                                    />
                                </div>
                                
                                <button 
                                    type="submit" 
                                    class="btn btn-primary w-100"
                                    disabled={*loading}
                                >
                                    { if *loading { 
                                        html! {
                                            <span>
                                                <span class="spinner-border spinner-border-sm me-2" role="status" aria-hidden="true"></span>
                                                {"Creating..."}
                                            </span>
                                        } 
                                    } else { 
                                        "Create Account".into() 
                                    }}
                                </button>
                                
                                <div class={message_class} role="alert">
                                    {&*message}
                                </div>
                            </form>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}