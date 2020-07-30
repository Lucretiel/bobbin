//! This module manages caching and retrieving tweet IDs in a redis store

/*
Schema overview

bobbin:user:{USER_ID}:blob {User Blob}

# ^^^^^ User keys expire because users can change their profile pic etc

bobbin:tweet:{TWEET_ID}:blob {Tweet Blob}
bobbin:tweet:{TWEET_ID}:cluster cluster_id

# ^^^^^ None of these keys need to expire, because they're all immutable
# over the lifetime of a tweet. The Thread set may change, but only in an
# additive way; it can never really be "wrong".

# A cluster is a set of tweets that were all written to the cache together.
# It is assumed that the tweets in a given cluster are part of the same
# thread, which in turn can be used when fetching a thread to speculatively
# fetch related tweets.
bobbin:cluster:{CLUSTER_ID}:tweets: Set of {TWEET_ID}
# TODO: in theory, over time clusters may become dead. Set up a background task
# That periodically goes through and cleans them up a bit. In practice, because
# we're very rarely dealing with intersecting threads, this shouldn't be a
# problem, and automatic LRU eviction will take care of dead clusters.
*/

// Additional design notes:
//
// Data is packed with MessagePack (see the struct types later in this
// module)
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
// Each tweet only has one thread ID, because threads are only looked up
// from back to front.
//
// TODO: use some analytics to verify this assumption.

use std::{
    collections::{HashMap, HashSet},
    error,
    fmt::{self, Display, Formatter, Write as FmtWrite},
    hash::{self, Hash},
    io::Write as IoWrite,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use redis::{
    self, AsyncCommands, Cmd, ConnectionLike, FromRedisValue, RedisError, RedisResult, RedisWrite,
    ToRedisArgs,
};
use rmp_serde::{self, encode::Error as MpError};
use serde::{Deserialize, Serialize};

use super::{
    api::{ReplyInfo, Tweet, TweetId, User, UserId},
    table::DedupeTable,
};
#[derive(Debug)]
pub enum Error {
    Redis(RedisError),
    Encode(MpError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Redis(err) => write!(f, "redis error: {}", err),
            Error::Encode(err) => write!(f, "redis serialization error: {}", err),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Redis(err) => Some(err),
            Error::Encode(err) => Some(err),
        }
    }
}

impl From<RedisError> for Error {
    fn from(err: RedisError) -> Self {
        Error::Redis(err)
    }
}

impl From<MpError> for Error {
    fn from(err: MpError) -> Self {
        Error::Encode(err)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ClusterId(TweetId);

impl Display for ClusterId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUser {
    // TODO: Convert all of this to &str. This should work because the contents
    // of these fields always references either a User object or a buffer
    // returned from redis. The trouble is in figuring out what to return from
    // fetch_tweets.
    pub display_name: String,
    pub handle: String,
    pub image_url: String,
}

// TODO: This schema meta-design makes no accounting for potential schema
// changes. For now we'll plan to do the ugly thing and erase the redis cache
// if we need to do any inline breaking changes.
//
// TODO: determine a good MessagePack serialization scheme to make this
// slightly more resilient to schema changes (like add / remove keys)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTweet {
    pub author_id: UserId,
    // TODO: Consider caching the reply-to userid. A decision will be made
    // based on the final implementation of the thread-fetcher.
    pub reply_to: Option<TweetId>,

    // TODO: find a way to replace this with &str. In theory it's possible,
    // because the source of this string is either a read from redis or a
    // Tweet object. Depends on the return interface of fetch_tweets
    pub image_url: Option<String>,

    cluster_id: ClusterId,
}

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
    client: &redis::Client,
    tweets: impl IntoIterator<Item = &Tweet>,
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
     * I'm not sure yet what the best solution is for updating the thread ids
     * for easy batched lookup of threads. The trouble is that tweets form a
     * directed tree, so we need to find a scheme that propagates thread ids
     * as much as possible without (?) overprapogating.
     *
     * On the other hand, overpropagating might be okay, if we assume that
     * intersecting threads don't have massive divergence.
     *
     * The other thing we could do is assign one of these tweets IDs as a
     * "cluster ID" for this particular batch. The idea is that a thread
     * containing a tweet in a particular cluster probably contains all or
     * most of the tweets in the same cluster. Then thread reads will be 1
     * or more clusters, which worst case is the same as the (much more
     * complicated) thread id reconciliation logic.
     *
     * This logic is primarily broken up into a series of iterator adapters.
     * While it could be very easily expressed with fewer such adapters,
     * as of this writing this design seems to have the advantage of clearly
     * separating and designating the different parts of the logic, which
     * with a conventional for loop would be more interleaved.
     */

