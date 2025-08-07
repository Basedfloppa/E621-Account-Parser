use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod pages;

use components::*;
use pages::*;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/account")]
    Account,
    #[at("/feed")]
    Feed,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! { <pages::HomePage /> },
        Route::Account => html! { <Account /> },
        Route::Feed => html! { <pages::FeedPage />},
        Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <Header />
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
