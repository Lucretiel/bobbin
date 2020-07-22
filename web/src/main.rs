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

fn make_reject<T>(_: T) -> warp::Rejection {
    warp::reject::reject()
}

/// Tokio's proc macro #[tokio::main] substantially obfuscates errors in main,
/// so we have this be the actual main function and `main` just awaits it
async fn run(args: Args) {
    // Pre-render the pages that never change
    let home: &'static str = Box::leak(Box::new(views::home().into_string().unwrap())).as_str();
    let faq: &'static str = Box::leak(Box::new(views::faq().into_string().unwrap())).as_str();

    // Create a rewest client for making API calls
    let http_client = reqwest::Client::builder().build().unwrap();

    let root = warp::path::end().map(move || warp::reply::html(home));
    let faq = warp::path!("faq").map(move || warp::reply::html(faq));
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

    let static_files = warp::path!("static" / ..).and(warp::fs::dir(args.static_dir));

    let service = root.or(faq).or(thread).or(static_files);

    warp::serve(service).run((args.bind, args.port)).await
}

#[paw::main]
#[tokio::main]
async fn main(args: Args) {
    run(args).await
}
