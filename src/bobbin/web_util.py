import functools
import re
import inspect
import enum
from json import dumps

from aiohttp import web


# This is what I'm using instead of a "real" web framework. I'm actually pretty
# happy with it. Something to note, though: Even though all handlers are
# technically coroutines, many of decorator wrappers here are plain functions,
# and in fact many of them are *required* to be plain functions. Most of the
# processing– including routing, method matching, and so on– check for raised
# exceptions in the intial call, without any await calls.


def dump_json(**kwargs):
	return dumps(kwargs, check_circular=False, separators=(',', ':'))


def bad_request_json(error, **kwargs):
	return web.HTTPBadRequest(
		text=dump_json(error=error, **kwargs),
		content_type='application/json'
	)


def with_context(handler=None, **context):
	if handler is None:
		return lambda handler: with_context(handler, **context)

	@functools.wraps(handler)
	def context_handler(request, **kwargs):
		return handler(request, **kwargs, **context)
	return context_handler


def only_context(*context_keys, handler=None):
	'''
	Make the handler ignore all kwargs that are not included in context_keys.
	Intended to be used near the top of the handler tree, to ensure that
	appropriate context is only routed to the correct child routes.
	'''
	if handler is None:
		return lambda handler: only_context(*context_keys, handler=handler)

	context_keys = set(context_keys)

	@functools.wraps(handler)
	def only_context_wrapper(request, **kwargs):
		return handler(request, **{k: v for k, v in kwargs.items() if k in context_keys})

	return only_context_wrapper


class MissingContextError(Exception):
	pass


def requires_context(*context_keys, handler=None):
	'''
	Assert that the context_keys exist in kwargs, raising an error if they
	don't. Generally this should be used for debugging.
	'''
	# TODO: add some kind of debug variable that we can use to skip this check
	# in production
	if handler is None:
		return lambda handler: requires_context(*context_keys, handler=handler)

	context_keys = set(context_keys)

	@functools.wraps(handler)
	def requires_context_wrapper(request, **kwargs):
		for key in context_keys:
			if key not in kwargs:
				raise MissingContextError(key)

		return handler(request, **kwargs)

	return requires_context_wrapper


def convert_context(handler=None, **rename_keys):
	'''
	Renamed context keys for the wrapped handler. It is left unspecified what
	happens to conflicting keys.
	'''

	# TODO: ensure that this conversion is the same size as the original
	rename_keys = {input_key: output_key for output_key, input_key in rename_keys.items()}

	@functools.wraps(handler)
	def convert_context_wrapper(request, **kwargs):
		return handler(request, **{
			(rename_keys.get(key, key)): value
			for key, value in kwargs.items
		})

	return convert_context_wrapper


QueryParam = object()


class QueryError(enum.Enum):
	unexpected = 0
	duplicate = 1
	missing = 2


def with_query(error_handler=None, handler=None, ignore_unexpected=False):
	'''
	Inject all the query fields as handler context. To prevent accidentally
	leaking implementation info to users of the API, handler parameters
	associated with the query must be annotated with QueryParam:

	@with_query()
	def handler(request, query1: QueryParam)

	error_handler is called with (kind, key) in the event of an error. Errors
	include missing keys, duplicate keys, and unexpected keys.
	'''
	if handler is None:
		return lambda handler: with_query(error_handler, handler)

	if error_handler is None:
		def error_handler(kind, key):
			raise web.HTTPBadRequest()

	sig = inspect.signature(handler)

	all_query_params = {
		key for key, param in sig.parameters.items()
		if param.annotation is QueryParam
	}

	required_query_params = {
		key for key in all_query_params
		if sig.parameters[key].default is sig.empty
	}

	@functools.wraps(handler)
	def query_handler(request, **kwargs):
		required = set(required_query_params)
		bound_query_params = {}

		for key, value in request.query.items():
			if key not in all_query_params:
				if not ignore_unexpected:
					return error_handler(QueryError.unexpected, key)
			elif key in bound_query_params:
				return error_handler(QueryError.duplicate, key)
			else:
				bound_query_params[key] = value
				required.discard(key)

		if required:
			return error_handler(QueryError.missing, required.pop())

		return handler(request, **kwargs, **bound_query_params)

	return query_handler


def query_error_handler_json(kind: QueryError, key: str):
	if kind is QueryError.duplicate:
		msg = f"unexpected duplicate query parameter: {key}"
	elif kind is QueryError.missing:
		msg = f"missing required query parameter: {key}"
	elif kind is QueryError.unexpected:
		msg = f"unexpected query parameter: {key}"
	else:
		raise RuntimeError("Unnaccounted for QueryError type")

	raise bad_request_json(msg)


