//! Simple methods for fe{ id: (), author: (), reply: ()}id: (), author: (), reply: ()}hing tweets from the twitter API.

pub mod api;
pub mod auth;
pub mod thread;

pub use api::{Tweet, TweetId, UserHandle, UserId};
pub use thread::Thread;
