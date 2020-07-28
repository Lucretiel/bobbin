// TODO: Convert these to mod.rs
mod twitter;
mod views;

use std::{
    convert, fs,
    io::{self, Read},
    net::IpAddr,
    path::{self, PathBuf},
    sync::Arc,
};

use bytes::Bytes;
use chrono;
use futures::FutureExt;
use horrorshow::prelude::*;
use redis;
use reqwest;
use secrecy::{Secret, SecretString};
use structopt;
use warp::{self, Filter};

use twitter::{auth, TweetId};

/// Implements the CLI secret parsing strategy. The string is returned directly,
/// unless it starts with '@', in which case it treats the str as a path
/// to a file containing the secret.
fn parse_secret(input: &str) -> io::Result<SecretString> {
    if input.as_bytes().first().copied() == Some(b'@') {
        let path = &input[1..];
        let path = path::Path::new(path);
        let mut file = fs::File::open(path)?;
        let mut secret = String::new();
        let _ = file.read_to_string(&mut secret)?;
        Ok(SecretString::new(secret))
    } else {
        Ok(SecretString::new(input.to_string()))
    }
}

#[derive(Debug, Clone, structopt::StructOpt)]
#[structopt(
    setting = structopt::clap::AppSettings::ColorAuto,
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

    /// The Twitter oauth consumer key.
    ///
    /// If this starts with an @ character, it will instead be used as a path
    /// to a file containing the consumer secret (intended for use with
    /// file-based secret distribution systems, such as docker secret).
    #[structopt(long, env = "CONSUMER_KEY", parse(try_from_str=parse_secret))]
    consumer_key: SecretString,

    /// The Twitter oauth consumer secret
    ///
    /// If this starts with an @ character, it will instead be used as a path
    /// to a file containing the consumer secret (intended for use with
    /// file-based secret distribution systems, such as docker secret)
    #[structopt(long, env = "CONSUMER_SECRET", parse(try_from_str=parse_secret))]
    consumer_secret: SecretString,

    /// The redis server in which to cache tweets & thread data.
    ///
    /// Make sure to use this in production to ensure we don't run into
    /// Twitter API rate limiting
    #[structopt(short, long, parse(try_from_str=redis::Client::open))]
    // It's fine to parse this directly because redis::Client::open doesn't
    // actually do any network operations, it just fallibly parses the input
    redis: Option<redis::Client>,
}

/// Type inference helper function for `warp`. `warp` requires async handlers
/// to return a Result, so for cases where such a result is infallible, this
/// function is the equivalent of `Ok`, but with the error type fixed to
/// `Infallible`.
#[inline]
fn infallible<T>(thing: T) -> Result<T, convert::Infallible> {
    Ok(thing)
}

/// Tokio's proc macro #[tokio::main] substantially obfuscates compile errors
/// in main, so we have this be the actual main function and `main` just awaits
/// it
async fn run(args: Args) {
    // TODO: Check that static_dir exists

    // Pre-render the pages that never change. We allocate them into strings,
    // then deliberately memory leak them so that we get &'static str
    let home = Bytes::from(views::home().into_string().unwrap());
    let faq = Bytes::from(views::faq().into_string().unwrap());

    // Server start time. This is used for cache headers.
    let server_start = chrono::Utc::now();

    // Create a reqwest client for making API calls. Reqwest clients are
    // a simple arc around an inner client type, so this is cheaply cloneable
    // TODO: figure out if this unwrap can ever trigger
    let http_client = reqwest::Client::builder().build().unwrap();

    // Create our redis client
    // TODO: Use r2d2 connection pool. Need to create an async wrapper for it
    // (consider stjepang/blocking)

    let redis_client = args.redis.map(Arc::new);

    // Get an auth token
    // TODO: Set up the handlers to refresh the token if necessary
    let credentials = auth::Credentials {
        consumer_key: args.consumer_key,
        consumer_secret: args.consumer_secret,
    };

    // TODO: Wrap this in an Arc? It's ~120 bytes, but copying that might be
    // cheaper than atomic operations?
    let token = auth::generate_bearer_token(&http_client, &credentials)
        .await
        .expect("Couldn't get a bearer token");

    // Route: /
    let root = warp::path::end().map(move || warp::reply::html(home.clone()));

    // Route: /faq
    let faq = warp::path!("faq").map(move || warp::reply::html(faq.clone()));

    // Route: /thread/{thread_id}
    let thread = warp::path!("thread" / u64).and_then(move |tweet_id| {
        let http_client = http_client.clone();
        let redis_client = redis_client.clone();
        let token = token.clone();
        let tweet_id = TweetId::new(tweet_id);

        views::thread(http_client, redis_client, token, tweet_id, None)
            // and_then requires a Result
            .map(infallible)
    });

    // Route: /static/...
    let static_files = warp::path!("static" / ..).and(warp::fs::dir(args.static_dir));

    let service = root.or(faq).or(thread).or(static_files);

    warp::serve(service).run((args.bind, args.port)).await
}

#[allow(unused_braces)]
#[paw::main]
#[tokio::main]
async fn main(args: Args) {
    run(args).await
}
