use horrorshow::{helper::doctype, owned_html, prelude::*};

pub(super) fn base_template<'a>(
    title: impl AsRef<str>,
    meta: impl RenderOnce,
    main_content: impl RenderOnce,
) -> impl Template {
    owned_html! {
        : doctype::HTML;
        html(prefix="og: https://ogp.me/ns#") {
            head {
                title {
                    : title.as_ref();
                }

                meta(charset="utf-8");
                meta(name="viewport", content="width=device-width, initial-scale=1");

                script(src="/static/js/common.js", async, charset="utf-8");
                script(src="/static/js/nav.js", async, charset="utf-8");

                :meta;
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
                    footer(class="footer", id="bobbin-footer") {
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
