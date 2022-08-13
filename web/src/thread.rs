use std::{cmp, collections::HashMap, fmt::Debug, mem, rc::Rc};

use futures::TryFutureExt as _;
use itertools::Itertools as _;
use thiserror::Error;
use tracing::Instrument as _;

use crate::{
    redis::{
        get_tweet_cluster as get_redis_tweet_cluster, get_user as get_user_from_redis, ClusterData,
        Error as RedisError, OwnedCachedTweet, OwnedCachedUser,
    },
    table::{DedupeTable, Entry as DedupeEntry},
    twitter::{
        self,
        api::{ReplyInfo, User},
        auth::Token,
        Tweet, TweetId, UserId,
    },
};

/// Helper struct for normalizing / deduplicating User objects. The idea is
/// that, since we're often receiving large sets of tweets from a single user,
/// we can save a lot of space by having all the Tweets have an Arc to a
/// single User instance.
pub(super) type UserTable = DedupeTable<UserId, User>;

#[derive(Debug, Clone)]
pub enum ThreadAuthor {
    Author(Rc<User>),
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
    // We found the tweet
    FoundTweet(Tweet),

    // We weren't able to find a tweet with this ID
    MissingTweet(TweetId),

    // We were only ever to fetch some of the tweet.
    PartiallyMissingTweet {
        tweet_id: TweetId,
        reply: Option<ReplyInfo>,
    },
}

impl TweetLookupResult {
    #[inline]
    #[must_use]
    fn tweet(&self) -> Option<&Tweet> {
        match *self {
            Self::FoundTweet(ref tweet) => Some(tweet),
            Self::MissingTweet(..) | Self::PartiallyMissingTweet { .. } => None,
        }
    }

    #[inline]
    #[must_use]
    fn tweet_id(&self) -> TweetId {
        match *self {
            TweetLookupResult::FoundTweet(ref tweet) => tweet.id,
            TweetLookupResult::MissingTweet(id) => id,
            TweetLookupResult::PartiallyMissingTweet { tweet_id, .. } => tweet_id,
        }
    }

    #[inline]
    #[must_use]
    fn previous_tweet_id(&self) -> Option<TweetId> {
        match *self {
            TweetLookupResult::MissingTweet(..) => None,

            TweetLookupResult::FoundTweet(Tweet { ref reply, .. })
            | TweetLookupResult::PartiallyMissingTweet { ref reply, .. } => {
                reply.as_ref().map(|reply| reply.id)
            }
        }
    }
}

