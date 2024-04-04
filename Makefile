.PHONY: web
web: build
	ttyd ./target/debug/minmux -e bash

.PHONY: webmc
webmc: build
	@ttyd ./target/debug/minmux -e /usr/local/bin/mc

.PHONY: build
build:
	@cargo build
