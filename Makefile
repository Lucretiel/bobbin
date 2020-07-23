YARN = yarn --cwd frontend

all: js css

css: static/css
js: static/js

frontend/node_modules: frontend/package.json frontend/yarn.lock
	$(YARN)
	touch -m $@

static/css: frontend/node_modules $(shell find frontend/sass -type f)
	$(YARN) run css-build
	touch -m $@

static/js: frontend/node_modules $(shell find frontend/src -type f)
	$(YARN) run js-build
	touch -m $@

.PHONY: all css js