    let mut conn = client.get_async_connection().await?;
    let tweets = tweets.into_iter();

    // collect users while iterating tweets, so that at the end we'll have a
    // set of unique users.
    let mut user_table = HashSet::new();
    let tweets = tweets.inspect(|tweet| {
        user_table.insert(UserHash(&tweet.author));
    });

    // Set up our cluster: the tweet ID of the first tweet will be arbitrarily
    // selected as our cluster ID
    let cluster_tweets = tweets.scan(None, |cluster_id, tweet| {
        if *cluster_id == None {
            *cluster_id = Some(ClusterId(tweet.id));
        }

        Some((*cluster_id, tweet))
    });

    // Convert all the tweets to CachedTweet instances to be serialized
    let cache_tweets = cluster_tweets.map(|(cluster_id, tweet)| {
        (
            tweet.id,
            CachedTweet {
                author_id: tweet.author.id,
                reply_to: tweet.reply.as_ref().map(|r| r.id),
                image_url: tweet.image_url.clone(),
                // The cluster_tweets scan ensures that this is always Some
                cluster_id: cluster_id.unwrap(),
            },
        )
    });

    // These are reusable buffers that we use when we construct our command.
    let mut key_buffer = String::new();
    let mut serialize_buffer = Vec::new();

    let mut pipeline = redis::pipe();

    // Start constructing the command pipeline: add a command to SET every
    // tweet instance. Potential errors here are from serializing, but I'm
    // pretty sure that's infallible in this case.
    //
    // TODO: this is where the cluster or thread ID needs to be determined
    // (assuming we want to get through this in a single pass).
    cache_tweets.for_each(|(tweet_id, tweet)| {
        // PART 1: WRITE THE TWEET BLOB
        write!(key_buffer, "bobbin:tweet:{}:blob", tweet_id).unwrap();

        // TODO: confirm that this serialize call is infallible.
        rmp_serde::encode::write(&mut serialize_buffer, &tweet).unwrap();

        // Add this command to the pipeline
        pipeline
            .set(&key_buffer, serialize_buffer.as_slice())
            .ignore();

        key_buffer.clear();
        serialize_buffer.clear();

        // PART 2: UPDATE THE CLUSTER
        // TODO: It's plausible that it's more performant to have a single
        // SADD command, since it can take an arbitrary number of arguments.
        write!(key_buffer, "bobbin:cluster:{}", tweet.cluster_id).unwrap();
        write!(serialize_buffer, "{}", tweet_id).unwrap();

        pipeline
            .sadd(&key_buffer, serialize_buffer.as_slice())
            .ignore();

        key_buffer.clear();
        serialize_buffer.clear();
    });

    // While that loop was looping, we created a set of users. Add them to
    // the command as well.
    let cache_users = user_table.iter().map(|user| {
        (
            user.id,
            CachedUser {
                display_name: user.display_name.clone(),
                handle: user.handle.clone(),
                image_url: user.image_url.clone(),
            },
        )
    });

    cache_users.for_each(|(user_id, user)| {
        write!(key_buffer, "bobbin:user:{}", user_id).unwrap();
        rmp_serde::encode::write(&mut serialize_buffer, &user).unwrap();

        const SECONDS_PER_DAY: u32 = 60 * 60 * 24;
        pipeline
            .set(&key_buffer, serialize_buffer.as_slice())
            .arg("EX")
            .arg(SECONDS_PER_DAY)
            .ignore();

        key_buffer.clear();
        serialize_buffer.clear();
    });

    // And that's it! Send all this to the cache and we're done.
    pipeline.query_async(&mut conn).await?;
    Ok(())
}

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
    tail: TweetId,
) -> Result<HashMap<TweetId, CachedTweet>, Error> {
    let conn = client.get_async_connection().await?;

    // First, get the tail tweet. If

    todo!()
}
