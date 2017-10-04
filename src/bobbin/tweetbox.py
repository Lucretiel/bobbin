from pickle import dumps as pickle_dump, loads as pickle_load

from bobbin.async_cache import KeyNotFound, Cache as TweetCache
from bobbin.async_util import shared_concurrent
from bobbin.twitter import get_tweet, get_user_tweets, Tweet
from bobbin.task_manager import TaskWaiter

# This is the primary interface where the logic lives. It handles caching and
# the algorithmic decisions of which APIs to use


class InvalidThreadError(Exception):
	pass


class MismatchedHeadError(InvalidThreadError):
	pass


async def generate_thread(*, session, cache: TweetCache, token, tail, head=None):
	'''
	Get a list a tweet IDs comprising a thread, in order from tail to
	head.

	Algorithm: starting with the tail, look up each tweet in the cache,
	working our way backwards. If we ever find a tweet not in the cahce,
	look it up via the API. If that tweet's parent is ALSO not in the
	cache, do a timeline lookup on that tweet's user. Fall back to
	individual lookups if the timeline fails for some reason (probably
	because timelines only go back 3,200 tweets). Once the full thread is
	found, insert tweets into the cache. Tweets are yielded in reverse order.
	Threads are yielded, but if the head tweet is never found, an exception is
	rasied.

	Cache should have async "get" and "write" methods.
	'''

	# local_store is where tweets pulled from the API live. Tweets retreived
	# from this store (via get_cached_tweet) are stored in the cache
	local_store = {}
	writers = TaskWaiter()

	def store_tweet_bg(tweet_id, tweet: Tweet):
		'''
		Given a tweet ID and a parent ID, schedule the parent-child
		to be stored in the background.
		'''
		writers.add_task(cache.write(tweet_id, pickle_dump(tweet, protocol=4)))

	async def get_cached_tweet(tweet_id):
		'''
		Returns a parent id, or None, or raise an exception, from the cache.
		Checks the local_store, which is tweets found from a user_timeline
		lookup. If not found, check the cache,
		'''
		# TODO: work out if it's better to check the real cache first
		try:
			tweet = local_store[tweet_id]
		except KeyError:
			pass
		else:
			store_tweet_bg(tweet_id, tweet)
			return tweet

		return pickle_load(await cache.get(tweet_id))

	async def load_tweets(tweet_id):
		'''
		Given a total cache miss (not available in the cache OR in the local
		store), we have to hit the API. Look up the tweet, then also look up
		the prior 100 tweets by that person, storing them all in the local
		store. We don't want to over-cache, so the get_cached_tweet function
		ensures that only local_store tweets that are actually part of the
		thread are written to the cache. The intial tweet is, of couse, cached.

		This function only returns the parent id for the tweet, but as a side-
		effect, attempts to populate the local_store with extra tweets from the
		user's timeline.
		'''

		# TODO: HANDLE ALL THE ERRORS
		tweet = await get_tweet(session=session, token=token, tweet_id=tweet_id)
		store_tweet_bg(tweet_id, tweet)

		if tweet.parent_user_id is None:
			return tweet

		# TODO: ignore most errors here
		user_tweets = await get_user_tweets(
			session=session,
			token=token,
			user_id=tweet.parent_user_id,
			max_tweet=tweet_id,
			count=100
		)

		for user_tweet in user_tweets:
			local_store[user_tweet.id] = user_tweet

		return tweet

	tweet_id = tail

	with writers:
		while tweet_id is not None:
			try:
				tweet = await get_cached_tweet(tweet_id)
			except KeyNotFound:
				tweet = await load_tweets(tweet_id)

			yield tweet

			if head is not None:
				if tweet_id == head:
					break
				elif tweet.parent_id is None:
					raise InvalidThreadError(head)

			tweet_id = tweet.parent_id

		await writers.wait(instant=True)


async def get_thread(*, session, cache, token, tail, head=None):
	return list(reversed([tweet async for tweet in generate_thread(
		session=session,
		cache=cache,
		token=token,
		tail=tail,
		head=head
	)]))


def make_thread_getter(*, session, cache, token):
	@shared_concurrent
	def local_get_thread(*, tail, head=None):
		return get_thread(session=session, cache=cache, token=token, tail=tail, head=head)
	return local_get_thread
