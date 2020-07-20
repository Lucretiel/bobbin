use std::collections::HashMap;

use crate::twitter::{self, auth::Token, Tweet, TweetId};

#[derive(Debug, Clone)]
pub enum ThreadItem {
    TweetId(TweetId),
    Missing(TweetId),
}

impl ThreadItem {
    fn tweet_id(&self) -> TweetId {
        match *self {
            ThreadItem::TweetId(id) => id,
            ThreadItem::Missing(id) => id,
        }
    }
}

#[derive(Debug, Clone)]
enum TweetLookupResult {
    FoundTweet(Tweet),
    //CacheHit(Tweet),  // TODO: Add me with Redis
    MissingTweet(TweetId),
}

impl TweetLookupResult {
    fn thread_item(&self) -> ThreadItem {
        match *self {
            TweetLookupResult::MissingTweet(tweet_id) => ThreadItem::Missing(tweet_id),
            TweetLookupResult::FoundTweet(ref tweet) => ThreadItem::TweetId(tweet.id),
        }
    }

    fn previous_tweet_id(&self) -> Option<TweetId> {
        match *self {
            TweetLookupResult::MissingTweet(..) => None,
            TweetLookupResult::FoundTweet(ref tweet) => tweet.reply.as_ref().map(|reply| reply.id),
        }
    }
}

pub async fn get_thread(
    client: &reqwest::Client,
    token: &impl Token,
    tail: TweetId,
    head: Option<TweetId>,
) -> reqwest::Result<Vec<ThreadItem>> {
    // Threads are constructed from back to front; thread is populated,
    // then reversed
    let mut thread: Vec<ThreadItem> = Vec::new();

    // As a key optimization, both for performance and for rate limiting, we
    // fetch tweet author timelines, under the assumption that tweets made
    // near the same time as this one are probably part of the thread. The
    // results of those cache lookups are stored here.
    // Because we don't know which of those tweets are actually part of the
    // thread, and we don't want to cache too aggresively, we don't move these
    // to a permanent cache (redis) until we discover that they're part of
    let mut tweet_box: HashMap<TweetId, Tweet> = HashMap::new();

    let mut current_tweet_id = tail;

    loop {
        // Try to fetch the tweet associated with next_tweet. Right now, we
        // search tweet_box (populated be speculative timeline lookups),
        // then turn to the API; in the future we will also check a Redis
        // cache.
        let tweet = match tweet_box.get(&current_tweet_id) {
            Some(cached_tweet) => TweetLookupResult::FoundTweet(cached_tweet.clone()),

            // TODO: Redis lookup here, before the API call
            None => match twitter::get_tweet(client, token, current_tweet_id).await? {
                Some(tweet) => {
                    // Pre-fetch the reply author's recent tweets, as described
                    // above
                    if let Some(ref reply) = tweet.reply {
                        let user_tweets =
                            twitter::get_user_tweets(client, token, reply.author, tweet.id)
                                .await?
                                .into_iter()
                                .map(|tweet| (tweet.id, tweet));
                        tweet_box.extend(user_tweets);
                    }

                    TweetLookupResult::FoundTweet(tweet)
                }
                None => TweetLookupResult::MissingTweet(current_tweet_id),
            },
        };

        thread.push(tweet.thread_item());

        if head == Some(current_tweet_id) {
            break;
        }

        match tweet.previous_tweet_id() {
            Some(prev_id) => current_tweet_id = prev_id,
            None => break,
        }
    }

    thread.reverse();
    Ok(thread)
}
