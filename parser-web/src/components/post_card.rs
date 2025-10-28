use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{Element, window};
use yew::prelude::*;

use crate::models::*;

#[derive(Properties, PartialEq)]
pub struct PostCardProps {
    pub post: Rc<Post>,
    pub affinity: f32,
    #[prop_or_default]
    pub alt: Option<AttrValue>,
}

#[function_component(PostCard)]
pub fn post_card(props: &PostCardProps) -> Html {
    let post = &props.post;

    let root_ref = use_node_ref();
    let current_img_url = {
        let preview = post.preview.clone().unwrap();
        let url = preview.url.unwrap();
        let initial = Some(AttrValue::from(url.clone()));
        use_state(|| initial)
    };

    let resize_cb = use_mut_ref::<Option<Closure<dyn FnMut(UiEvent)>>, _>(|| None);

    {
        let root_ref = root_ref.clone();
        let post = Rc::clone(post);
        let current_img_url = current_img_url.clone();
        let resize_cb = resize_cb.clone();

        use_effect_with(post.id, move |_pid| {
            let choose = {
                let root_ref = root_ref.clone();
                let current_img_url = current_img_url.clone();
                let post = Rc::clone(&post);
                move |win: &web_sys::Window| {
                    let dpr = win.device_pixel_ratio();
                    let required_css_px = root_ref
                        .cast::<Element>()
                        .map(|el| el.client_width() as f64)
                        .unwrap_or(0.0);
                    let required_device_px = (required_css_px * dpr).ceil() as i64;

                    let new_url = preferred_image_url(post.as_ref(), required_device_px);

                    if *current_img_url != new_url {
                        current_img_url.set(new_url);
                    }
                }
            };

            if let Some(win) = window() {
                choose(&win);

                let cb = Closure::wrap(Box::new(move |_: UiEvent| {
                    if let Some(win) = window() {
                        choose(&win);
                    }
                }) as Box<dyn FnMut(_)>);

                let _ = win.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
                *resize_cb.borrow_mut() = Some(cb);
            }

            move || {
                if let (Some(win), Some(cb)) = (window(), resize_cb.borrow_mut().take()) {
                    let _ = win
                        .remove_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
                }
            }
        });
    }

    let img_url = (*current_img_url).clone();

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
        let cfg = read_config_from_head().unwrap();
        let id = post.id;
        Callback::from(move |e: MouseEvent| {
            if e.button() == 1 {
                e.prevent_default();
                if let Some(win) = window() {
                    let _ = win.open_with_url_and_target(
                        &format!("{}/posts/{}", cfg.posts_domain, id),
                        "_blank",
                    );
                }
            } else if e.button() == 0 {
                e.prevent_default();
                if let Some(win) = window() {
                    let _ = win.open_with_url(&format!("{}/posts/{}", cfg.posts_domain, id));
                }
            }
        })
    };

    let root_classes = classes!(
        "card",
        "h-100",
        "overflow-hidden",
        "cursor-pointer",
        "w-100"
    );

    let inner: Html = html! {
        <>
            <div class="position-relative card-body p-0">
                {
                    if let Some(url) = img_url {
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
                    class={classes!("badge", "rounded","bg-secondary","position-absolute", "top-0", "end-0", "m-2")} >
                    { format!("{:.2}",&props.affinity) }
                </span>

                <span
                    class={classes!("badge", "position-absolute", "bottom-0", "end-0", "m-2", if score_summary > 0 {"bg-success"} else {"bg-danger"})}
                    title={score_detail}
                >
                    { score_summary }
                </span>
            </div>

            <div class="card-text p-2">
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
            ref={root_ref}                               // <-- NEW
            onmousedown={onclick}
            aria-label={format!(
                "Post {}, rating {:?}, score {}, affinity {}",
                post.id, post.rating, post.score.total, &props.affinity
            )}
        >
            { inner }
        </button>
    }
}

fn preferred_image_url(post: &Post, required_width: i64) -> Option<AttrValue> {
    let mut candidates: Vec<(AttrValue, i64)> = Vec::new();

    if let Some(p) = post.preview.as_ref() {
        if let Some(u) = p.url.as_ref() {
            candidates.push((AttrValue::from(u.clone()), p.width));
        }
    }
    if let Some(s) = post.sample.as_ref() {
        if let Some(u) = s.url.as_ref() {
            candidates.push((AttrValue::from(u.clone()), s.width?));
        }
    }
    if let Some(f) = post.file.as_ref() {
        if let Some(u) = f.url.as_ref() {
            candidates.push((AttrValue::from(u.clone()), f.width));
        }
    }

    candidates.sort_by_key(|&(_, w)| w);
    if let Some((u, _)) = candidates
        .iter()
        .find(|&&(_, w)| w >= required_width)
        .cloned()
    {
        return Some(u);
    }
    candidates.last().map(|(u, _)| u.clone())
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