// TODO: attach much more context to these errors (or use anyhow)
#[derive(Debug, Error)]
pub enum BuildThreadError {
    #[error("error fetching data from the Twitter API")]
    ApiError(#[from] reqwest::Error),

    #[error("error fetching cached data from Redis")]
    RedisError(#[from] RedisError),
}

/// Main logic for constructing a thread.
#[tracing::instrument(skip(client, redis, token))]
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

    // In order to save space, we deduplicate user objects as Rc<User> and
    // store them here in the user table for easy resuse.
    let mut user_table = UserTable::new();

    // As a key optimization, both for performance and for rate limiting, we
    // fetch tweet author timelines, under the assumption that tweets made
    // near the same time as this one are probably part of the thread. The
    // results of those cache lookups are stored here.
    let mut tweet_box: HashMap<TweetId, Tweet> = HashMap::new();

    let mut cluster_data = ClusterData::new();

    // This is the tweet we're attempting to fetch. In the course of fetching
    // it, we'll also be fetching "nearby" tweets (via Twitter's timeline API
    // and Redis clusters) which we'll use when attempting to fetch future
    // tweets
    let mut current_tweet_id = Some(tail);

    while let Some(tweet_id) = current_tweet_id.take() {
        // TODO: protect against cycles. For now we rely on twitter API to not
        // give us cycles.
        let entry = build_thread_entry(
            tweet_id,
            client,
            token,
            redis,
            &mut user_table,
            &mut tweet_box,
            &mut cluster_data,
        )
        .unwrap_or_else(|err| {
            tracing::error!(?err, "error creating thread entry");
            TweetLookupResult::MissingTweet(tweet_id)
        })
        .instrument(tracing::info_span!("thread_entry", %tweet_id))
        .await;

        current_tweet_id = entry.previous_tweet_id();
        thread_items.push(entry);
    }

    todo!()
}

// TODO: Usually (for all tweets after the very first), we'll know ahead of time
// the author of this tweet. Include that author here so that we can eagerly
// fetch directly from their timeline instead of fetching just the one tweet.
// TODO: If a timeline fetch fails, it'll probably continue to fail for the
// remainder of the thread. Track which users aren't worth doing timeline fetches
// for.
// TODO: If a redis cluster fails to return tweets (perhaps because they were
// cleared from redis for LRU reasons), we should keep track of the IDs in the
// cluster and fetch them all eagerly from the twitter API in here.

#[tracing::instrument(skip(client, redis, token))]
async fn build_thread_entry(
    tweet_id: TweetId,
    client: &reqwest::Client,
    token: &impl Token,
    redis: &mut redis::aio::Connection,
    user_table: &mut UserTable,
    tweet_box: &mut HashMap<TweetId, Tweet>,
    cluster_data: &mut ClusterData,
) -> Result<TweetLookupResult, BuildThreadError> {
    // Step 1: try to fetch it from tweet_box, our local source of high
    // quality organic preserved tweets
    if let Some(tweet) = tweet_box.remove(&tweet_id) {
        return Ok(TweetLookupResult::FoundTweet(tweet));
    }

    // Step 2: try to fetch it from cluster_data, our local source of low
    // quality tweet dregs from Redis. Need to freshen them up into a Tweet.
    if let Some(tweet) = cluster_data.tweets.remove(&tweet_id) {
        // We found something from redis; need to reproduce a Tweet. In
        // particular we need to recreate the tweet's author.
        return reconstruct_tweet_from_cluster(
            tweet_id,
            tweet,
            user_table,
            client,
            token,
            redis,
            &mut cluster_data.users,
        )
        .await;
    }

    // Step 3: We didn't have a local copy, so we're going to try to fetch it
    // from redis.
    if let Ok(new_cluster_data) = get_redis_tweet_cluster(redis, tweet_id).await {
        // We got something, but no guarantee the specific tweet was there. Retry
        // the same reconstruction from above.
        // TODO: Find a way to dedupe steps 3 and 4
        *cluster_data = mem::take(cluster_data).merge(new_cluster_data);
        if let Some(tweet) = cluster_data.tweets.remove(&tweet_id) {
            return reconstruct_tweet_from_cluster(
                tweet_id,
                tweet,
                user_table,
                client,
                token,
                redis,
                &mut cluster_data.users,
            )
            .await;
        }
    }

    // Step 4: All else has failed; we have no choice but to reach out to the
    // twitter API directly.
    let tweet = twitter::api::get_tweet(client, token, tweet_id, user_table).await?;

    // Okay, we finally have a tweet. Before we return it, we're going to
    // perform an optimistic fetch of this user's recent timeline tweets, to
    // avoid having to fetch future tweets 1-by-1.
    //
    // We're going to fetch both this user's and the reply tweet's author's.
    // We'd like to do this concurrently, but fetching user tweets requires
    // an &mut UserTable, so we'll be sequential for now.
    for user_id in [Some(tweet.author.id), tweet.reply.map(|reply| reply.author)]
        .iter()
        .flatten()
        .copied()
        .dedup()
    {
        tweet_box.extend(
            twitter::api::get_user_tweets(client, token, user_id, tweet.id, user_table)
                .await?
                .into_iter()
                .map(|tweet| (tweet.id, tweet)),
        )
    }

    // Note that we specifically don't do this timeline fetch for tweets that
    // we got from redis; we assume that redis will tend to have all the tweets
    // of a particular thread, so we don't start hitting twitter until we know
    // we have to.
    //
    // Essentially, there are two cases we expect to be common: the entire
    // thread is in Redis, and the entire thread is absent from redis. The
    // design of this function is based around those cases, rather than the
    // uncommon case where only some of a thread is present in redis.

    // We don't know how much of the optimistic fetching above will end up in
    // the final thread, so we don't do any publishing to Redis. We only want
    // to push to redis the *actual* tweets involved in this Thread.
    Ok(TweetLookupResult::FoundTweet(tweet))
}

async fn reconstruct_tweet_from_cluster(
    tweet_id: TweetId,
    tweet: OwnedCachedTweet,
    user_table: &mut UserTable,
    client: &reqwest::Client,
    token: &impl Token,
    redis: &mut redis::aio::Connection,
    user_cluster_data: &mut HashMap<UserId, OwnedCachedUser>,
) -> Result<TweetLookupResult, BuildThreadError> {
    let user = match user_table.entry(tweet.author_id) {
        DedupeEntry::Occupied(user) => user,
        DedupeEntry::Vacant(entry) => {
            let user =
                get_cached_tweet_author(*entry.key(), client, token, redis, user_cluster_data)
                    .await?;

            entry.insert(user)
        }
    };

    Ok(TweetLookupResult::FoundTweet(Tweet {
        id: tweet_id,
        text: tweet.text,
        author: user.clone(),
        reply: tweet.reply,
        image_url: tweet.image_url,
    }))
}

// Get the data for a User, associated with a user_id. Check the local redis
// cluter data, redis itself, and the twitter API. We specifically don't check
// user_table, since this function will be used to populate the user_table.
async fn get_cached_tweet_author(
    user_id: UserId,
    client: &reqwest::Client,
    token: &impl Token,
    redis: &mut redis::aio::Connection,
    user_cluster_data: &mut HashMap<UserId, OwnedCachedUser>,
) -> Result<User, BuildThreadError> {
    // TODO: Return an Option, maybe? If we're calling this function we're
    // probably pretty confident that the user exists (or, at least, used
    // to exist). We therefore treat failures here as true errors rather than
    // successful nulls. In the future we'll at least have a more useful
    // structured error for reporting different kinds of failure.

    // First, check the cluster data directly. We're okay with popping data
    // out of cluter_data, because we know that it's going to end up in the
    // user_table later, which will serve for subsequent deduplications.
    if let Some(user) = user_cluster_data.remove(&user_id) {
        return Ok(user_from_cached(user_id, user));
    }

    // We don't have a local copy of this user. First try fetching from
    // redis.
    if let Some(user) = get_user_from_redis(redis, user_id).await? {
        return Ok(user_from_cached(user_id, user));
    }

    // Okay, we don't have *any* copy of this user; time to go ask twitter
    // directly. We don't need to do any cache writes at this point; the thread
    // builder will take care of that after the whole thread has been assembled
    twitter::api::get_user(client, token, user_id)
        .await
        .map_err(BuildThreadError::ApiError)
}

#[inline]
#[must_use]
fn user_from_cached(user_id: UserId, cached: OwnedCachedUser) -> User {
    User {
        id: user_id,
        display_name: cached.display_name,
        handle: cached.handle,
        image_url: cached.image_url,
    }
}

fn thread_author<'a>(authors: impl IntoIterator<Item = &'a Rc<User>>) -> ThreadAuthor {
    struct Entry<'a> {
        pub count: usize,
        pub author: &'a Rc<User>,
    }

    let mut counter: HashMap<UserId, Entry> = HashMap::new();

    authors.into_iter().for_each(|author| {
        counter
            .entry(author.id)
            .and_modify(|entry| entry.count += 1)
            .or_insert(Entry { count: 1, author });
    });

    let mut counted: Vec<&Entry> = counter.values().collect();
    counted.sort_unstable_by_key(|entry| cmp::Reverse(entry.count));

    match counted.get(0) {
        // If there's no one, it's a conversation
        None => ThreadAuthor::Conversation,

        // `most` is the user with the most tweets in this thread
        Some(most) => match counted.get(1) {
            // If they're the only user in the thread, they're the author
            None => ThreadAuthor::Author(most.author.clone()),

            // If they don't have a unique plurality of tweets, it's a conversation
            Some(next) if next.count >= most.count => ThreadAuthor::Conversation,

            // If the user has more tweets than there are people here, they're the author
            Some(..) if most.count * 2 >= counted.len() => {
                ThreadAuthor::Author(most.author.clone())
            }

            // Otherwise it's a conversation
            _ => ThreadAuthor::Conversation,
        },
    }
}

/*
   // Apply meta stuff.
   // - The description is the content of the first tweet
   // - The image is the image in the first tweet, or if there is none, the
   //   author's image, or if there's no author, the image of the first person
   //   in the conversation
   let meta = thread_items
       .iter()
       .rev()
       .find_map(|item| match *item {
           TweetLookupResult::FoundTweet(ref tweet) => Some(tweet),
           _ => None,
       })
       .map(|tweet| {
           let description = tweet.text.clone();
           let image_url = match tweet.image_url {
               Some(ref url) => url,
               None => match author {
                   ThreadAuthor::Author(ref thread_author) => &thread_author.image_url,
                   ThreadAuthor::Conversation => &tweet.author.image_url,
               },
           }
           .clone();

           Meta {
               description,
               image_url,
           }
       });
*/
