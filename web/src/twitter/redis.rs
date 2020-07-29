//! This module manages caching and retrieving tweet IDs in a redis store

/*
Schema overview

author:{AUTHOR_ID}:handle: string
author:{AUTHOR_ID}:display_name: string
author:{AUTHOR_ID}:image_url: string

# ^^^^^ Author keys expire

tweet:{TWEET_ID}:author: {AUTHOR_ID}
tweet:{TWEET_ID}:reply_to: {TWEET_ID} or empty string
tweet:{TWEET_ID}:image_url: string or empty string
tweet:{TWEET_ID}:thread: {THREAD_ID}

thread:{THREAD_ID}:tweets: Set of {TWEET_ID}

# ^^^^^ None of these keys need to expire, because they're all immutable
# over the lifetime of a tweet. The Thread set may change, but only in an
# additive way; it can never really be "wrong".
*/

// Additional design notes:
//
// Keys may all be (semi-)randomly expired, so all of our struct types are
// filled with Option.
//
// Unlike with twitter::api, our types here don't include their own IDs. This
// better reflects the schema, and also lends itself better to the HashMap
// design that permeates this interface.
//
// Thread IDs are redundant, but reduce redis pressure. They allow us to look
// up a whole thread with a single query. The ID is typically the same as the
// last tweet in the thread (this is how the UI presents a "thread ID") but
// may be anything unique. Part of the operation of this interface is to
// merge thread IDs if necessary. A thread ID may include more than 1 thread;
// formally, a thread ID in redis identifies a single connected tree of tweets.
// This could result in too much data being fetched, but in practice we assume
// that this is still better than one-at-a-time reply following, especially
// assuming that in practice there aren't many threads that share heads.
//
// TODO: use some analytics to verify this assumption.

use std::{collections::HashMap, sync::Arc};

use redis::{self, RedisResult};

use crate::twitter::api;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct ThreadId(u64);

#[derive(Debug, Clone)]
pub struct CachedAuthor {
    pub id: Option<api::UserId>,
    pub handle: Option<api::UserHandle>,
    pub display_name: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CachedTweet {
    pub author: Option<Arc<CachedAuthor>>,
    pub reply_to: Option<Option<String>>,
    pub image_url: Option<Option<String>>,
    thread_id: Option<ThreadId>,
}

// TODO: Connection pooling

/// Fetch as much data as possible about a single tweet, returning a collection
/// of CachedTweet and CachedAuthor. The caller then uses this information to
/// make API calls and send us stuff to cache.
///
/// Note that because a thread is just a tweet tail, this function can be used
/// to fetch the middle of threads
///
/// This function returns our best knowledge of *all* the data for a single
/// thread (notwithstanding concurrent writes from other tasks), so there's no
/// reason to call it again for the construction of a given thread.
///
/// If redis throws an error before we could get any data, we return the error;
/// otherwise, we return as much data as we were able to fetch.
///
// TODO: tracing for redis errors
pub async fn fetch_thread(
    client: &redis::Client,
    tail: api::TweetId,
) -> RedisResult<HashMap<api::TweetId, CachedTweet>> {
    todo!()
}

/// Save a bunch of tweets to redis, overwriting the existing data. Where
/// possible, send all of these as a single batch during a single thread
/// resolution, as it helps to pipeline requests & deduplicate serializing
/// of authors.
///
/// This function will return an error if any error occurred, but in practice
/// these errors can be ignored in the business logic, because the cache is
/// ephemeral by design.
// TODO: tracing for redis errors
pub async fn save_tweets(
    client: &redis::Client,
    tweets: impl IntoIterator<Item = &api::Tweet>,
) -> RedisResult<()> {
    todo!()
}
