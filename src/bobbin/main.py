import os
import pathlib

from aiohttp import web
from autocommand import autocommand
import aiohttp
import cachetools

from bobbin import twitter, tweetbox, async_cache, api_server, web_util, frontend_server


class AsyncLRUCache(async_cache.Cache):
	def __init__(self, max_size):
		self.cache = cachetools.LRUCache(max_size, getsizeof=async_cache.get_size_of)

	async def get(self, key):
		with self.convert_keyerror():
			return self.cache[key]

	async def write(self, key, value):
		self.cache[key] = value


main_handler = web_util.routes(
	(r'/$', frontend_server.index_handler, 'index_path'),
	(r'/thread/[0-9]{1,21}/?$', frontend_server.index_handler, 'index_path'),
	(r'/faq/?$', frontend_server.index_handler, 'index_path'),
	(r'/api/', api_server.handler, 'get_thread'),
	(r'/static/', frontend_server.static_file_handler, ['base_directory', 'valid_paths']),
)


def walk_dir(path):
	for child in path.iterdir():
		if child.is_file():
			yield child
		elif child.is_dir():
			yield from walk_dir(child)


def parse_size(size):
	if size.endswith("KB"):
		return int(size[:-2]) * 1024
	elif size.endswith("MB"):
		return int(size[:-2]) * 1024 * 1024
	elif size.endswith("GB"):
		return int(size[:-2]) * 1024 * 1024 * 1024
	elif size.endswith("B"):
		return int(size[:-1])
	else:
		return int(size)


@autocommand(__name__, loop=True, pass_loop=True)
async def main(
	key: str =os.environ.get("CONSUMER_KEY", None),
	secret: str =os.environ.get("CONSUMER_SECRET", None),
	host="0.0.0.0",
	port=8080,
	static_dir=pathlib.Path('./static'),
	cache_size="256MB",
	loop=None,
):
	if key is None:
		return "Missing CONSUMER_KEY or --key"

	if secret is None:
		return "Missing CONSUMER_SECRET or --secret"

	static_dir = static_dir.resolve()
	if not static_dir.is_dir():
		return "--static_dir must be a directory"

	cache = AsyncLRUCache(max_size=parse_size(cache_size))

	with aiohttp.ClientSession() as session:
		token = twitter.Token(session, key, secret)

		get_thread = tweetbox.make_thread_getter(
			session=session,
			cache=cache,
			token=token
		)

		handler = web_util.with_context(
			web_util.shitty_logging(main_handler),
			get_thread=get_thread,
			base_directory=static_dir,
			valid_paths=None,
			index_path=static_dir / 'index.html'
		)

		http_server = web.Server(handler, loop=loop)
		server = await loop.create_server(http_server, host, port)

		await server.wait_closed()
