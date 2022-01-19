//! Low level methods & types for the Twitter API.

use std::{
    collections::hash_map::HashMap,
    fmt::{self, Display, Formatter, Write},
    future::Future,
    iter,
    num::NonZeroU64,
    str::FromStr,
    sync::Arc,
};

use futures::{
    future::ready,
    stream::{iter, FuturesUnordered},
    StreamExt, TryFutureExt, TryStreamExt,
};
use horrorshow::{Render, RenderMut, RenderOnce};
use itertools::Itertools;
use redis::ToRedisArgs;
use reqwest;
use serde::{Deserialize, Serialize};

use crate::{serialize_static_map, table::DedupeTable};

use super::auth::Token;

macro_rules! twitter_id_types {
    ($($Name:ident)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
        #[serde(transparent)]
        pub struct $Name(NonZeroU64);

        impl Display for $Name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $Name {
            type Err = <NonZeroU64 as FromStr>::Err;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse().map(Self)
            }
        }

        impl Render for $Name {
            fn render<'a>(&self, buf: &mut horrorshow::TemplateBuffer<'a>) {
                // It's never necessary to escape digits, so skip the penalty of
                // escaping by using raw_writer.
                // This is infallible; any errors that occur are stored in the
                // TemplateBuffer and handled via a side channel.
                write!(buf.as_raw_writer(), "{}", self.0).unwrap();
            }
        }

        impl RenderMut for $Name {
            #[inline]
            fn render_mut<'a>(&mut self, buf: &mut horrorshow::TemplateBuffer<'a>) {
                self.render(buf)
            }
        }

        impl RenderOnce for $Name {
            #[inline]
            fn render_once(self, buf: &mut horrorshow::TemplateBuffer<'_>)
            where
                Self: Sized,
            {
                self.render(buf)
            }
        }

        impl ToRedisArgs for $Name {
            fn write_redis_args<W>(&self, out: &mut W)
            where
                W: ?Sized + redis::RedisWrite,
            {
                self.0.write_redis_args(out)
            }

            fn describe_numeric_behavior(&self) -> redis::NumericBehavior {
                redis::NumericBehavior::NumberIsInteger
            }
        }
    )*};
}

twitter_id_types! {TweetId UserId}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub id: UserId,

    /// The user's "real name"
    #[serde(rename = "name")]
    pub display_name: String,

    /// The user's @name
    #[serde(rename = "screen_name")]
    pub handle: String,

    // TODO: use a structured URI? Might be a performance penalty to parse and
    // unparse, but type safety is nice
    #[serde(rename = "profile_image_url_https")]
    pub image_url: String,
}

/// Helper struct for normalizing / deduplicating User objects. The idea is
/// that, since we're often receiving large sets of tweets from a single user,
/// we can save a lot of space by having all the Tweets have an Arc to a
/// single User instance.
pub(super) type UserTable = DedupeTable<UserId, User>;

// TODO: Replace all these strings with bytes::Bytes, which is a reference
// counted buffer that's cheaper to clone.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyInfo {
    pub id: TweetId,
    pub author: UserId,
}

#[derive(Debug, Clone)]
pub struct Tweet {
    pub id: TweetId,
    pub text: String,
    pub author: Arc<User>,
    pub reply: Option<ReplyInfo>,
    pub image_url: Option<String>,
}

