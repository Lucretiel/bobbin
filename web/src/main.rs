mod twitter;
mod views;

use std::net::IpAddr;
use std::path::PathBuf;

use futures::{FutureExt, TryFutureExt};
use horrorshow::prelude::*;
use reqwest;
use structopt;
use warp::{self, Filter, Reply};

use twitter::TweetId;

#[derive(Debug, Clone, structopt::StructOpt)]
struct Args {
    #[structopt(short, long, env = "PORT", help = "The port to serve on")]
    port: u16,

    #[structopt(short, long, help = "The IP address to bind to")]
    bind: IpAddr,

    #[structopt(short, long, help = "Directory containing all the static files")]
    static_dir: PathBuf,
}

/// Tokio's proc macro #[tokio::main] substantially obfuscates errors in main,
/// so we have this be the actual main function and `main` just awaits it
async fn run(args: Args) {
    // Pre-render the pages that never change.
    let home: &str = Box::leak(views::home().into_string().unwrap().into_boxed_str());
    let faq: &str = Box::leak(views::faq().into_string().unwrap().into_boxed_str());

    // Create a rewest client for making API calls
    let http_client = reqwest::Client::builder().build().unwrap();

    // Route: /
    let root = warp::path::end().map(move || warp::reply::html(home));

    // Route: /faq
    let faq = warp::path!("faq").map(move || warp::reply::html(faq));

    // Route: /thread/{thread_id}
    // TODO: How much of this should be part of the view? Probably the view
    // should return an HTTP response, and the closure should take care of
    // gathering the client, tweet ID, etc. While we're at it, the other views
    // should return an http::response too.
    let thread = warp::path!("thread" / u64).and_then(move |tweet_id| {
        let client = http_client.clone();
        async move {
            let tweet_id = TweetId::new(tweet_id);
            let response = match views::thread(&client, tweet_id, None).await {
                Ok(content) => http::Response::builder()
                    .status(200)
                    .header(http::header::CONTENT_TYPE, "text/html")
                    .body(hyper::Body::from(content))
                    .unwrap(),
                Err(err) => http::Response::builder()
                    .status(500)
                    .body(hyper::Body::empty())
                    .unwrap(),
            };

            if true {
                Ok(response)
            } else {
                Err(warp::reject::reject())
            }
        }
    });

    // Route: /static/...
    let static_files = warp::path!("static" / ..).and(warp::fs::dir(args.static_dir));

    let service = root.or(faq).or(thread).or(static_files);

    warp::serve(service).run((args.bind, args.port)).await
}

#[paw::main]
#[tokio::main]
async fn main(args: Args) {
    run(args).await
}
