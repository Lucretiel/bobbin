# Low level async interface for twitter

from base64 import b64encode
from collections import namedtuple
from functools import lru_cache
from urllib.parse import quote as url_encode

from bobbin import async_util

BASE_API_URL = "https://api.twitter.com"

BASE_OAUTH_URL = f"{BASE_API_URL}/oauth2"
TOKEN_URL = f"{BASE_OAUTH_URL}/token"
RELEASE_URL = f"{BASE_OAUTH_URL}/invalidate_token"

API_URL = f"{BASE_API_URL}/1.1"
USER_TIMELINE_URL = f"{API_URL}/statuses/user_timeline"
TWEET_URL = f"{API_URL}/statuses/show.json"


class TwitterError(Exception):
	pass


class RateLimitError(TwitterError):
	pass


class TwitterIDError(TwitterError):
	pass


class NoSuchTweetError(TwitterIDError):
	pass


class NoSuchUserError(TwitterIDError):
	pass


@lru_cache()
def encode_twitter_key(*, consumer_key: str, consumer_secret: str):
	return "Basic {code}".format(code=b64encode(
		"{key}:{secret}".format(
			key=url_encode(consumer_key),
			secret=url_encode(consumer_secret)
		).encode(encoding='ascii')
	).decode(encoding='ascii'))


@lru_cache()
def encode_bearer_token(token):
	return 'Bearer {}'.format(token)


@async_util.shared_concurrent
async def generate_bearer_token(*, session, consumer_key, consumer_secret):
	headers = {
		"Authorization": encode_twitter_key(
			consumer_key=consumer_key,
			consumer_secret=consumer_secret,
		),
		"Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
		"Accept": "application/json",
	}

	async with session.post(
		url=TOKEN_URL,
		headers=headers,
		data=b"grant_type=client_credentials",
	) as response:
		# TODO: handle errors better
		response.raise_for_status()
		result = await response.json()

	if result['token_type'] != "bearer":
		raise Exception('Token type wasn\'t "bearer"')

	return encode_bearer_token(result["access_token"])


class Token:
	def __init__(self, session, consumer_key, consumer_secret):
		self.session = session
		self.consumer_key = consumer_key
		self.consumer_secret = consumer_secret
		self.token = None

	async def regenerate(self):
		token = self.token = await generate_bearer_token(
			session=self.session,
			consumer_key=self.consumer_key,
			consumer_secret=self.consumer_secret,
		)
		return token

	async def get_token(self):
		token = self.token
		if token is None:
			token = await self.regenerate()
		return token


class TwitterUser(namedtuple("TwitterUser", "id handle name")):
	__slots__ = ()

	@lru_cache()
	def __new__(cls, id, handle, name):
		return super().__new__(cls, id, handle, name)

	@classmethod
	def from_user_json(cls, blob):
		return cls(
			blob["id_str"],
			blob["screen_name"],
			blob["name"]
		)


class Tweet(namedtuple("Tweet", "id user parent_id parent_user_id")):
	__slots__ = ()

	@lru_cache()
	def __new__(cls, id, user, parent, parent_user_id):
		return super().__new__(cls, id, user, parent, parent_user_id)

	@classmethod
	def from_tweet_json(cls, blob):
		return cls(
			blob["id_str"],
			TwitterUser.from_user_json(blob["user"]),
			blob["in_reply_to_status_id_str"],
			blob["in_reply_to_user_id_str"],
		)


# TODO: find a better way to report errors related to rate limiting

@async_util.shared_concurrent
async def get_tweet(*, session, token, tweet_id):
	if isinstance(token, Token):
		token = await token.get_token()

	async with session.get(
		url=TWEET_URL,
		params={
			"id": tweet_id,
			"include_entities": "false",
			"include_ext_alt_text": "false",
		},
		headers={
			"Authorization": token,
			"Accept": "application/json",
		}
	) as response:
		response.raise_for_status()
		result = await response.json()

	return Tweet.from_tweet_json(result)


@async_util.shared_concurrent
async def get_user_tweets(*, session, token, user_id, max_tweet, count=200):
	if isinstance(token, Token):
		token = await token.get_token()

	async with session.get(
		url=USER_TIMELINE_URL,
		params={
			"user_id": user_id,
			"count": count,
			"max_id": max_tweet,
			"exclude_replies": "false",
			"include_rts": "true",
		},
		headers={
			"Authorization": token,
			"Accept": "application/json"
		},
	) as response:
		# TODO: handle errors better
		response.raise_for_status()
		result = await response.json()

	# Ordinarily I dislike pre-emptively unrolling iterators like this, but in
	# this case we don't want to carry around the immense json value.
	return list(map(Tweet.from_tweet_json, result))
