FEATURES ?= --features "dev"

all: build

build:
	cargo build

dev:
	multirust run nightly cargo do clean, build $(FEATURES)

.PHONY: all build dev
