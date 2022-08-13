//! Low level methods & types for the Twitter API.

use std::{
    collections::hash_map::HashMap,
    fmt::{self, Display, Formatter, Write},
    num::NonZeroU64,
    rc::Rc,
    str::FromStr,
};

use futures::{
    stream::{iter, FuturesUnordered},
    StreamExt, TryStreamExt,
};
use horrorshow::{Render, RenderMut, RenderOnce};
use itertools::Itertools;
use redis::ToRedisArgs;
use reqwest;
use serde::{Deserialize, Serialize};
use tracing::Instrument as _;
use url::Url;

use crate::{serialize_map, table::DedupeTable};

use super::auth::{ApplyToken as _, Token};

macro_rules! twitter_id_types {
    ($($Name:ident)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $Name(NonZeroU64);

        impl Display for $Name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $Name {
            type Err = <NonZeroU64 as FromStr>::Err;

            #[inline]
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
    )*};
}

twitter_id_types! {TweetId UserId}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    /// The user's ID
    pub id: UserId,

    /// The user's "real name"
    #[serde(rename = "name")]
    pub display_name: String,

    /// The user's @name
    #[serde(rename = "screen_name")]
    pub handle: String,

    #[serde(rename = "profile_image_url_https")]
    pub image_url: Url,
}

/// Helper struct for normalizing / deduplicating User objects. The idea is
/// that, since we're often receiving large sets of tweets from a single user,
/// we can save a lot of space by having all the Tweets have an Arc to a
/// single User instance.
///
// TODO: Bench to see if this is worth it; we still have to briefly pay the
// space cost because the user is fully deserialized either way.
pub(super) type UserTable = DedupeTable<UserId, User>;

// TODO: Replace all these strings with bytes::Bytes, which is a reference
// counted buffer that's cheaper to clone.

// TODO: Migrate to Twitter API 2.0 as soon as possible; it includes a
// conversation_id that's essentially the same as our cluster id that'll make
// it *much* easier to rebuild threads in fewer API calls.

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReplyInfo {
    pub id: TweetId,
    pub author: UserId,
}

#[derive(Debug, Clone)]
pub struct Tweet {
    pub id: TweetId,
    pub text: String,
    pub author: Rc<User>,
    pub reply: Option<ReplyInfo>,
    pub image_url: Option<Url>,
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
            author: user_table.dedup_item(raw.author.id, raw.author).clone(),
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
    url: Url,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEntities {
    // TODO: we only care about one media entity- consider a custom
    // deserializer that dumps the rest
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

/// Fetch a bunch of tweets with /statuses/lookup. Note that, because that API
/// call has a limit of 100 tweets per call, this may make several API calls
/// to fulfill all the tweets.
#[tracing::instrument(skip(client, token, tweet_ids))]
pub async fn get_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    tweet_ids: impl IntoIterator<Item = TweetId>,
    user_table: &mut UserTable,
) -> Result<HashMap<TweetId, Tweet>, reqwest::Error> {
    let chunks = tweet_ids.into_iter().chunks(100);

    // Collection of HTML fetch tasks, each responsible for 100 tweets. We rely
    // on client to constrain concurrency as necessary.
    let tasks = chunks.into_iter().map(|tweet_ids| {
        let id_list = tweet_ids.map(|id| id.0).join(",");
        let id_list = id_list.as_str();

        let request = client
            .get(LOOKUP_TWEETS_URL)
            .query(&serialize_map! {
                id: id_list,
                trim_user: true,
                include_entities: false,
            })
            .header("Accept", "application/json")
            .apply_token(token);

        async move { request.send().await?.error_for_status()?.json().await }
            .instrument(tracing::info_span!("get_tweets_chunk", tweet_ids = id_list))
    });

    // In the common case that we're not fetching more than 100 tweets, just
    // await the single task directly, rather than going through FuturesUnordered
    match tasks.at_most_one() {
        Ok(None) => Ok(HashMap::new()),
        Ok(Some(task)) => task.await.map(|raw_tweets: Vec<RawTweet>| {
            raw_tweets
                .into_iter()
                .map(|raw| Tweet::from_raw_tweet(raw, user_table))
                .map(|tweet| (tweet.id, tweet))
                .collect()
        }),
        Err(tasks) => {
            let tasks: FuturesUnordered<_> = tasks.collect();

            tasks
                .map_ok(|raw_tweets: Vec<RawTweet>| iter(raw_tweets).map(Ok))
                .try_flatten()
                .map_ok(|raw| Tweet::from_raw_tweet(raw, user_table))
                .map_ok(|tweet| (tweet.id, tweet))
                .try_collect()
                .await
        }
    }
}

const GET_TWEET_URL: &str = "https://api.twitter.com/1.1/statuses/show.json;";

#[tracing::instrument(skip(client, token))]
pub async fn get_tweet(
    client: &reqwest::Client,
    token: &impl Token,
    tweet_id: TweetId,
    user_table: &mut UserTable,
) -> Result<Tweet, reqwest::Error> {
    // TODO: Replace this with a dataloader
    // TODO: /statuses/lookup has a separate rate limit from /statuses/show, so
    // try both if one is rate limited.
    client
        .get(GET_TWEET_URL)
        .query(&serialize_map! {
            id: tweet_id,
            trim_user: true,
            include_entities: false,
        })
        .header("Accept", "application/json")
        .apply_token(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map(|raw: RawTweet| Tweet::from_raw_tweet(raw, user_table))
}

const USER_TIMELINE_URL: &str = "https://api.twitter.com/1.1/statuses/user_timeline";

#[tracing::instrument(skip(client, token))]
pub async fn get_user_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    user_id: UserId,
    max_id: TweetId,
    user_table: &mut UserTable,
) -> Result<Vec<Tweet>, reqwest::Error> {
    // TODO: check for certain kinds of recoverable errors (auth errors etc)
    client
        .get(USER_TIMELINE_URL)
        .query(&serialize_map! {
            user_id: user_id,
            max_id: max_id,
            count: 200,
            exclude_replies: "false",
            include_rts: "true",
        })
        .header("Accept", "application/json")
        .apply_token(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map(|raw_tweets: Vec<RawTweet>| {
            raw_tweets
                .into_iter()
                .map(move |raw| Tweet::from_raw_tweet(raw, user_table))
                .collect()
        })
}

const GET_USER_URL: &str = "https://api.twitter.com/1.1/users/show.json";

/// Get data for a specific user. Typically you won't need to call this, as
/// user data is included with tweet data in API calls, but we you *will* need
/// it if Redis excised the User data from the table.
///
/// It doesn't include a UserTable, as only one User is being fetched, and we
/// assume if the user was in the table you wouldn't be calling this in the
/// first place.
#[tracing::instrument(skip(client, token))]
pub async fn get_user(
    client: &reqwest::Client,
    token: &impl Token,
    user_id: UserId,
) -> Result<User, reqwest::Error> {
    client
        .get(GET_USER_URL)
        .query(&serialize_map! {
            include_entities: false,
            user_id: user_id,
        })
        .header("Accept", "application/json")
        .apply_token(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

/*
TODO:

- Error code 401 unauthorized; make one (1) attempt to refresh the token.
- Error code 420 or 429 rate limited: Page me
*/
