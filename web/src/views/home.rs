use super::base::base_template;
use crate::social_tags;

use horrorshow::{owned_html, prelude::*};
use std::iter;

pub fn home() -> impl Template {
    let meta = owned_html! {
        link(rel="stylesheet", href="/static/css/index.css");
        script(src="/static/js/search.js", async, charset="utf-8");
        : social_tags! {
            m:title: "Bobbin";
            m:description: "Share Twitter threads with Bobbin";
            f:type: "website";
        };
    };

    let content = owned_html! {
        div {
            section(class="hero") {
                div(class="hero-body") {
                    div(class="container") {
                        h1(class="title has-text-centered") {
                            :"Share threads with ";
                            strong: "Bobbin";
                        }
                    }
                }
            }
            section(class="section") {
                div(class="container") {
                    form(id="tweet-entry-form") {
                        div(class="field") {
                            div(class="control has-icons-right") {
                                input(
                                    type="text",
                                    class="input transition",
                                    placeholder="Link to last tweet in thread",
                                    id="thread-input-field"
                                );
                                span(class="icon is-small is-right") {
                                    i(id="thread-input-icon");
                                }
                            }
                        }
                        div(class="field is-grouped is-grouped-centered") {
                            div(class="control") {
                                a(class="button is-info", id="help-button"): "Help";
                            }
                            div(class="control") {
                                a(class="button is-link disabled", id="thread-button"): "View Thread";
                            }
                        }
                    }
                }
            }
        }
    };

    base_template("Bobbin", meta, content)
}
