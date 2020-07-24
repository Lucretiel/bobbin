//! Simple methods for fe{ id: (), author: (), reply: ()}id: (), author: (), reply: ()}hing tweets from the twitter API.

pub mod auth;
pub mod thread;

use std::collections::hash_map::{Entry, HashMap};
use std::fmt::{self, Display, Formatter, Write};
use std::sync::Arc;

use joinery::prelude::*;
use joinery::separators::Comma;
use reqwest;
use serde::{Deserialize, Serialize};

use auth::Token;

// TODO: use more `Raw` types to firmly establish a construction boundary;
// only this module may create UserHandle, TweetId, etc
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TweetId(u64);

impl TweetId {
    pub fn as_int(&self) -> u64 {
        self.0
    }

    pub fn new(id: u64) -> Self {
        TweetId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UserId(u64);

impl UserId {
    pub fn as_int(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UserHandle(String);

impl AsRef<str> for UserHandle {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub id: UserId,

    #[serde(rename = "name")]
    pub display_name: String,

    #[serde(rename = "screen_name")]
    pub handle: UserHandle,
}

/// Helper struct for normalizing / deduplicating User objects
#[derive(Debug, Default)]
struct UserTable {
    table: HashMap<UserId, Arc<User>>,
}

impl UserTable {
    fn new() -> Self {
        Self::default()
    }

    /// Get an Arc<User> from a User (for instance, as returned from the
    /// twitter API.) If the existing entry's username / handle don't
    /// match the new entry, the entry is replaced, though this doesn't
    /// change any existing references.
    fn get_user(&mut self, user: User) -> Arc<User> {
        match self.table.entry(user.id) {
            Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                if **existing == user {
                    existing.clone()
                } else {
                    let replacement = Arc::new(user);
                    existing.clone_from(&replacement);
                    replacement
                }
            }
            Entry::Vacant(entry) => {
                let arc = Arc::new(user);
                entry.insert(arc.clone());
                arc
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplyInfo {
    pub id: TweetId,
    pub author: UserId,
}

#[derive(Debug, Clone)]
pub struct Tweet {
    pub id: TweetId,
    pub author: Arc<User>,
    pub reply: Option<ReplyInfo>,
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
            author: user_table.get_user(raw.author),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawTweet {
    pub id: TweetId,

    #[serde(rename = "user")]
    pub author: User,

    #[serde(rename = "in_reply_to_status_id")]
    pub reply_id: Option<TweetId>,

    #[serde(rename = "in_reply_to_user_id")]
    pub reply_author_id: Option<UserId>,
}

const LOOKUP_TWEETS_URL: &'static str = "https://api.twitter.com/1.1/statuses/lookup";

/// Fetch a bunch of tweets with /statuses/lookup. Note that this only
/// fetches the first 100 tweets in the list, and silently drops the rest;
/// be sure to make multiple calls if necessary.
pub async fn get_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    tweet_ids: impl IntoIterator<Item = TweetId>,
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
) -> Result<Option<Tweet>, reqwest::Error> {
    // TODO: Replace this with a dataloader
    get_tweets(client, token, Some(tweet_id))
        .await
        .map(|tweets| tweets.get(&tweet_id).cloned())
}

const USER_TIMELINE_URL: &'static str = "https://api.twitter.com/1.1/statuses/user_timeline";

pub async fn get_user_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    user_id: UserId,
    max_id: TweetId,
) -> Result<Vec<Tweet>, reqwest::Error> {
    #[derive(Serialize)]
    struct Query {
        user_id: UserId,
        count: u32,
        max_id: TweetId,
        exclude_replies: &'static str,
        include_rts: &'static str,
    }

    // TODO: parse the URL once and use a global Url object?
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

    let response_tweets: Vec<RawTweet> = request.send().await?.error_for_status()?.json().await?;

    let mut user_table = UserTable::new();

    let tweets = response_tweets
        .into_iter()
        .map(move |raw| Tweet::from_raw_tweet(raw, &mut user_table))
        .collect();

    Ok(tweets)
}

/*
TODO:

- Error code 401 unauthorized; make one (1) attempt to refresh the token.
- Error code 420 or 429 rate limited: Page me
*/

pub fn sample_thread() -> (Arc<User>, Vec<Tweet>) {
    let userId = UserId(7909592);

    let user = User {
        id: userId,
        display_name: "Lucretiel 🦀".to_string(),
        handle: UserHandle("Lucretiel".to_string()),
    };

    let user = Arc::new(user);

    let tweets = vec![
        Tweet {
            id: TweetId(1285393620091187200),
            author: user.clone(),
            reply: None,
        },
        Tweet {
            id: TweetId(1285393916825665537),
            author: user.clone(),
            reply: Some(ReplyInfo {
                id: TweetId(1285393620091187200),
                author: userId,
            }),
        },
        Tweet {
            id: TweetId(1285394118508765184),
            author: user.clone(),
            reply: Some(ReplyInfo {
                id: TweetId(1285393916825665537),
                author: userId,
            }),
        },
    ];

    (user, tweets)
}