use std::{sync::Arc, thread::ThreadId};

use crate::{
    table::DedupeTable,
    twitter::{api::User, auth::Token, Tweet, TweetId, UserId},
};

/// Helper struct for normalizing / deduplicating User objects. The idea is
/// that, since we're often receiving large sets of tweets from a single user,
/// we can save a lot of space by having all the Tweets have an Arc to a
/// single User instance.
pub(super) type UserTable = DedupeTable<UserId, User>;

#[derive(Debug, Clone)]
pub enum ThreadAuthor {
    Author(Arc<User>),
    Conversation,
}

#[derive(Debug, Clone)]
pub struct Meta {
    pub description: String,
    pub image_url: String,
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub items: Vec<TweetId>,
    pub author: ThreadAuthor,
    pub meta: Option<Meta>,
}

#[derive(Debug, Clone)]
enum TweetLookupResult {
    FoundTweet(Tweet),
    MissingTweet(TweetId),
}

impl TweetLookupResult {
    fn tweet_id(&self) -> TweetId {
        match *self {
            TweetLookupResult::FoundTweet(ref tweet) => tweet.id,
            TweetLookupResult::MissingTweet(id) => id,
        }
    }

    fn previous_tweet_id(&self) -> Option<TweetId> {
        match *self {
            TweetLookupResult::MissingTweet(..) => None,
            TweetLookupResult::FoundTweet(ref tweet) => tweet.reply.as_ref().map(|reply| reply.id),
        }
    }
}

struct ScratchTweet {}

struct TweetGetter<'a, T: Token> {
    client: &'a reqwest::Client,
    token: &'a T,
    redis: &'a mut redis::aio::Connection,

    /// A collection of tweets that have been retrieved from the API or Redis
    /// that might be associated with the thread we're building
    tweets: HashMap<TweetId, ScratchTweet>,
}

/// Main logic for constructing a thread.
pub async fn build_thread(
    client: &reqwest::Client,
    token: &impl Token,
    redis: &mut redis::aio::Connection,
    tail: TweetId,
    head: Option<TweetId>,
) -> Thread {
    // Threads are constructed from back to front; thread is populated,
    // then reversed
    let mut thread_items: Vec<TweetLookupResult> = Vec::new();

    let mut user_table = UserTable::new();
}
