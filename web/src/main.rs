mod twitter;
mod views;

use std::{convert, net::IpAddr, path::PathBuf};

use futures::{FutureExt, TryFutureExt};
use horrorshow::prelude::*;
use reqwest;
use structopt;
use warp::{self, Filter, Reply};

use secrecy::Secret;
use twitter::{auth, TweetId};

fn parse_consumer_secret(s: &str) -> Secret<String> {
    Secret::new(String::from(s))
}

#[derive(Debug, Clone, structopt::StructOpt)]
#[structopt(
    setting = structopt::clap::AppSettings::ColoredHelp,
    setting = structopt::clap::AppSettings::UnifiedHelpMessage,
)]
struct Args {
    /// The port to serve on
    #[structopt(short, long, env = "PORT")]
    port: u16,

    /// The IP address to bind to
    #[structopt(short, long)]
    bind: IpAddr,

    /// Directory containing all the static files.
    ///
    /// This directory should contain the /js, /css, etc directories.
    #[structopt(short, long, default_value = "./static")]
    static_dir: PathBuf,

    /// The Twitter oauth consumer key
    #[structopt(long, env = "CONSUMER_KEY")]
    consumer_key: String,

    /// The Twitter oauth consumer secret
    #[structopt(
        long,
        env = "CONSUMER_SECRET",
        parse(from_str=parse_consumer_secret)
    )]
    consumer_secret: Secret<String>,
}

/// Type inference helper function for `warp`. `warp` requires async handlers
/// to return a Result, so for cases where such a result is infallible, this
/// function is the equivalent of `Ok`, but with the error type fixed to
/// `Infallible`.
#[inline]
fn infallible<T>(thing: T) -> Result<T, convert::Infallible> {
    Ok(thing)
}

/// Tokio's proc macro #[tokio::main] substantially obfuscates errors in main,
/// so we have this be the actual main function and `main` just awaits it
async fn run(args: Args) {
    // Pre-render the pages that never change.
    let home: &str = Box::leak(views::home().into_string().unwrap().into_boxed_str());
    let faq: &str = Box::leak(views::faq().into_string().unwrap().into_boxed_str());

    // Create a rewest client for making API calls
    let http_client = reqwest::Client::builder().build().unwrap();

    // Get an auth token
    // TODO: Set up the handlers to refresh the token if necessary
    let credentials = auth::Credentials {
        consumer_key: args.consumer_key,
        consumer_secret: args.consumer_secret,
    };

    // TODO: Wrap this in an Arc
    let token = auth::generate_bearer_token(&http_client, &credentials)
        .await
        .expect("Couldn't get a bearer token");

    // Route: /
    let root = warp::path::end().map(move || warp::reply::html(home));

    // Route: /faq
    let faq = warp::path!("faq").map(move || warp::reply::html(faq));

    // Route: /thread/{thread_id}
    let thread = warp::path!("thread" / u64).and_then(move |tweet_id| {
        let client = http_client.clone();
        let token = token.clone();
        let tweet_id = TweetId::new(tweet_id);

        views::thread(client, token, tweet_id, None)
            // and_then requires a Result
            .map(infallible)
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
