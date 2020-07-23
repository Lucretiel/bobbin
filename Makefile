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
	$(YARN) run js-build
	touch -m $@

web/target/debug/bobbin: $(shell find web/src -type f) web/Cargo.toml web/Cargo.lock
	cd web && cargo build

web/target/release/bobbin: $(shell find web/src -type f) web/Cargo.toml web/Cargo.lock
	cd web && cargo build --release

.PHONY: all css js web
