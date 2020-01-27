//! Simple methods for fetching tweets from the twitter API.

use std::collections::hash_map::{Entry, HashMap};
use std::sync::{Arc, Mutex};

use dataloader::{BatchFn, BatchFuture, Loader};
use futures::future::{self, FutureExt, TryFutureExt};
use joinery::prelude::*;
use joinery::separators::Comma;
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TweetId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UserId(i64);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    id: UserId,

    #[serde(rename = "name")]
    display_name: String,

    #[serde(rename = "screen_name")]
    handle: String,
}

/// Helper struct for normalizing / decuplicating User objects
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
    id: TweetId,
    author: UserId,
}

#[derive(Debug, Clone)]
pub struct Tweet {
    pub id: TweetId,
    pub author: Arc<User>,
    pub reply: Option<ReplyInfo>,
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

pub trait Token {
    fn apply(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder;
}

/// A dataloader implementation for twitter
#[derive(Debug)]
pub struct TweetLoader<'a, T> {
    token: &'a T,
    client: &'a reqwest::Client,
}

impl<'a, T: Token + Clone + Sync + Send> BatchFn<TweetId, Option<Tweet>> for TweetLoader<'a, T> {
    type Error = reqwest::Error;

    #[inline(always)]
    fn max_batch_size(&self) -> usize {
        100
    }

    fn load(&self, tweet_ids: &[TweetId]) -> BatchFuture<Option<Tweet>, reqwest::Error> {
        load_tweets(self.client.clone(), self.token.clone(), tweet_ids).boxed()
    }
}

/// Same as get_tweets, but adapted for the DataLoader interface.
async fn load_tweets(
    client: reqwest::Client,
    token: impl Token,
    tweet_ids: &[TweetId],
) -> Result<Vec<Option<Tweet>>, reqwest::Error> {
    let tweets = get_tweets(&client, &token, tweet_ids).await?;
    Ok(tweet_ids.iter().map(move |id| tweets.remove(id)).collect())
}

const LOOKUP_TWEETS_URL: &'static str = "https://api.twitter.com/1.1/statuses/lookup";

/// Fetch a bunch of tweets with /statuses/lookup. Note that this only
/// fetches the first 100 tweets in the slice, and silently drops the rest;
/// be sure to make multiple calls if necessary. Prefer get_many_tweets,
/// which doesn't have this limitation.
pub async fn get_tweets(
    client: &reqwest::Client,
    token: &impl Token,
    tweet_ids: &[TweetId],
) -> Result<HashMap<TweetId, Tweet>, reqwest::Error> {
    #[derive(Serialize)]
    struct Query {
        #[serde(rename = "id")]
        id_list: String,
        trim_user: &'static str,
        include_entities: &'static str,
    }

    let request = client
        .get(LOOKUP_TWEETS_URL)
        .query(&Query {
            id_list: tweet_ids[..100]
                .iter()
                .map(|id| id.0)
                .join_with(Comma)
                .to_string(),
            trim_user: "false",
            include_entities: "false",
        })
        .header("Accept", "application/json");

    let request = token.apply(request);

    let response_tweets: Vec<RawTweet> = request.send().await?.error_for_status()?.json().await?;

    let mut user_table = UserTable::new();

    let tweets = response_tweets
        .into_iter()
        .map(move |raw_tweet| Tweet {
            id: raw_tweet.id,
            author: user_table.get_user(raw_tweet.author),
            reply: match (raw_tweet.reply_id, raw_tweet.reply_author_id) {
                (Some(id), Some(author)) => Some(ReplyInfo { id, author }),
                (None, None) => None,
                // TODO: don't panic
                _ => {
                    panic!("invalid response from twitter API: had a reply author but no reply id")
                }
            },
        })
        .map(|tweet| (tweet.id, tweet))
        .collect();

    Ok(tweets)
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
        .map(move |raw_tweet| Tweet {
            id: raw_tweet.id,
            author: user_table.get_user(raw_tweet.author),
            reply: match (raw_tweet.reply_id, raw_tweet.reply_author_id) {
                (Some(id), Some(author)) => Some(ReplyInfo { id, author }),
                (None, None) => None,
                // TODO: don't panic
                _ => {
                    panic!("invalid response from twitter API: had a reply author but no reply id")
                }
            },
        })
        .collect();

    Ok(tweets)
}
