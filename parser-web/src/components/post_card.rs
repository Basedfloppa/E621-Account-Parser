use std::rc::Rc;
use web_sys::window;
use yew::prelude::*;

use crate::models::{self, *};

#[derive(Properties, PartialEq)]
pub struct PostCardProps {
    pub post: Rc<Post>,
    #[prop_or_default]
    pub alt: Option<AttrValue>,
}

#[function_component(PostCard)]
pub fn post_card(props: &PostCardProps) -> Html {
    let post = &props.post;

    let img_url = preferred_image_url(post.as_ref());

    let alt_text = {
        let post = Rc::clone(post);
        let alt = props.alt.clone();
        use_memo((post.id, alt.clone()), move |_| {
            if let Some(alt) = alt {
                alt
            } else {
                let mut parts: Vec<&str> = Vec::new();
                parts.extend(post.tags.general.iter().map(String::as_str));
                parts.extend(post.tags.character.iter().map(String::as_str));
                parts.extend(post.tags.artist.iter().map(String::as_str));
                if parts.is_empty() {
                    AttrValue::from(format!("Post #{}", post.id))
                } else {
                    AttrValue::from(parts.join(", "))
                }
            }
        })
    };

    let (rating_label, rating_classes) = rating_badge_classes(&post.rating);

    let score_summary = post.score.total;
    let score_detail = AttrValue::from(format!("↑ {}   ↓ {}", post.score.up, post.score.down));

    let onclick = {
        let id = post.id;
        Callback::from(move |e: MouseEvent| {
            if e.button() == 1 {
                e.prevent_default();
                if let Some(win) = window() {
                    let _ = win.open_with_url_and_target(
                        &format!("{}/posts/{}", models::POST_API_BASE, id),
                        "_blank",
                    );
                }
            } else if e.button() == 0 {
                e.prevent_default();
                if let Some(win) = window() {
                    let _ = win.open_with_url(&format!("{}/posts/{}", models::POST_API_BASE, id));
                }
            }
        })
    };

    let root_classes = classes!("card", "h-100", "overflow-hidden", "cursor-pointer");

    let inner: Html = html! {
        <>
            <div class="position-relative">
                {
                    if let Some(url) = img_url.clone() {
                        html! {
                            <img
                                class="card-img-top img-fluid"
                                src={url}
                                alt={(*alt_text).clone()}
                                loading="lazy"
                                decoding="async"
                            />
                        }
                    } else {
                        html! {
                            <div
                                class="bg-secondary text-white d-flex align-items-center justify-content-center"
                                style="aspect-ratio: 4 / 3;"
                                aria-label="No preview available"
                            >
                                { "No preview available" }
                            </div>
                        }
                    }
                }

                <span
                    class={classes!(rating_classes, "position-absolute", "top-0", "start-0", "m-2")}
                    title="Rating"
                    aria-label={format!("Rating {rating_label}")}
                >
                    { rating_label }
                </span>

                <span
                    class="badge bg-dark position-absolute bottom-0 end-0 m-2"
                    title={score_detail}
                    aria-label={format!("Score {score_summary}")}
                >
                    { score_summary }
                </span>
            </div>

            <div class="card-body p-2">
                <h6 class="card-title mb-1">{ format!("#{}", post.id) }</h6>
                {
                    if !post.tags.general.is_empty() {
                        html! {
                            <p class="card-text text-muted small mb-0">
                                { tag_preview(&post.tags.general, 3) }
                            </p>
                        }
                    } else {
                        html! { <p class="card-text text-muted small mb-0">{ "—" }</p> }
                    }
                }
            </div>
        </>
    };

    html! {
        <button
            type="button"
            class={root_classes}
            onmousedown={onclick}
            aria-label={format!("Post {}, rating {:?}, score {}", post.id, post.rating, post.score.total)}
        >
            { inner }
        </button>
    }
}

fn preferred_image_url(post: &Post) -> Option<AttrValue> {
    post.preview
        .as_ref()
        .and_then(|p| p.url.clone())
        .or_else(|| post.sample.as_ref().and_then(|s| s.url.clone()))
        .or_else(|| post.file.as_ref().and_then(|f| f.url.clone()))
        .map(AttrValue::from)
}

fn rating_badge_classes(r: &Rating) -> (&'static str, Classes) {
    match r {
        Rating::S => ("S", classes!("badge", "bg-success")),
        Rating::Q => ("Q", classes!("badge", "bg-warning", "text-dark")),
        Rating::E => ("E", classes!("badge", "bg-danger")),
    }
}

fn tag_preview(tags: &[String], n: usize) -> String {
    tags.iter()
        .take(n)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
