use yew::prelude::*;

use crate::pages::{Post, Rating};

/// Props for the PostCard component.
#[derive(Properties, PartialEq)]
pub struct PostCardProps {
    pub post: Post,
    #[prop_or_default]
    pub on_click: Callback<i64>,
    #[prop_or(false)]
    pub clickable: bool,
    #[prop_or_default]
    pub alt: Option<String>,
}

#[function_component(PostCard)]
pub fn post_card(props: &PostCardProps) -> Html {
    let post = &props.post;

    let img_url = preferred_image_url(post);
    let alt_text = props.alt.clone().unwrap_or_else(|| {
        let mut parts: Vec<String> = Vec::new();
        parts.extend(post.tags.general.iter().cloned());
        parts.extend(post.tags.character.iter().cloned());
        parts.extend(post.tags.artist.iter().cloned());
        if parts.is_empty() {
            format!("Post #{}", post.id)
        } else {
            parts.join(", ")
        }
    });

    let (rating_label, rating_class) = rating_badge_class(&post.rating);
    let score_summary = format!("{}", post.score.total);
    let score_detail = format!("↑ {}   ↓ {}", post.score.up, post.score.down);

    // If clickable, attach handler that always emits; noop is safe.
    let onclick = {
        let cb = props.on_click.clone();
        let id = post.id;
        if props.clickable {
            Some(Callback::from(move |_| cb.emit(id)))
        } else {
            None
        }
    };

    let card_style = if props.clickable {
        "overflow:hidden; cursor:pointer;"
    } else {
        "overflow:hidden;"
    };

    html! {
        <div
            class="card h-100"
            style={card_style}
            // Only add onclick when clickable, to avoid confusing semantics.
            onclick={onclick}
            aria-label={format!("Post {}, rating {:?}, score {}", post.id, post.rating, post.score.total)}
            role={if props.clickable { "button" } else { "group" }}
        >
            <div class="position-relative">
                {
                    if let Some(url) = img_url {
                        html! {
                            <img
                                class="card-img-top img-fluid"
                                src={url}
                                alt={alt_text.clone()}
                                loading="lazy"
                            />
                        }
                    } else {
                        html! {
                            <div class="bg-secondary text-white d-flex align-items-center justify-content-center" style="height:200px;">
                                { "No preview available" }
                            </div>
                        }
                    }
                }

                <span class={classes!("badge", rating_class, "position-absolute", "top-0", "start-0", "m-2")} title="Rating">
                    { rating_label }
                </span>

                <span class="badge bg-dark position-absolute bottom-0 end-0 m-2" title={score_detail.clone()}>
                    { score_summary }
                </span>
            </div>

            <div class="card-body p-2">
                <h6 class="card-title mb-1">{ format!("#{}", post.id) }</h6>
                {
                    if !post.tags.general.is_empty() {
                        let preview_tags = tag_preview(&post.tags.general, 3);
                        html! {
                            <p class="card-text text-muted small mb-0">{ preview_tags }</p>
                        }
                    } else {
                        html! { <p class="card-text text-muted small mb-0">{ "—" }</p> }
                    }
                }
            </div>
        </div>
    }
}

fn preferred_image_url(post: &Post) -> Option<String> {
    post.preview
        .as_ref()
        .and_then(|p| p.url.clone())
        .or_else(|| post.sample.as_ref().and_then(|s| s.url.clone()))
        .or_else(|| post.file.as_ref().and_then(|f| f.url.clone()))
}

fn rating_badge_class(r: &Rating) -> (&'static str, &'static str) {
    match r {
        Rating::S => ("S", "bg-success"),
        Rating::Q => ("Q", "bg-warning text-dark"),
        Rating::E => ("E", "bg-danger"),
    }
}

fn tag_preview(tags: &[String], n: usize) -> String {
    tags.iter().take(n).cloned().collect::<Vec<_>>().join(", ")
}
