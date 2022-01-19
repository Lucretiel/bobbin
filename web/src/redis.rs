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
//
// One thing we'd like to add in the future is an additional way to associate
// groups of tweets so that we don't need to make numerous round trips to redis
// when loading a thread

use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display, Formatter, Write as FmtWrite},
    hash::{self, Hash},
    iter,
    ops::Deref,
};

use itertools::Itertools;
use redis::{self, aio::ConnectionLike, ErrorKind as RedisErrorKind, RedisError};
use rmp_serde::{self, decode::Error as MpDeError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    thread::Thread,
    twitter::api::{ReplyInfo, Tweet, TweetId, User, UserId},
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
pub struct CachedUser<S: AsRef<str>> {
    pub display_name: S,
    pub handle: S,
    pub image_url: S,
}

pub type OwnedCachedUser = CachedUser<String>;

// TODO: This schema meta-design makes no accounting for potential schema
// changes. For now we'll plan to do the ugly thing and erase the redis cache
// if we need to do any inline breaking changes.
//
// TODO: determine a good MessagePack serialization scheme to make this
// slightly more resilient to schema changes (like add / remove keys)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTweet<S: AsRef<str>> {
    pub author_id: UserId,
    pub reply: Option<ReplyInfo>,
    pub image_url: Option<S>,
    pub text: S,
    pub cluster_id: ClusterId,
}

pub type OwnedCachedTweet = CachedTweet<String>;

/// Helper type to hash & compare users by ID. Used to create a HashSet of
/// unique users.
#[derive(Debug, Clone)]
struct UserHash<'a>(&'a User);

impl Hash for UserHash<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state)
    }
}

impl PartialEq for UserHash<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for UserHash<'_> {}

impl Deref for UserHash<'_> {
    type Target = User;

    fn deref(&self) -> &User {
        self.0
    }
}

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
    tweets: impl IntoIterator<Item = &Tweet>,
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
    let mut user_table = HashSet::new();

    // TODO: confirm that these serialize calls are infallible.
    tweets.into_iter().for_each(|tweet| {
        // PART 1: Add a SET command for this tweet
        key_buffer.clear();
        write!(&mut key_buffer, "{}", schema::tweet_blob_key(tweet.id)).unwrap();

        serialize_buffer.clear();
        rmp_serde::encode::write(
            &mut serialize_buffer,
            &CachedTweet {
                author_id: tweet.author.id,
                reply: tweet.reply.clone(),
                image_url: tweet.image_url.as_deref(),
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
        rmp_serde::encode::write(&mut serialize_buffer, &tweet.id).unwrap();
        cluster_add_cmd.arg(serialize_buffer.as_slice());

        // PART 3: Collect the user into the set
        user_table.insert(UserHash(&tweet.author));
    });

    // Add the cluster command to the pipeline
    pipeline.add_command(cluster_add_cmd).ignore();

    // While that loop was looping, we created a set of users. Add a command to
    // SET each of them to the command as well. Ensure that the users are timed
    // out after 1 day.
    user_table.iter().for_each(|user| {
        key_buffer.clear();
        write!(key_buffer, "{}", schema::user_blob_key(user.id)).unwrap();

        serialize_buffer.clear();
        rmp_serde::encode::write(
            &mut serialize_buffer,
            &CachedUser {
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
    pub tweets: HashMap<TweetId, OwnedCachedTweet>,
    pub users: HashMap<UserId, OwnedCachedUser>,
}

/// Fetch data for a tweet, along with all the other tweets in the same cluster.
/// While it does use the cluster, this method does not attempt to separately
/// follow reply chains, since the top-level logic (which fetches from both
/// redis and twitter) will handle that
pub async fn get_tweet_cluster(
    conn: &mut redis::aio::Connection,
    tweet_id: TweetId,
) -> Result<ClusterData, Error> {
    // TODO: Improved error handling here. In general, errors in this function
    // should result in:
    // - Empty success result
    // - Logged error
    // - Key purged, if it's a data error
    //
    // Redis connection etc errors should result in some kind of retry, followed
    // by an error returned
    let mut data = ClusterData::default();

    // Start by fetching this tweet
    let response = conn
        .req_packed_command(&redis::Cmd::get(
            schema::tweet_blob_key(tweet_id).to_string(),
        ))
        .await?;

    let tweet: OwnedCachedTweet = match response {
        redis::Value::Nil => return Ok(data),
        redis::Value::Data(blob) => rmp_serde::from_slice(&blob)?,
        _ => {
            return Err(Error::Redis(RedisError::from((
                RedisErrorKind::TypeError,
                "response type wasn't a blob",
            ))))
        }
    };

    // Next, get all the tweet IDs for the cluster
    let response = conn
        .req_packed_command(&redis::Cmd::smembers(
            schema::cluster_key(tweet.cluster_id).to_string(),
        ))
        .await?;

    let cluster_ids: Vec<TweetId> = match response {
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
            .filter_ok(|id: &TweetId| *id != tweet_id)
            .try_collect()?,
        _ => {
            return Err(Error::Redis(RedisError::from((
                RedisErrorKind::TypeError,
                "response type wasn't a blob",
            ))))
        }
    };

    // Next, get all the tweets in the cluster
    let keys: Vec<String> = cluster_ids
        .iter()
        .map(|&id| schema::tweet_blob_key(id).to_string())
        .collect();

    let response = conn.req_packed_command(&redis::Cmd::get(keys)).await?;

    // Need to build a vec of options because we need to pair with tweet IDs
    let tweets: Vec<Option<OwnedCachedTweet>> = match response {
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

    if cluster_ids.len() != tweets.len() {
        todo!()
    }

    let all_tweets: HashMap<TweetId, OwnedCachedTweet> = cluster_ids
        .iter()
        .copied()
        .zip(tweets)
        .filter_map(|(tweet_id, maybe_tweet)| maybe_tweet.map(|tweet| (tweet_id, tweet)))
        .chain(iter::once((tweet_id, tweet)))
        .collect();

    // Finally, get all the authors of these tweets
    let user_ids: HashSet<UserId> = all_tweets.values().map(|tweet| tweet.author_id).collect();

    todo!()
}