impl Tweet {
    fn from_raw_tweet(raw: RawTweet, user_table: &mut UserTable) -> Self {
        Self {
            id: raw.id,
            reply: match (raw.reply_id, raw.reply_author_id) {
                (None, None) => None,
                (Some(id), Some(author)) => Some(ReplyInfo { id, author }),
                // TODO: Log an error here (tracing) and return None instead of panic
                _ => {
                    panic!("invalid response from twitter API: had a reply author but no reply id")
                }
            },
            author: user_table.dedup_item(raw.author.id, raw.author),
            text: raw.text,
            image_url: raw
                .entities
                .and_then(|e| e.media.into_iter().next())
                .map(|raw| raw.url),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawMedia {
    #[serde(rename = "media_url_https")]
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEntities {
    media: Vec<RawMedia>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTweet {
    pub id: TweetId,

    #[serde(rename = "user")]
    pub author: User,

    pub text: String,

    #[serde(rename = "in_reply_to_status_id")]
    pub reply_id: Option<TweetId>,

    #[serde(rename = "in_reply_to_user_id")]
    pub reply_author_id: Option<UserId>,

    #[serde(rename = "extended_entities")]
    entities: Option<RawEntities>,
}

const LOOKUP_TWEETS_URL: &'static str = "https://api.twitter.com/1.1/statuses/lookup.json";

/// Fetch a bunch of tweets with /statuses/lookup. Note that this only
/// fetches the first 100 tweets in the list, and silently drops the rest;
/// be sure to make multiple calls if necessary.
///
/// This could be an async fn, but because of how `get_tweets` is implemented,
/// we'd actually prefer the earlier parts (in particular, the iteration of
/// tweet_ids) to happen synchronously
fn get_raw_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    tweet_ids: impl IntoIterator<Item = TweetId>,
) -> impl Future<Output = Result<Vec<RawTweet>, reqwest::Error>> + 'static {
    let id_list = tweet_ids.into_iter().take(100).map(|id| id.0).join(",");

    #[derive(Serialize)]
    struct Query {
        #[serde(rename = "id")]
        id_list: String,
    }

    let request = client
        .get(LOOKUP_TWEETS_URL)
        .query(&Query { id_list })
        .query(&serialize_static_map!(
            trim_user: "false",
            include_entities: "false",
        ))
        .header("Accept", "application/json");

    let request = token.apply(request);

    request
        .send()
        .and_then(|response| ready(response.error_for_status()))
        .and_then(|response| response.json())
}

/// Fetch a bunch of tweets with /statuses/lookup. Note that, because that API
/// call has a limit of 100 tweets per call, this may make several API calls
/// to fulfill all the tweets.
pub async fn get_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    tweet_ids: impl IntoIterator<Item = TweetId>,
    user_table: &mut UserTable,
) -> Result<HashMap<TweetId, Tweet>, reqwest::Error> {
    let chunks = tweet_ids.into_iter().chunks(100);

    let tasks: FuturesUnordered<_> = chunks
        .into_iter()
        .map(|tweet_ids| get_raw_tweets(client, token, tweet_ids))
        .collect();

    tasks
        .map_ok(|raw_tweets| iter(raw_tweets).map(Ok))
        .try_flatten()
        .map_ok(|raw| Tweet::from_raw_tweet(raw, user_table))
        .map_ok(|tweet| (tweet.id, tweet))
        .try_collect()
        .await
}

pub async fn get_tweet(
    client: &reqwest::Client,
    token: &impl Token,
    tweet_id: TweetId,
    user_table: &mut UserTable,
) -> Result<Option<Tweet>, reqwest::Error> {
    // TODO: Replace this with a dataloader
    // TODO: /statuses/lookup has a separate rate limit from /statuses/show, so
    // try both if one is rate limited.
    get_raw_tweets(client, token, iter::once(tweet_id))
        .await
        .map(|mut raw_tweets| {
            raw_tweets
                .pop()
                .map(|raw| Tweet::from_raw_tweet(raw, user_table))
        })
}

const USER_TIMELINE_URL: &'static str = "https://api.twitter.com/1.1/statuses/user_timeline";

pub async fn get_user_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    user_id: UserId,
    max_id: TweetId,
    user_table: &mut UserTable,
) -> Result<Vec<Tweet>, reqwest::Error> {
    #[derive(Serialize)]
    struct Query {
        user_id: UserId,
        max_id: TweetId,
    }

    // TODO: parse the URL once, using lazy_static
    // TODO: check for certain kinds of recoverable errors (auth errors etc)
    let request = client
        .get(USER_TIMELINE_URL)
        .query(&Query { user_id, max_id })
        .query(&serialize_static_map!(
            count: 200,
            exclude_replies: "false",
            include_rts: "true",
        ))
        .header("Accept", "application/json");

    let request = token.apply(request);

    eprintln!("User Tweets: {:?}", request);

    let response_tweets: Vec<RawTweet> = request.send().await?.error_for_status()?.json().await?;

    let tweets = response_tweets
        .into_iter()
        .map(move |raw| Tweet::from_raw_tweet(raw, user_table))
        .collect();

    Ok(tweets)
}

/*
TODO:

- Error code 401 unauthorized; make one (1) attempt to refresh the token.
- Error code 420 or 429 rate limited: Page me
*/
