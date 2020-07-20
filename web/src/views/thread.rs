use crate::twitter::{
    thread::{get_thread, ThreadItem},
    TweetId, User,
};

use std::sync::Arc;

use horrorshow::{html, owned_html, prelude::*};
use reqwest;

#[derive(Debug, Clone)]
enum ThreadHeader {
    Conversation,
    Authored(Arc<User>),
}

impl Render for ThreadHeader {
    fn render<'a>(&self, tmpl: &mut TemplateBuffer<'a>) {
        match self {
            ThreadHeader::Conversation => {
                tmpl << html! {
                    h3: "Conversation";
                }
            }
            ThreadHeader::Authored(author) => {
                let handle = author.handle.as_ref();
                tmpl << html! {
                    a(
                        href=format_args!("https://twitter/com/{}", handle),
                        target="_blank"
                    ) {
                        h3(class="author-header") {
                            : "Thread by ";
                            span(class="author") {
                                span(class="author-name"): &author.display_name;
                                span(class="author-handle") {
                                    : "@";
                                    : handle;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl RenderMut for ThreadHeader {
    fn render_mut<'a>(&mut self, tmpl: &mut TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl RenderOnce for ThreadHeader {
    fn render_once(self, tmpl: &mut TemplateBuffer<'_>)
    where
        Self: Sized,
    {
        self.render(tmpl)
    }
}

/// The synchronous part of building a thread; once we have all the twitter
/// ids and an author, render to HTML
fn render_thread(
    thread: impl IntoIterator<Item = ThreadItem>,
    header: ThreadHeader,
) -> impl Template {
    let thread_items = thread.into_iter();

    owned_html! {
        div(class="container") {
            div(class="row") {
                div(class="col text-center") {
                    : header;
                }
            }
            div(class="row justify-content-center") {
                div(class="col") {
                    ul(class="list-unstyled") {
                        @ for tweet in thread_items {
                            blockquote(class="twitter-tweet", data-theme="light") {
                                a(href=format_args!("https://twitter.com/someone/{}", tweet.tweet_id().as_int()))
                            }
                        }
                    }
                }
            }
        }
    }
}

/*
pub async fn thread(
    client: &reqwest::Client,
    tail: TweetId,
    head: Option<TweetId>,
) -> impl Template {
    panic!()
}
*/
