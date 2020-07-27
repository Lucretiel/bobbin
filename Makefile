# TODO: Add debug / prod modes. For now we default to debug.
# Open problem to solve: when we switch to production mode, make doesn't
# realize anything has changed, because the dependency chains are still fine.
# Maybe separate directories for dev and prod output?

STATIC_DIR = frontend/static
NODE_MODULES = frontend/node_modules
FONT_SRC_DIR = $(NODE_MODULES)/@fortawesome/fontawesome-free/webfonts

all: js css web webfonts

web: web/target/debug/bobbin
css: $(STATIC_DIR)/css
js: $(STATIC_DIR)/js
webfonts: $(STATIC_DIR)/webfonts

.PHONY: all css js web webfonts

$(NODE_MODULES): frontend/package.json frontend/yarn.lock
	cd frontend && yarn
	touch -m $@

$(STATIC_DIR)/css: $(NODE_MODULES) $(shell find frontend/sass -type f)
	cd frontend && yarn run css-build-debug
	touch -m $@

$(STATIC_DIR)/js: frontend/webpack.config.ts frontend/tsconfig.json $(NODE_MODULES)
$(STATIC_DIR)/js: $(shell find frontend/src -type f)
	cd frontend && yarn run webpack --dev
	touch -m $@

$(STATIC_DIR)/webfonts: $(NODE_MODULES)
	mkdir -p $@
	cp $(FONT_SRC_DIR)/fa-solid* $@
	touch -m $@

web/target/debug/bobbin: $(shell find web/src -type f) web/Cargo.toml web/Cargo.lock
	cd web && cargo build

web/target/release/bobbin: $(shell find web/src -type f) web/Cargo.toml web/Cargo.lock
	cd web && cargo build --release

clean-web:
	cd web && cargo clean

clean-css:
	rm -rf $(STATIC_DIR)/css

clean-js:
	rm -rf $(STATIC_DIR)/js

clean-static:
	rm -rf $(STATIC_DIR)

clean-node-modules:
	rm -rf frontend/node_modules

clean-all: clean-web clean-static clean-node-modules

.PHONY: clean-web clean-css clean-js clean-static clean-node-modules
