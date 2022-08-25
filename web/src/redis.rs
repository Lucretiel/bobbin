//! This module manages caching and retrieving tweet IDs in a redis store

/*
Schema overview

bobbin:user:{USER_ID}:blob {User Blob}
bobbin:tweet:{TWEET_ID}:blob {Tweet Blob}
bobbin:cluster:{TWEET_ID}:tweets {set of tweet IDs}

User keys expire because users can change their profile pic etc
*/

// Additional design notes:
//
// Data is packed with MessagePack (see the struct types later in this
// module)
//
// Unlike with twitter::api, our types here don't include their own IDs. This
// better reflects the schema, and also lends itself better to the HashMap
// design that permeates this interface.

use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::{self, Display, Formatter, Write as FmtWrite},
    hash::Hash,
};

use itertools::Itertools as _;
use redis::{self, ErrorKind as RedisErrorKind, RedisError};
use rmp_serde::{self, decode::Error as MpDeError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::{
    thread::Thread,
    twitter::api::{ReplyInfo, Tweet, TweetId, UserId},
};

mod schema {
    use std::fmt::Display;

    use lazy_format::lazy_format;

    use super::{ClusterId, TweetId, UserId};

    pub fn user_blob_key(user_id: UserId) -> impl Display {
        lazy_format!("bobbin:user:{}:blob", user_id)
    }

    pub fn tweet_blob_key(tweet_id: TweetId) -> impl Display {
        lazy_format!("bobbin:tweet:{}:blob", tweet_id)
    }

    pub fn cluster_key(cluster_id: ClusterId) -> impl Display {
        lazy_format!("bobbin:cluster:{}:tweets", cluster_id)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("error from redis")]
    Redis(#[from] RedisError),

    #[error("error deserializing from redis")]
    Decode(#[from] MpDeError),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClusterId(TweetId);

impl Display for ClusterId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Thread {
    pub fn cluster_id(&self) -> Option<ClusterId> {
        self.items.first().copied().map(ClusterId)
    }
}

// TODO: find a convenient abstraction for reading CachedUser and CachedTweet
// from redis responses (wh)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUser<S, U> {
    pub display_name: S,
    pub handle: S,
    pub image_url: U,
}

pub type OwnedCachedUser = CachedUser<String, Url>;
type BorrowedCachedUser<'a> = CachedUser<&'a str, &'a Url>;

// TODO: This schema meta-design makes no accounting for potential schema
// changes. For now we'll plan to do the ugly thing and erase the redis cache
// if we need to do any inline breaking changes.
//
// TODO: determine a good MessagePack serialization scheme to make this
// slightly more resilient to schema changes (like add / remove keys)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTweet<S, U> {
    pub author_id: UserId,
    pub reply: Option<ReplyInfo>,
    pub image_url: Option<U>,
    pub text: S,
    pub cluster_id: ClusterId,
}

pub type OwnedCachedTweet = CachedTweet<String, Url>;
type BorrowedCachedTweet<'a> = CachedTweet<&'a str, &'a Url>;

// TODO: Connection pooling

/// Save a bunch of tweets to redis, overwriting the existing data. Where
/// possible, send all of these as a single batch during a single thread
/// resolution, as it helps to pipeline requests & deduplicate serializing
/// of users.
///
/// This function will return an error if any error occurred, but in practice
/// these errors can be ignored in the business logic, because the cache is
/// ephemeral by design.
///
/// Note that, if several tweets have disagreeing user data (for instance,
/// our API calls raced against the user changing their profile picture),
/// it's arbitrary which one ends up in the database.
///
/// This function will assume that all the submitted tweets are part of the
/// same thread and will attempt to add additional indexing to the cache
/// to speed up future lookups.
// TODO: tracing for redis errors
pub async fn save_tweets(
    conn: &mut redis::aio::Connection,
    tweets: impl IntoIterator<Item = (TweetId, &Tweet)>,
    cluster_id: ClusterId,
) -> Result<(), Error> {
    /*
     * The basic plan here is that we're going to construct a single redis
     * command pipeline with all of our inserts. We're going to insert every
     * tweet, as well as every user associated with those tweets. We're going
     * to take care to deduplicate our users so that we only insert each
     * user ID once. We're going to set a 1-day expiry on User data to ensure
     * that changes to a user's username, handle, or profile pic are reflected
     * in a reasonably timely manner.
     *
     * This logic is primarily broken up into a series of iterator adapters.
     * While it could be very easily expressed with fewer such adapters,
     * as of this writing this design seems to have the advantage of clearly
     * separating and designating the different parts of the logic, which
     * with a conventional for loop would be more interleaved.
     */

    // This pipeline includes all the commands we'll be sending: a series of
    // SET (one per tweet and one per user), and an SADD for the tweets IDs for
    // the cluster
    let mut pipeline = redis::pipe();

    // This command is the insert into the cluster for these tweets. It'll be
    // built while looping over tweets and then added to the pipeline
    let mut cluster_add_cmd = redis::cmd("SADD");
    cluster_add_cmd.arg(schema::cluster_key(cluster_id).to_string());

    // These are reusable buffers that we use when we construct our command.
    let mut key_buffer = String::new();
    let mut serialize_buffer = Vec::new();

    // collect users while iterating tweets, so that at the end we'll have a
    // set of unique users.
    let mut user_table = HashMap::new();

    // TODO: confirm that these serialize calls are infallible.
    tweets.into_iter().for_each(|(tweet_id, tweet)| {
        // PART 1: Add a SET command for this tweet
        key_buffer.clear();
        write!(&mut key_buffer, "{}", schema::tweet_blob_key(tweet_id)).unwrap();

        serialize_buffer.clear();
        rmp_serde::encode::write(
            &mut serialize_buffer,
            &BorrowedCachedTweet {
                author_id: tweet.author.id,
                reply: tweet.reply,
                image_url: tweet.image_url.as_ref(),
                text: &tweet.text,
                cluster_id,
            },
        )
        .unwrap();

        // Add this command to the pipeline
        pipeline
            .set(&key_buffer, serialize_buffer.as_slice())
            .ignore();

        // PART 2: Add this tweet to the cluster
        serialize_buffer.clear();
        rmp_serde::encode::write(&mut serialize_buffer, &tweet_id).unwrap();
        cluster_add_cmd.arg(serialize_buffer.as_slice());

        // PART 3: Collect the user into the set
        user_table.insert(tweet.author.id, tweet.author.as_ref());
    });

    // Add the cluster command to the pipeline
    pipeline.add_command(cluster_add_cmd).ignore();

    // While that loop was looping, we created a set of users. Add a command to
    // SET each of them to the command as well. Ensure that the users are timed
    // out after 1 day.
    user_table.values().for_each(|user| {
        key_buffer.clear();
        write!(key_buffer, "{}", schema::user_blob_key(user.id)).unwrap();

        serialize_buffer.clear();
        rmp_serde::encode::write(
            &mut serialize_buffer,
            &BorrowedCachedUser {
                display_name: &user.display_name,
                handle: &user.handle,
                image_url: &user.image_url,
            },
        )
        .unwrap();

        const SECONDS_PER_DAY: u32 = 60 * 60 * 24;
        pipeline
            .set(&key_buffer, serialize_buffer.as_slice())
            .arg("EX")
            .arg(SECONDS_PER_DAY)
            .ignore();
    });

    // And that's it! Send all this to the cache and we're done.
    pipeline.query_async(conn).await?;
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct ClusterData {
    // We store options here to indicate the successful discovery of the
    // absence of data in Redis; this helps us avoid duplicate work
    pub tweets: HashMap<TweetId, Option<OwnedCachedTweet>>,
    pub users: HashMap<UserId, Option<OwnedCachedUser>>,
}

impl ClusterData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge a pair of `ClusterData`s together. We assume that the argument
    /// to this function is newer data, so its fields will always replace local
    /// ones.
    pub fn merge(self, newer: ClusterData) -> Self {
        fn merge_tables<K: Eq + Hash, V>(
            older: HashMap<K, V>,
            newer: HashMap<K, V>,
        ) -> HashMap<K, V> {
            match (older.len(), newer.len()) {
                (0, _) => newer,
                (_, 0) => older,

                // Prefer to merge into the table with more capacity
                _ => match older.capacity() > newer.capacity() {
                    // Newer keys unconditionally override older ones. We're inserting into
                    // `older`, so always override
                    true => newer.into_iter().fold(older, |mut map, (key, value)| {
                        map.insert(key, value);
                        map
                    }),
                    // Newer keys unconditionally override older ones. We're inserting into
                    // `newer`, so only use keys that aren't already present.
                    false => older.into_iter().fold(newer, |mut map, (key, value)| {
                        map.entry(key).or_insert(value);
                        map
                    }),
                },
            }
        }

        ClusterData {
            tweets: merge_tables(self.tweets, newer.tweets),
            users: merge_tables(self.users, newer.users),
        }
    }
}

/// Fetch data for a tweet, along with all the other tweets in the same cluster.
/// While it does use the cluster, this method does not attempt to separately
/// follow reply chains, since the top-level logic (which fetches from both
/// redis and twitter) will handle that.
pub async fn get_tweet_cluster(
    conn: &mut redis::aio::Connection,
    tweet_id: TweetId,
    data: &mut ClusterData,
) -> Result<(), Error> {
    // TODO: Improved error handling here. In general, errors in this function
    // should result in:
    // - Empty success result
    // - Logged error
    // - Key purged, if it's a data error
    //
    // Redis connection etc errors should result in some kind of retry, followed
    // by an error returned

    // Start by fetching this tweet
    let entry = match data.tweets.entry(tweet_id) {
        Entry::Occupied(_) => return Ok(()),
        Entry::Vacant(entry) => entry,
    };

    let tweet: OwnedCachedTweet = match redis::cmd("GET")
        .arg(schema::tweet_blob_key(tweet_id).to_string())
        .query_async(conn)
        .await?
    {
        redis::Value::Nil => {
            entry.insert(None);
            return Ok(());
        }
        redis::Value::Data(blob) => rmp_serde::from_slice(&blob)?,
        _ => {
            return Err(Error::Redis(RedisError::from((
                RedisErrorKind::TypeError,
                "response type wasn't a blob",
            ))))
        }
    };

    let cluster_id = tweet.cluster_id;
    let user_id = tweet.author_id;
    entry.insert(Some(tweet));

    // Next, get all the tweet IDs for the cluster
    let tweet_ids_in_cluster: Vec<TweetId> = match redis::cmd("SMEMBERS")
        .arg(schema::cluster_key(cluster_id).to_string())
        .query_async(conn)
        .await?
    {
        redis::Value::Nil => vec![],
        redis::Value::Bulk(items) => items
            .into_iter()
            .map(|item| match item {
                redis::Value::Data(blob) => rmp_serde::from_slice(&blob).map_err(Error::Decode),
                _ => Err(Error::Redis(RedisError::from((
                    RedisErrorKind::TypeError,
                    "response type wasn't a blob",
                )))),
            })
            // Exclude tweet IDs we already know things about
            .filter_ok(|id: &TweetId| !(data.tweets.contains_key(id) || *id == tweet_id))
            .try_collect()?,
        _ => {
            return Err(Error::Redis(RedisError::from((
                RedisErrorKind::TypeError,
                "response type wasn't an array",
            ))))
        }
    };

    // Next, get all the tweets in the cluster
    let mut request = redis::cmd("MGET");

    // We'll be pairing up tweet ids from `tweet_ids_in_cluster` with the
    // tweets in this list, so we need to ensure they're the same length,
    // so we create a list of optionals
    let tweets: Vec<Option<OwnedCachedTweet>> = match tweet_ids_in_cluster
        .iter()
        .map(|&tweet_id| schema::tweet_blob_key(tweet_id).to_string())
        .fold(&mut request, |request, key| request.arg(key))
        .query_async(conn)
        .await?
    {
        redis::Value::Bulk(items) => items
            .into_iter()
            .map(|item| match item {
                redis::Value::Nil => Ok(None),
                redis::Value::Data(blob) => rmp_serde::from_slice(&blob)
                    .map(Some)
                    .map_err(Error::Decode),
                _ => Err(Error::Redis(RedisError::from((
                    RedisErrorKind::TypeError,
                    "response type wasn't a blob",
                )))),
            })
            .try_collect()?,
        _ => {
            return Err(Error::Redis(RedisError::from((
                RedisErrorKind::TypeError,
                "response type wasn't an array",
            ))))
        }
    };

    if tweet_ids_in_cluster.len() != tweets.len() {
        todo!()
    }

    let user_ids: HashSet<UserId> = tweets
        .iter()
        .filter_map(|tweet| Some(tweet.as_ref()?.author_id))
        .chain([user_id])
        .filter(|user_id| !data.users.contains_key(user_id))
        .collect();

    data.tweets
        .extend(tweet_ids_in_cluster.into_iter().zip(tweets));

    // Finally, get all the authors of these tweets
    let user_ids: Vec<UserId> = Vec::from_iter(user_ids);

    let mut request = redis::cmd("MGET");
    let users: Vec<Option<OwnedCachedUser>> = match user_ids
        .iter()
        .map(|&user_id| schema::user_blob_key(user_id).to_string())
        .fold(&mut request, |request, key| request.arg(key))
        .query_async(conn)
        .await?
    {
        redis::Value::Bulk(items) => items
            .into_iter()
            .map(|item| match item {
                redis::Value::Nil => Ok(None),
                redis::Value::Data(blob) => rmp_serde::from_slice(&blob)
                    .map(Some)
                    .map_err(Error::Decode),
                _ => Err(Error::Redis(RedisError::from((
                    RedisErrorKind::TypeError,
                    "response type wasn't a blob",
                )))),
            })
            .try_collect()?,
        _ => {
            return Err(Error::Redis(RedisError::from((
                RedisErrorKind::TypeError,
                "response type wasn't an array",
            ))))
        }
    };

    if user_ids.len() != users.len() {
        todo!()
    }

    data.users.extend(user_ids.into_iter().zip(users));

    Ok(())
}

pub async fn get_user(
    conn: &mut redis::aio::Connection,
    user_id: UserId,
) -> Result<Option<OwnedCachedUser>, Error> {
    match redis::cmd("GET")
        .arg(schema::user_blob_key(user_id).to_string())
        .query_async(conn)
        .await?
    {
        redis::Value::Nil => Ok(None),
        redis::Value::Data(blob) => rmp_serde::from_slice(&blob)
            .map(Some)
            .map_err(Error::Decode),
        _ => Err(Error::Redis(RedisError::from((
            RedisErrorKind::TypeError,
            "response type wasn't a blob",
        )))),
    }
}
