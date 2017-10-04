import functools
import asyncio
import collections


def inject_loop(func):
	@functools.wraps(func)
	def inject_loop_wrapper(*args, loop=None, **kwargs):
		return func(*args, loop=(asyncio.get_event_loop() if loop is None else loop), **kwargs)
	return inject_loop_wrapper


def invoke_coro(coro, args):
	if callable(coro):
		return coro(*args)
	elif args:
		raise ValueError(f"Provided args with non-callable coroutine: {args}")
	else:
		return coro


class TaskWaiter:
	@inject_loop
	def __init__(self, *, loop):
		self.loop = loop
		self.waiters = set()
		self.running = set()

	def __done(self, task):
		self.running.discard(task)
		if not self.running:
			waiters = self.waiters
			for waiter in waiters:
				if not waiter.done():
					waiter.set_result(None)
			waiters.clear()

	def add_task(self, task):
		task = asyncio.ensure_future(task, loop=self.loop)
		self.running.add(task)
		task.add_done_callback(self.__done)
		return task

	def __enter__(self):
		return self

	def __exit__(self, *exc):
		# Note that this will simply cancel all ongoing tasks. You must
		# explicitly wait for the task manager at the end. This is because
		# errors from awaiting the remaining tasks can't reasonably be handled,
		# and more generally because it's bad practice to delay context
		# termination.
		running = self.running
		waiters = self.waiters

		self.running = set()
		self.waiters = set()

		for waiter in waiters:
			if not waiter.done():
				waiter.set_result(None)

		waiters.clear()

		for task in running:
			if not task.done():
				task.cancel()

		running.clear()

	async def wait(self, *, instant=True):
		if instant:
			if not self.running:
				return

		waiter = asyncio.Future(loop=self.loop)
		waiter.add_done_callback(lambda fut: self.waiters.discard(waiter))
		self.waiters.add(waiter)
		try:
			await waiter
		finally:
			waiter.cancel()


class TaskLimiter:
	@inject_loop
	def __init__(self, max_tasks, *, loop):
		self.max_tasks = max_tasks
		self.running = 0
		self.starters = collections.deque()
		self.loop = loop

	async def __task_runner(self, coro, args, start_waiter):
		# Will raise exception if cancelled
		await start_waiter
		return (await invoke_coro(coro, args))

	def __launch_next(self, fut):
		'''
		Callback passed to task.add_done_callback which sets the next task to
		launch when the current task ends
		'''
		starters = self.starters
		if self.running <= self.max_tasks:
			while starters:
				starter = starters.popleft()
				if not starter.done():
					starter.set_result(None)
					return
		self.running -= 1

	def schedule(self, coro, *args):
		if self.running < self.max_tasks:
			self.running += 1
			task = asyncio.ensure_future(invoke_coro(coro, args))
		else:
			waiter = asyncio.Future(loop=self.loop)
			self.starters.append(waiter)
			task = asyncio.ensure_future(self.__task_runner(coro, args, waiter))

		task.add_done_callback(self.__launch_next)
		return task
