use super::shared::{Script, Stylesheet};

use horrorshow::helper::doctype;
use std::borrow::Cow;

use horrorshow::owned_html;
use horrorshow::prelude::*;

const BULMA_CSS: Stylesheet = Stylesheet {
    href: "https://cdnjs.cloudflare.com/ajax/libs/bulma/0.9.0/css/bulma.min.css",
    integrity: Some("sha512-ADrqa2PY1TZtb/MoLZIZu/Z/LlPaWQeDMBV73EMwjGam43/JJ5fqW38Rq8LJOVGCDfrJeOMS3Q/wRUVzW5DkjQ=="),
    crossorigin: Some("anonymous"),
};

const FONTAWESOME: Stylesheet = Stylesheet {
    href: "https://use.fontawesome.com/releases/v5.8.2/css/all.css",
    integrity: Some("sha384-oS3vJWv+0UjzBfQzYUhtDYW+Pj2yciDJxpsK1OYPAYjqT085Qq/1cq5FLXAZQ7Ay"),
    crossorigin: Some("anonymous"),
};

pub(super) fn base_template<'a>(
    title: impl Into<Cow<'static, str>>,
    css: impl IntoIterator<Item = Stylesheet>,
    scripts: impl IntoIterator<Item = Script>,
    main_content: impl RenderOnce,
) -> impl Template {
    let title = title.into();
    let css_items = css.into_iter();
    let scripts = scripts.into_iter();

    owned_html! {
        : doctype::HTML;
        html {
            head {
                title {
                    : title.as_ref();
                }
                meta(charset="utf-8");
                meta(name="viewport", content="width=device-width, initial-scale=1");

                :BULMA_CSS;
                :FONTAWESOME;

                :Stylesheet::new("/static/base.css");

                @ for css_item in css_items {
                    : css_item
                }

                :Script::Module {
                    src: "/static/nav.mjs",
                };

                @ for script in scripts {
                    : script
                }
            }
            body {
                div(class="grow-main") {
                    nav(class="navbar is-dark", role="navigation", aria-label="main navigation") {
                        div(class="container") {
                            div(class="navbar-brand") {
                                a(class="navbar-item", href="/") {
                                    span(class="logo") {
                                        span(class="logo-label"): "Bobbin";
                                        span(class="beta-label"): "Beta 2";
                                    }
                                }
                                a(type="button", role="button", class="navbar-burger", id="nav-burger") {
                                    span(aria-hidden="true");
                                    span(aria-hidden="true");
                                    span(aria-hidden="true");
                                }
                            }
                            div(class="navbar-menu", id="navbar-links") {
                                div(class="navbar-start is-dark") {
                                    a(class="navbar-item", href="/"): "Home";
                                    a(class="navbar-item", href="/faq"): "FAQ";
                                }
                            }
                        }
                    }
                    main {
                        : main_content;
                    }
                    footer(class="footer is-dark") {
                        div(class="container") {
                            span(class="footer-item") {
                                a(href="https://github.com/Lucretiel/bobbin", target="_blank"):
                                    "Github";
                            }
                            span(class="footer-item") {
                                a(href="https://github.com/Lucretiel/bobbin/issues", target="_blank"):
                                    "Issues & Feedback";

                            }
                        }
                    }
                }
            }
        }
    }
}
