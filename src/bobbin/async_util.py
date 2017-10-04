import asyncio
import functools


def inject_loop(func):
	get_event_loop = asyncio.get_event_loop

	@functools.wraps(func)
	def inject_loop_wrapper(*args, loop=None, **kwargs):
		if loop is None:
			loop = get_event_loop()
		return func(*args, loop=loop, **kwargs)
	return inject_loop_wrapper


def make_key(kwargs):
	return frozenset(kwargs.items())


def shared_concurrent(func):
	'''
	Make an async function shared. The function should accept only kwargs.
	While the function is running, subsequent concurrent calls to the same
	function will simply await the initial instance.

	Note that, while this is basically like a cahce, we explicilty discard the
	result after the function is running. Users interested in caching should
	layer their own solutions on top of this one.
	'''
	running_tasks = {}

	@functools.wraps(func)
	async def shared_concurrent_wrapper(**kwargs):
		key = make_key(kwargs)
		try:
			task = running_tasks[key]
		except KeyError:
			task = running_tasks[key] = asyncio.ensure_future(func(**kwargs))
			task.add_done_callback(lambda task: running_tasks.pop(key, None))

		# TODO: Update shared_concurrent to cancel the task when there are no
		# more waiters.
		return (await asyncio.shield(task))

	return shared_concurrent_wrapper
