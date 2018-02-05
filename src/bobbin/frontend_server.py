# This file serves javascript, html, etc.

import pathlib
from aiohttp import web
from bobbin import web_util


@web_util.final_route
@web_util.route(r"/(?P<path>[a-zA-Z0-9._-]+(?:/[a-zA-Z0-9._-]+)*)$")
@web_util.method_handler('GET', 'HEAD')
async def static_file_handler(request, *, base_directory: pathlib.Path, path, valid_paths):
	path = pathlib.Path(path)

	if '..' in path.parts:
		raise web.HTTPNotFound(body=b'')

	if valid_paths is not None and path not in valid_paths:
		raise web.HTTPNotFound(body=b'')

	complete_path = base_directory.joinpath(path).resolve()

	try:
		complete_path.relative_to(base_directory)
	except ValueError as e:
		raise web.HTTPNotFound(body=b'') from e

	if not complete_path.is_file():
		raise web.HTTPNotFound(body=b'')

	# TODO: Pull request for FileResponse to serve brotli as well as gzip
	return web.FileResponse(complete_path, chunk_size=1024 * 1024)


@web_util.method_handler('GET', 'HEAD')
async def index_handler(request, index_path):
	return web.FileResponse(index_path, )
