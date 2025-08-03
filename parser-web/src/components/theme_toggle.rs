use wasm_bindgen::prelude::*;
use web_sys::{wasm_bindgen::prelude::Closure, window, StorageEvent};
use yew::{function_component, html, use_effect_with, use_state, Callback, Html};

#[function_component(ThemeToggle)]
pub fn theme_toggle() -> Html {
    // Synchronously read theme on component initialization
    let is_light_theme = use_state(|| {
        let theme = window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
            .and_then(|s| s.get_item("theme").ok())
            .flatten()
            .unwrap_or_else(|| "light".to_string());

        // Immediately apply theme to document
        if let Some(doc_elem) = window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = doc_elem.set_attribute("data-bs-theme", &theme);
        }

        theme == "light"
    });

    // Handle theme changes from other tabs/windows
    use_effect_with((), {
        let is_light_theme = is_light_theme.clone();
        move |_| {
            // Create closure for event handler
            let handler = Closure::<dyn FnMut(StorageEvent)>::new(move |e: StorageEvent| {
                // Only respond to 'theme' key changes
                if e.key().as_deref() != Some("theme") {
                    return;
                }
                
                let theme = e.new_value().unwrap_or_else(|| "light".into());
                let is_light = theme == "light";
                is_light_theme.set(is_light);

                // Apply theme to document
                if let Some(doc_elem) = window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.document_element())
                {
                    let _ = doc_elem.set_attribute("data-bs-theme", &theme);
                }
            });

            // Add event listener
            window()
                .unwrap()
                .add_event_listener_with_callback(
                    "storage",
                    handler.as_ref().unchecked_ref()
                )
                .expect("Failed to add storage event listener");

            // Cleanup closure - will remove event listener and drop handler
            move || {
                window()
                    .unwrap()
                    .remove_event_listener_with_callback(
                        "storage",
                        handler.as_ref().unchecked_ref()
                    )
                    .expect("Failed to remove storage event listener");
                // handler is dropped here, releasing memory
            }
        }
    });

    let on_click = {
        let light_theme = is_light_theme.clone();

        Callback::from(move |_| {
            let new_theme = !*light_theme;
            light_theme.set(new_theme);

            let theme_str = if new_theme { "light" } else { "dark" };

            // Update document
            if let Some(doc_elem) = window()
                .and_then(|w| w.document())
                .and_then(|d| d.document_element())
            {
                let _ = doc_elem.set_attribute("data-bs-theme", theme_str);
            }

            // Update storage
            if let Some(storage) = window()
                .and_then(|w| w.local_storage().ok())
                .flatten()
            {
                let _ = storage.set_item("theme", theme_str);
            }
        })
    };

    html!(
        <button type="button" class="btn" onclick={on_click}>
            <i class={ if *is_light_theme {"bi bi-brightness-high"} else {"bi bi-moon"}}></i>
        </button>
    )
}