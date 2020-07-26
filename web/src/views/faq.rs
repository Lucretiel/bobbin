use super::base::base_template;
use crate::social_tags;

use std::iter;

use horrorshow::{html, owned_html, prelude::*};

#[derive(Debug, Clone)]
pub struct Entry<A: Render> {
    slug: &'static str,
    question: &'static str,
    answer: A,
}

impl<A: Render> Render for Entry<A> {
    fn render<'a>(&self, tmpl: &mut TemplateBuffer<'a>) {
        tmpl << html! {}
    }
}

impl<A: Render> RenderMut for Entry<A> {
    fn render_mut<'a>(&mut self, tmpl: &mut TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl<A: Render> RenderOnce for Entry<A> {
    fn render_once(self, tmpl: &mut TemplateBuffer<'_>)
    where
        Self: Sized,
    {
        self.render(tmpl)
    }
}

macro_rules! question_list {
    ($(slug:$slug:expr, question:$question:expr, answer:$answer:expr;)*) => {
        owned_html! {
            dl {$(
                dt(class="faq-question", id=$slug) {
                    strong: $question;
                    a(class="hoverlink", href=format_args!("#{}", $slug)) {
                        i(class="fas fas-link");
                    }
                }
                dd(class="faq-answer"): $answer;
            )*}
        }
    }
}

// TODO: This content is totally static; figure out the best way to tell
// downstream clients to cache it. Also, figure out a way to move it totally
// into static content handling
pub fn faq() -> impl Template {
    let question_list = question_list! {
        slug: "what-is-this",
        question: "What is this?",
        answer: html! { span:
            "Bobbin is a way to easily share Twitter threads with your friends and family.";
        };

        slug: "how-does-it-work",
        question: "How does it work?",
        answer: html! {
            span {
                :"Bobbin threads are defined by the final tweet in the thread. When given \
                the final tweet in a thread, Bobbin follows the reply chain backwards, \
                towards the beginning of the thread, and displays the thread from the \
                beginning. It ignores tweets "; em: "after"; :" the final tweet, even if \
                they were posted by the author of the thread."
            }
        };

        slug: "load-times",
        question: "Why does it take a while for my thread to load?",
        answer: html! {
            span:
                "The first time a user shares a thread, Bobbin must look up each \
                individual tweet one-by-one, because Twitter doesn't currently provide a \
                way to look up whole threads. Internally, Bobbin stores the reply chain, \
                so subsequent loads of the thread should be faster.";
        };

        slug: "does-bobbin-store-tweets",
        question: "Does bobbin store my tweets?",
        answer: html! {
            span:
                "Nope! The only thing that bobbin stores is the 20+ digit tweet ID of \
                each tweet in the thread, plus your own User ID and some other metadata
                (such as API urls). It uses Twitter's own \"embedded tweet\" widget to
                actually display the tweet. We don't store any of your content, and any
                tweets you delete will not appear in the thread.";
        };

        slug: "why-is-it-called-bobbin",
        question: "Why is it called Bobbin?",
        answer: html! {
            span {
                : "Because a ";
                a(href="https://en.wikipedia.org/wiki/Bobbin"): "bobbin";
                : " is how you share thread."
            }
        };
    };

    base_template(
        "Bobbin FAQ",
        owned_html! {
            link(rel="stylesheet", href="/static/css/faq.css");
            : social_tags! {
                m:title: "Bobbin FAQ";
                m:description: "Share Twitter threads with Bobbin";
                f:type: "website";
                f:determiner: "the";
            };
        },
        owned_html! {
            section(class="section") {
                div(class="container", id="faq-content") {
                    h1(class="title"): "Frequently Asked Questions";
                    div(class="content"): question_list;
                }
            }
        },
    )
}
