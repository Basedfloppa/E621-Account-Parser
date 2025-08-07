use yew::{function_component, html, Html};

use crate::ThemeToggle;

#[function_component(Header)]
pub fn header() -> Html {
    html!(
    <nav class="navbar navbar-expand-lg bg-body-tertiary border">
      <div class="container-fluid">
        <a class="navbar-brand" href="/">
          {"e621 Account parser"}
        </a>
        <ul class="col navbar-nav justify-content-start">
            <li class="nav-item">
                <a class="nav-link active" href="/">{"Home"}</a>
            </li>
            <li class="nav-item">
                <a class="nav-link" href="/account">{"Account"}</a>
            </li>
            <li class="nav-item">
                <a class="nav-link" href="/feed">{"Feed"}</a>
            </li>
        </ul>
        <ul class="col navbar-nav justify-content-end">
            <li>
                <ThemeToggle />
            </li>
        </ul>
      </div>
    </nav>
    )
}
