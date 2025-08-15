use crate::ThemeToggle;
use yew::{Html, classes, function_component, html};

#[function_component(Header)]
pub fn header() -> Html {
    let path = {
        let p = if let Some(win) = web_sys::window() {
            win.location()
                .pathname()
                .unwrap_or_else(|_| "/".to_string())
        } else {
            "/".to_string()
        };
        if p.len() > 1 {
            p.trim_end_matches('/').to_string()
        } else {
            p
        }
    };

    let is_active = |p: &str| -> bool {
        if p == "/" {
            path == "/"
        } else {
            let p = p.trim_end_matches('/');
            path == p || path.starts_with(&format!("{p}/"))
        }
    };

    html! {
        <nav class="navbar bg-body-tertiary border flex-nowrap">
            <div class="container-fluid d-flex align-items-center gap-3 flex-nowrap">
                <a class="navbar-brand text-nowrap" href="/">
                    {"e621 Account parser"}
                </a>
                <ul class="navbar-nav flex-row me-auto gap-2 flex-nowrap">
                    <li class="nav-item">
                        <a
                            class={classes!("nav-link", is_active("/").then_some("active"))}
                            aria-current={is_active("/").then_some("page")}
                            href="/"
                        >
                            {"Home"}
                        </a>
                    </li>
                    <li class="nav-item">
                        <a
                            class={classes!("nav-link", is_active("/account").then_some("active"))}
                            aria-current={is_active("/account").then_some("page")}
                            href="/account"
                        >
                            {"Account"}
                        </a>
                    </li>
                    <li class="nav-item">
                        <a
                            class={classes!("nav-link", is_active("/feed").then_some("active"))}
                            aria-current={is_active("/feed").then_some("page")}
                            href="/feed"
                        >
                            {"Feed"}
                        </a>
                    </li>
                </ul>
                <ul class="navbar-nav flex-row ms-auto flex-nowrap">
                    <li class="nav-item">
                        <ThemeToggle />
                    </li>
                </ul>
            </div>
        </nav>
    }
}
