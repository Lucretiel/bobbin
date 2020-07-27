use crate::{
    social_tags,
    twitter::{
        auth,
        thread::{get_thread, Thread, ThreadAuthor},
        TweetId,
    },
    views::base::base_template,
};

use horrorshow::{html, owned_html, prelude::*};
use lazy_format::lazy_format;
use reqwest;
use std::borrow::Cow;

#[derive(Debug, Clone)]
struct ThreadHeader<'a> {
    author: &'a ThreadAuthor,
}

impl Render for ThreadHeader<'_> {
    fn render<'a>(&self, tmpl: &mut TemplateBuffer<'a>) {
        match self.author {
            ThreadAuthor::Conversation => {
                tmpl << html! {
                    h3(class="author-header"): "Conversation";
                }
            }
            ThreadAuthor::Author(author) => {
                let handle = author.handle.as_ref();
                let author_url = lazy_format!("https://twitter.com/{}", handle);
                tmpl << html! {
                    h3(class="author-header") {
                        : "Thread by ";
                        span(class="author") {
                            a(
                                href=author_url,
                                target="_blank"
                            ) {
                                span(class="author-name"): &author.display_name;
                                :" ";
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

impl RenderMut for ThreadHeader<'_> {
    fn render_mut<'a>(&mut self, tmpl: &mut TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl RenderOnce for ThreadHeader<'_> {
    fn render_once(self, tmpl: &mut TemplateBuffer<'_>)
    where
        Self: Sized,
    {
        self.render(tmpl)
    }
}

/// The synchronous part of building a thread; once we have all the twitter
/// ids and an author, render to HTML
fn render_thread(thread: Thread) -> impl Template {
    // TODO: Arc here too
    let title = match thread.author() {
        ThreadAuthor::Author(author) => {
            Cow::Owned(format!("Thread by {} on Bobbin", author.display_name))
        }
        ThreadAuthor::Conversation => Cow::Borrowed("Twitter conversation on Bobbin"),
    };

    // TODO: publicify the content of thread; that will let us avoid some
    // of these clones
    let thread_meta = thread.meta().cloned();
    let meta_title = title.clone();

    // TODO: meta tag for thread author
    // TODO: meta tag for URL
    let meta = owned_html! {
        link(rel="stylesheet", href="/static/css/thread.css");
        script(src="https://platform.twitter.com/widgets.js", charset="utf-8", async);
        script(src="/static/js/thread.js", charset="utf-8", async);

        :social_tags! {
            s:title: meta_title.as_ref();
        };

        @if let Some(meta) = thread_meta {
            :social_tags! {
                m:description: &meta.description;
                s:image: &meta.image_url;
            };
        }
    };

    let content = owned_html! {
        div(class="container thread-container") {
            div(class="columns") {
                div(class="column has-text-centered") {
                    : ThreadHeader{ author: thread.author() };
                }
            }
            div(class="columns") {
                div(class="column") {
                    div(class="tweet-list") {
                        @ for item in thread.items() {
                            div(class="tweet-container", data-tweet-id=item.as_int()) {
                                div(class="fake-tweet tweet-failure hidden") {
                                    :"Error: failed to load tweet (tweet ID: ";
                                    :item.as_int();
                                    :")";
                                }
                            }
                        }
                    }
                }
            }
            div(class="columns") {
                div(class="column") {
                    div(class="tweet-like has-text-centered thread-end") {
                        span(class="strike") {
                            span(id="thread-end-message"): "Loading thread...";
                        }
                    }
                }
            }
        }
    };

    base_template(title, meta, content)
}

pub async fn thread(
    client: reqwest::Client,
    token: impl auth::Token,
    tail: TweetId,
    head: Option<TweetId>,
) -> http::Response<hyper::Body> {
    match get_thread(&client, &token, tail, head).await {
        Ok(thread) => {
            // TODO: Enumerate the failure mode here. It's not really documented
            // how this can fail, and I'm pretty sure it can't?
            // TODO: cache this; a thread page's HTML should always be
            // identical given a head and tail.
            // TODO: cache headers, see above.
            let thread_page = render_thread(thread).into_string().unwrap();
            http::Response::builder()
                .status(http::StatusCode::OK)
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(hyper::Body::from(thread_page))
                .unwrap()
        }
        Err(err) => {
            // TODO: there are a lot of specific error cases to handle here.
            // For now we show this rudimentary error page.
            let page = format!(
                "Error fetching thread (thread ID: {}): {}",
                tail.as_int(),
                err
            );
            http::Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .header(http::header::CONTENT_TYPE, "text/plain")
                .body(hyper::Body::from(page))
                .unwrap()
        }
    }
}
