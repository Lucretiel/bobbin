from collections import Counter

from aiohttp import web

from bobbin import web_util


def is_valid_tweet_id(tweet_id):
	'''
	Doesn't check if the tweet actually exists, just that the string pattern
	is that of a tweet
	'''
	return (1 <= len(tweet_id) <= 20) and tweet_id.isdecimal()


def get_thread_author(thread):
	user_counts = Counter(tweet.user for tweet in thread)

	if len(user_counts) == 0:
		return None
	elif len(user_counts) == 1:
		return user_counts.popitem()[0]

	top_users = user_counts.most_common(2)

	if top_users[0][1] > top_users[1][1] and top_users[0][1] * 2 >= len(user_counts):
		return top_users[0][0]
	else:
		return None


@web_util.method_handler('GET')
@web_util.with_query(web_util.query_error_handler_json)
async def thread_handler(
	request, *,
	get_thread,
	tail: web_util.QueryParam,
	head: web_util.QueryParam =None
):
	if not is_valid_tweet_id(tail):
		raise web_util.bad_request("Invalid tweet id", param="tail", tweet_id=tail)

	if head is not None and not is_valid_tweet_id(head):
		raise web_util.bad_request("Invalid tweet id", param="head", tweet_id=head)

	thread = await get_thread(tail=tail, head=head)
	thread_tweet_ids = [tweet.id for tweet in thread]
	author = get_thread_author(thread)

	return web.Response(
		text=web_util.dump_json(
			thread=thread_tweet_ids,
			author={
				"handle": author.handle,
				"name": author.name,
			} if author is not None else None),
		content_type="application/json",
	)


handler = web_util.routes(
	(r"/thread/?$", thread_handler)
)