def compose_handlers(handlers, IgnoredError, error_factory=None):
	'''
	Given a list of web handlers, create a new one which tries each on in
	sequence. Each time a handler raises an Exception matching IgnoredError,
	the next handler is tried. If none of them succeed, error_facrotr() is
	raised, defaulting to IgnoredError.
	'''
	if error_factory is None:
		error_factory = IgnoredError

	handlers = tuple(handlers)

	def compose_wrapper(request, **kwargs):
		for handler in handlers:
			try:
				return handler(request, **kwargs)
			except IgnoredError:
				pass

		raise error_factory()

	return compose_wrapper


def method_handler(*methods, inject=False, handler=None):
	if handler is None:
		return lambda handler: method_handler(*methods, inject=inject, handler=handler)

	methods = frozenset(method.upper() for method in methods)

	@functools.wraps(handler)
	def method_handler_wrapper(request, **kwargs):
		method = request.method.upper()
		if method not in methods:
			raise web.HTTPMethodNotAllowed(method=method, allowed_methods=methods)

		if inject:
			return handler(request, **kwargs, method=method)
		else:
			return handler(request, **kwargs)

	return method_handler_wrapper


def _flatten_methods(methods):
	for method in methods:
		if isinstance(method, str):
			yield from method.split()
		else:
			yield from _flatten_methods(method)


def _make_method_handler(thing, inject):
	if callable(thing):
		return thing

	*methods, handler = thing

	return method_handler(*_flatten_methods(methods), handler=handler, inject=inject)


def methods(*targets, inject=False):
	return compose_handlers(
		(_make_method_handler(target, inject) for target in targets),
		IgnoredError=web.HTTPMethodNotAllowed
	)


class RouteNotFound(web.HTTPNotFound):
	def __init__(self, body=b''):
		super().__init__(body=body)


def _make_route_handler(url, handler):
	pattern = re.compile(url)

	@functools.wraps(handler)
	def route_handler(request, **kwargs):
		url = request.rel_url
		url_path = url.path

		match = pattern.match(url_path)

		if match is None:
			raise RouteNotFound()

		new_request = request.clone(
			rel_url=url.with_path(
				url_path[match.end():]
			).with_query(url.query_string)  # TODO: replace this with efficient version
		)

		return handler(new_request, **kwargs, **match.groupdict())

	return route_handler


def final_route(handler):
	'''
	Any RouteNotFound exceptions raised from the wrapped handler will be
	converted to HTTPNotFound exceptions. This allows the creation of route
	barriers, such that once a handler function is selected, no other choices
	in that route group can be tried.
	'''
	def final_route_wrapper(request, **kwargs):
		try:
			return handler(request, **kwargs)
		except RouteNotFound as e:
			raise web.HTTPNotFound(
				headers=e.headers,
				reason=e.reason,
				body=e.body,
				content_type=e.content_type
			) from e
	return final_route_wrapper


def route(path, handler=None):
	'''
	Given a handler, wrap the handler such that it can only serve a path. The
	path should should be a regular expression pattern (or compiled regular
	expression object) that will be matched against the request path. If the
	path matches, the matching part of the path will be removed from the
	request path, and the handler will be invoked. Any named groups in the
	regex match will be added to the handler's context.

	If the path does not match, a RouteNotFound exception is raised. This is
	a subclass of HTTPNotFound, and is used by routes (below) to search for a
	matching route.
	'''
	if handler is None:
		return lambda handler: route(path, handler)

	if isinstance(path, str):
		pattern = re.compile(path)
	else:
		pattern = path

	@functools.wraps(handler)
	def route_handler(request, **kwargs):
		url = request.rel_url
		url_path = url.path

		match = pattern.match(url_path)

		if match is None:
			raise RouteNotFound()

		new_request = request.clone(
			rel_url=url.with_path(
				url_path[match.end():]
			).with_query(url.query_string)  # TODO: replace this with efficient version
		)

		return handler(new_request, **kwargs, **match.groupdict())

	return route_handler


def _make_route(thing):
	if callable(thing):
		return thing
	elif len(thing) == 2:
		path, handler = thing
		return route(path, handler)
	elif len(thing) == 3:
		path, handler, context_keys = thing

		if isinstance(context_keys, str):
			context_keys = [context_keys]

		return route(path, only_context(*context_keys, handler=handler))


def routes(*routes):
	return compose_handlers(map(_make_route, routes), RouteNotFound)


def shitty_logging(handler):
	def shitty_log_handler(request, **context):
		print(request)
		return handler(request, **context)
	return shitty_log_handler
