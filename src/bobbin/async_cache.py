import abc
import contextlib

from bobbin import task_manager


class KeyNotFound(Exception):
	pass


# An interface to an overwhelmingly simple cache. Caches of this type should
# atomically readale, insertable, and volotile. They should store data which can
# easily be reproduced. They should not be updatable. Generally, a key should
# always have the same value. The only use for this is to store which tweets
# are replies to which other tweets.
#
# Reads and writes should freely raise exceptions to communicate errors to the
# user. The only contract is that KeyNotFound should be used if the key is not
# in the cache; this is used by StackedCache to try reads in differnet cache
# layers.
#
# In principle, data generators (like the twitter API) could be used as a non-
# writable cache. The volatility property of caches makes this acceptable.


class Cache(abc.ABC):
	@staticmethod
	@contextlib.contextmanager
	def convert_keyerror():
		try:
			yield
		except KeyError as e:
			raise KeyNotFound(e.args[0]) from e

	@abc.abstractmethod
	async def get(self, key):
		raise NotImplementedError()

	@abc.abstractmethod
	async def write(self, key, value):
		raise NotImplementedError()


class BaseMultiCache(Cache):
	def __init__(self, caches):
		self.caches = caches

	async def write(self, key, value):
		with task_manager.TaskWaiter() as writes:
			for cache in self.caches:
				writes.add_task(cache.write(key, value))

			await writes.wait()


# StackedCache provides layered access to a number of cache layers. Ideally,
# each layer (from first to last) is progressively less volatile but also
# slower.
class SimpleStackedCache(BaseMultiCache):
	async def get(self, key):
		'''
		Attempt to get a value by trying each cache in order. If the read
		succeeds, and write is True, also write the read value to the caches
		that missed.
		'''
		# TODO: Background, non-blocking writes. Need to figure out a way to
		# emit errors in this case.
		for child in self.caches:
			try:
				return (await child.get(key))
			except KeyNotFound:
				pass

		raise KeyNotFound(key)


class BaseUpdaterStackedCache(BaseMultiCache):
	async def _get_with_writes(self, key, write_manager):
		cache_misses = []

		for child in self.caches:
			try:
				result = await child.get(key)
				break
			except KeyNotFound:
				cache_misses.append(child)
		else:
			raise KeyNotFound(key)

		for cache in cache_misses:
			write_manager.add_task(cache.write(key, result))

		return result


# Same as SimpleStackedCache, but caches that don't contain the target key
# are updated with it. If a write_manager is provided, writes are submitted to
# it, and do not block the get call.
class StackedCache(BaseUpdaterStackedCache):
	def __init__(self, caches, write_manager):
		super().__init__(caches)
		self.write_manager = write_manager

	async def get(self, key):
		return (await self._get_with_writes(key, self.write_manager))


class SyncStackedCache(BaseUpdaterStackedCache):
	async def get(self, key):
		with task_manager.TaskWaiter() as writes:
			result = await self._get_with_writes(key, writes)
			await writes.wait()

		return result
