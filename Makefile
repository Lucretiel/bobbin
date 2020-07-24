# TODO: Add debug / prod modes. For now we default to prod.

YARN = yarn --cwd frontend

all: js css web

web: web/target/debug/bobbin
css: static/css
js: static/js

frontend/node_modules: frontend/package.json frontend/yarn.lock
	$(YARN)
	touch -m $@

static/css: frontend/node_modules $(shell find frontend/sass -type f)
	$(YARN) run css-build
	touch -m $@

static/js: frontend/node_modules $(shell find frontend/src -type f) frontend/tsconfig.json
	$(YARN) run webpack --prod
	touch -m $@

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

.PHONY: all css js web clean clean-web clean-css clean-js clean-static clean-node-modules
