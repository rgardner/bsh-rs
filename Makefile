FEATURES ?= --features "dev"

all: build

build:
	cargo clean
	multirust run nightly cargo build $(FEATURES)

.PHONY: all build
