.PHONY: all compressed bundle zopfli gzip brotli sizes clean mod-clean clean-all compressed

WEBPACK_OUTPUT_DIR ?= $(PWD)/static/dist

BUNDLEJS = $(WEBPACK_OUTPUT_DIR)/bundle.js
BUNDLEBR = $(BUNDLEJS).br
BUNDLEGZ = $(BUNDLEJS).gz

SRC_FILES = $(shell find frontend-src -type f)
WEBPACK = $(shell npm bin)/webpack
BROTLI = $(shell which bro brotli)
ZOPFLI = $(shell which zopfli)

all: bundle compressed
compressed: zopfli brotli
bundle: $(BUNDLEJS)
zopfli: $(BUNDLEGZ)
brotli: $(BUNDLEBR)
gzip: zopfli

sizes:
	ls -lh $(WEBPACK_OUTPUT_DIR)

compressed: zopfli brotli

ifeq ($(NODE_ENV),production)
ENV = production
WEBPACK_FLAGS = -p

else ifeq ($(NODE_ENV),development)
ENV = development
WEBPACK_FLAGS = -d

else
ENV = development
WEBPACK_FLAGS =

endif

$(BUNDLEJS): $(SRC_FILES) \
	webpack.config.js \
	node_modules \
	$(WEBPACK_OUTPUT_DIR)

	env NODE_ENV=$(ENV) $(WEBPACK) --progress $(WEBPACK_FLAGS) --output-path $(WEBPACK_OUTPUT_DIR)

$(BUNDLEBR): $(BUNDLEJS)
	$(BROTLI) < $(BUNDLEJS) > $(BUNDLEBR)

$(BUNDLEGZ): $(BUNDLEJS)
	$(ZOPFLI) $(BUNDLEJS) -c > $(BUNDLEGZ)

node_modules: package.json $(wildcard yarn.lock)
	yarn install
	touch -ma node_modules

clean-all: clean mod-clean

clean:
	rm -rf $(WEBPACK_OUTPUT_DIR)

mod-clean:
	rm -rf node_modules
