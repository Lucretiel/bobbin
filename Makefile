# TODO: Add debug / prod modes. For now we default to debug.
# Open problem to solve: when we switch to production mode, make doesn't
# realize anything has changed, because the dependency chains are still fine.
# Maybe separate directories for dev and prod output?

NODE_MODULES = frontend/node_modules
FONT_SRC_DIR = $(NODE_MODULES)/@fortawesome/fontawesome-free/webfonts

all: js css web webfonts

web: web/target/debug/bobbin
css: static/css
js: static/js
webfonts: static/webfonts

.PHONY: all css js web webfonts

static:
	mkdir static

$(NODE_MODULES): frontend/package.json frontend/yarn.lock
	cd frontend && yarn
	touch -m $@

static/css: $(NODE_MODULES) $(shell find frontend/sass -type f) |static
	cd frontend && yarn run css-build-debug
	touch -m $@

static/js: frontend/webpack.config.ts frontend/tsconfig.json $(NODE_MODULES) |static
static/js: $(shell find frontend/src -type f)
	cd frontend && yarn run webpack --dev
	touch -m $@

static/webfonts: |static $(NODE_MODULES)
	cd static && ln -s ../$(FONT_SRC_DIR)

web/target/debug/bobbin: $(shell find web/src -type f) web/Cargo.toml web/Cargo.lock
	cd web && cargo build

web/target/release/bobbin: $(shell find web/src -type f) web/Cargo.toml web/Cargo.lock
	cd web && cargo build --release

clean-web:
	cd web && cargo clean

clean-css:
	rm -rf static/css

clean-js:
	rm -rf static/js

clean-static:
	rm -rf static

clean-node-modules:
	rm -rf frontend/node_modules

clean-all: clean-web clean-static clean-node-modules

.PHONY: clean-web clean-css clean-js clean-static clean-node-modules
