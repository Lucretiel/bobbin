use crate::twitter::{self, auth::Token, Tweet, TweetId};

use std::{cmp, collections::HashMap, sync::Arc};

use twitter::{User, UserId};

#[derive(Debug, Clone)]
pub enum ThreadItem {
    Missing(TweetId),
    Tweet(Tweet),
}

impl ThreadItem {
    pub fn tweet_id(&self) -> TweetId {
        match *self {
            ThreadItem::Missing(id) => id,
            ThreadItem::Tweet(ref tweet) => tweet.id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ThreadAuthor {
    Author(Arc<User>),
    Conversation,
}

#[derive(Debug, Clone)]
pub struct Thread {
    items: Vec<ThreadItem>,
    author: ThreadAuthor,
}

impl Thread {
    pub fn items(&self) -> &[ThreadItem] {
        &self.items
    }

    pub fn author(&self) -> &ThreadAuthor {
        &self.author
    }
}

#[derive(Debug, Clone)]
enum TweetLookupResult {
    FoundTweet(Tweet),
    MissingTweet(TweetId),
}

impl TweetLookupResult {
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
) -> reqwest::Result<Thread> {
    // Threads are constructed from back to front; thread is populated,
    // then reversed
    let mut thread: Vec<ThreadItem> = Vec::new();

    // As a key optimization, both for performance and for rate limiting, we
    // fetch tweet author timelines, under the assumption that tweets made
    // near the same time as this one are probably part of the thread. The
    // results of those cache lookups are stored here.
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
                    // TODO: If this comes back empty, it's probably because
                    // the get_user_tweets API can't search arbitrarily far
                    // back in time. This means that
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

        let prev_id = tweet.previous_tweet_id();

        thread.push(match tweet {
            TweetLookupResult::FoundTweet(tweet) => ThreadItem::Tweet(tweet),
            TweetLookupResult::MissingTweet(id) => ThreadItem::Missing(id),
        });

        if head == Some(current_tweet_id) {
            break;
        }

        match prev_id {
            Some(id) => current_tweet_id = id,
            None => break,
        }
    }

    thread.reverse();

    let author = thread_author(thread.iter().filter_map(|item| match item {
        ThreadItem::Tweet(tweet) => Some(&tweet.author),
        _ => None,
    }));

    Ok(Thread {
        items: thread,
        author,
    })
}

fn thread_author<'a>(authors: impl IntoIterator<Item = &'a Arc<User>>) -> ThreadAuthor {
    struct Entry<'a> {
        pub count: usize,
        pub author: &'a Arc<User>,
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
