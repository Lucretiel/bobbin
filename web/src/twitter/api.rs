//! Low level methods for the Twitter API.

use std::{
    collections::hash_map::{Entry, HashMap},
    fmt::{self, Display, Formatter, Write},
    num::NonZeroU64,
    str::FromStr,
    sync::Arc,
};

use horrorshow::{Render, RenderMut, RenderOnce};
use joinery::{prelude::*, separators::Comma};
use reqwest;
use serde::{Deserialize, Serialize};

use super::{auth::Token, table::DedupeTable};

// TODO: TweetId and UserId are basically identical; use a macro to dedupe
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TweetId(pub(super) NonZeroU64);

impl Display for TweetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for TweetId {
    type Err = <NonZeroU64 as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl Render for TweetId {
    fn render<'a>(&self, tmpl: &mut horrorshow::TemplateBuffer<'a>) {
        // It's never necessary to escape digits, so skip the penalty of
        // escaping by using raw_writer.
        // This is infallible; any errors that occur are stored in the
        // TemplateBuffer and handled via a side channel.
        write!(tmpl.as_raw_writer(), "{}", self.0).unwrap();
    }
}

impl RenderMut for TweetId {
    #[inline]
    fn render_mut<'a>(&mut self, tmpl: &mut horrorshow::TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl RenderOnce for TweetId {
    #[inline]
    fn render_once(self, tmpl: &mut horrorshow::TemplateBuffer<'_>)
    where
        Self: Sized,
    {
        self.render(tmpl)
    }
}

// TODO: convert this to NonZeroU64 so that optionals don't take up SIXTEEN
// BYTES.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UserId(pub(super) NonZeroU64);

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for UserId {
    type Err = <NonZeroU64 as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl Render for UserId {
    fn render<'a>(&self, tmpl: &mut horrorshow::TemplateBuffer<'a>) {
        // It's never necessary to escape digits, so skip the penalty of
        // escaping by using raw_writer.
        // This is infallible; any errors that occur are stored in the
        // TemplateBuffer and handled via a side channel.
        write!(tmpl.as_raw_writer(), "{}", self.0).unwrap();
    }
}

impl RenderMut for UserId {
    #[inline]
    fn render_mut<'a>(&mut self, tmpl: &mut horrorshow::TemplateBuffer<'a>) {
        self.render(tmpl)
    }
}

impl RenderOnce for UserId {
    #[inline]
    fn render_once(self, tmpl: &mut horrorshow::TemplateBuffer<'_>)
    where
        Self: Sized,
    {
        self.render(tmpl)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub id: UserId,

    #[serde(rename = "name")]
    pub display_name: String,

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
pub async fn get_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    tweet_ids: impl IntoIterator<Item = TweetId>,
    user_table: &mut UserTable,
) -> Result<HashMap<TweetId, Tweet>, reqwest::Error> {
    #[derive(Serialize)]
    struct Query {
        #[serde(rename = "id")]
        id_list: String,
        trim_user: &'static str,
        include_entities: &'static str,
    }

    let id_list = tweet_ids
        .into_iter()
        .take(100)
        .map(|id| id.0)
        .iter_join_with(Comma)
        .fold(String::new(), |mut id_list, item| {
            // Unwrap is fine here because write! to a String is infallible
            write!(&mut id_list, "{}", item).unwrap();
            id_list
        });

    let request = client
        .get(LOOKUP_TWEETS_URL)
        .query(&Query {
            id_list,
            trim_user: "false",
            include_entities: "false",
        })
        .header("Accept", "application/json");

    let request = token.apply(request);

    eprintln!("Single Tweet: {:?}", request);

    let response_tweets: Vec<RawTweet> = request.send().await?.error_for_status()?.json().await?;

    let mut user_table = UserTable::new();

    let tweets = response_tweets
        .into_iter()
        .map(move |raw| Tweet::from_raw_tweet(raw, &mut user_table))
        .map(|tweet| (tweet.id, tweet))
        .collect();

    Ok(tweets)
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
    get_tweets(client, token, Some(tweet_id), user_table)
        .await
        .map(|tweets| tweets.get(&tweet_id).cloned())
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
        count: u32,
        max_id: TweetId,
        exclude_replies: &'static str,
        include_rts: &'static str,
    }

    // TODO: parse the URL once, using lazy_static
    // TODO: check for certain kinds of recoverable errors (auth errors etc)
    let request = client
        .get(USER_TIMELINE_URL)
        .query(&Query {
            user_id,
            max_id,
            count: 200,
            exclude_replies: "false",
            include_rts: "true",
        })
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
