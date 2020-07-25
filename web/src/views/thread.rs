use crate::{
    twitter::{
        thread::{example_thread, get_thread, Thread, ThreadAuthor, ThreadItem},
        TweetId, User, UserHandle,
    },
    views::{base::base_template, shared::Script, Stylesheet},
};

use arrayvec::ArrayVec;
use horrorshow::{html, owned_html, prelude::*};
use lazy_format::lazy_format;
use reqwest;

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
                        a(
                            href=author_url,
                            target="_blank"
                        ) {
                            span(class="author") {
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

#[derive(Debug, Clone)]
struct TweetStub {
    id: TweetId,
}

impl Render for TweetStub {
    fn render<'a>(&self, tmpl: &mut TemplateBuffer<'a>) {
        let tweet_id = self.id.as_int();
        let id = lazy_format!("thread-item-{}", tweet_id);

        tmpl << html! {
            div(class="tweet-container", data-tweet-id=tweet_id, id=id);
        }
    }
}

impl RenderMut for TweetStub {
    fn render_mut<'a>(&mut self, tmpl: &mut TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl RenderOnce for TweetStub {
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
    let title = match thread.author() {
        ThreadAuthor::Author(author) => format!("Thread by {} on Bobbin", author.display_name),
        ThreadAuthor::Conversation => format!("Twitter conversation on Bobbin"),
    };

    let scripts = ArrayVec::from([
        Script {
            src: "https://platform.twitter.com/widgets.js",
        },
        Script {
            src: "/static/js/thread.js",
        },
    ]);

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
                            : TweetStub { id: item.tweet_id() };
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

    base_template(
        title,
        Some(Stylesheet::new("/static/css/thread.css")),
        scripts,
        content,
    )
}

pub async fn thread(
    client: &reqwest::Client,
    tail: TweetId,
    head: Option<TweetId>,
) -> reqwest::Result<String> {
    let thread = example_thread();
    Ok(render_thread(thread).into_string().unwrap())
}
