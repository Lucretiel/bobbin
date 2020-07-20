mod twitter;
mod views;

use std::net::IpAddr;
use std::path::PathBuf;

use horrorshow::prelude::*;
use structopt;
use warp::{self, Filter};

#[derive(Debug, Clone, structopt::StructOpt)]
struct Args {
    #[structopt(short, long, env = "PORT", help = "The port to serve on")]
    port: u16,

    #[structopt(short, long, help = "The IP address to bind to")]
    bind: IpAddr,

    #[structopt(short, long, help = "Directory containing all the static files")]
    static_dir: PathBuf,
}

#[paw::main]
#[tokio::main]
async fn main(args: Args) {
    // Pre-render the pages that never change
    let home: &'static str = Box::leak(Box::new(views::home().into_string().unwrap())).as_str();
    let faq: &'static str = Box::leak(Box::new(views::faq().into_string().unwrap())).as_str();

    let root = warp::path::end().map(move || warp::reply::html(home));
    let faq = warp::path!("faq").map(move || warp::reply::html(faq));
    let static_files = warp::path!("static" / ..).and(warp::fs::dir(args.static_dir));

    let service = root.or(faq).or(static_files);

    warp::serve(service).run((args.bind, args.port)).await
}
