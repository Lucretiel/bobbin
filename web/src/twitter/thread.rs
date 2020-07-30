// TODO: Consider moving thread.rs out of twitter, to further clarify that
// it's a direct dependency and sits on top of it.

use std::{cmp, collections::HashMap, sync::Arc};

use super::{
    api::{self, Tweet, TweetId, User, UserId},
    auth::Token,
};

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

pub async fn get_thread(
    client: &reqwest::Client,
    token: &impl Token,
    tail: TweetId,
    head: Option<TweetId>,
) -> reqwest::Result<Thread> {
    // Threads are constructed from back to front; thread is populated,
    // then reversed
    let mut thread_items: Vec<TweetLookupResult> = Vec::new();

    // As a key optimization, both for performance and for rate limiting, we
    // fetch tweet author timelines, under the assumption that tweets made
    // near the same time as this one are probably part of the thread. The
    // results of those cache lookups are stored here.
    let mut tweet_box: HashMap<TweetId, Tweet> = HashMap::new();

    // See the twitter module docs for why we use a UserTable.
    let mut user_table = api::UserTable::new();

    let mut current_tweet_id = tail;

    loop {
        // Try to fetch the tweet associated with next_tweet. Right now, we
        // search tweet_box (populated be speculative timeline lookups),
        // then turn to the API; in the future we will also check a Redis
        // cache.
        let tweet = match tweet_box.get(&current_tweet_id) {
            Some(cached_tweet) => TweetLookupResult::FoundTweet(cached_tweet.clone()),

            // TODO: Redis lookup here, before the API call
            None => match api::get_tweet(client, token, current_tweet_id, &mut user_table).await? {
                Some(tweet) => {
                    // Pre-fetch the reply author's recent tweets, as described
                    // above
                    // TODO: If this comes back empty, it's probably because
                    // the get_user_tweets API can't search arbitrarily far
                    // back in time. This means that
                    if let Some(ref reply) = tweet.reply {
                        let user_tweets = api::get_user_tweets(
                            client,
                            token,
                            reply.author,
                            tweet.id,
                            &mut user_table,
                        )
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

        thread_items.push(tweet);

        if head == Some(current_tweet_id) {
            break;
        }

        match prev_id {
            Some(id) => current_tweet_id = id,
            None => break,
        }
    }

    // Reverse the tweet IDs to get our actual thread
    let thread: Vec<TweetId> = thread_items
        .iter()
        .map(|item| item.tweet_id())
        .rev()
        .collect();

    // Decide who the author is
    let author = thread_author(thread_items.iter().filter_map(|item| match *item {
        TweetLookupResult::FoundTweet(ref tweet) => Some(&tweet.author),
        _ => None,
    }));

    // Apply meta stuff.
    // - The description is the content of the first tweet
    // - The image is the image in the first tweet, or if there is none, the
    //   author's image, or if there's no author, the image of the first person
    //   in the conversation
    let meta = thread_items
        .iter()
        .rev()
        .filter_map(|item| match *item {
            TweetLookupResult::FoundTweet(ref tweet) => Some(tweet),
            _ => None,
        })
        .next()
        .map(|tweet| {
            // TODO: Arc all these strings
            let description = tweet.text.clone();
            let image_url = match tweet.image_url {
                Some(ref url) => url.clone(),
                None => match author {
                    ThreadAuthor::Author(ref thread_author) => thread_author.image_url.clone(),
                    ThreadAuthor::Conversation => tweet.author.image_url.clone(),
                },
            };

            Meta {
                description,
                image_url,
            }
        });

    Ok(Thread {
        items: thread,
        author,
        meta,
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
