use super::{base::base_template, shared::Script, Stylesheet};
use crate::twitter::{
    thread::{example_thread, get_thread, Thread, ThreadAuthor, ThreadItem},
    TweetId, User, UserHandle,
};

use std::{iter, sync::Arc};

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
                    a(
                        href=author_url,
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
        let url = lazy_format!("https://twitter.com/someone/status/{}", self.id.as_int());

        tmpl << html! {
            div(class="tweet-container") {
                // This is a hack on the standard twitter embed widget. In the
                // future we'll do this with javascript
                blockquote(class="twitter-tweet", data-conversation="none") {
                    a(href=url)
                }
            }
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

    let twitter_js = Some(Script::Script {
        src: "https://platform.twitter.com/widgets.js",
        asinc: true,
        defer: false,
    });

    let thread_css = Some(Stylesheet::new("/static/thread.css"));

    let content = owned_html! {
        script(src="https://platform.twitter.com/widgets.js", charset="utf-8", async);
        div(class="container") {
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
        }
    };

    base_template(title, thread_css, twitter_js, content)
}

pub async fn thread(
    client: &reqwest::Client,
    tail: TweetId,
    head: Option<TweetId>,
) -> reqwest::Result<String> {
    let thread = example_thread();
    Ok(render_thread(thread).into_string().unwrap())
}
