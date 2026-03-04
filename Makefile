.PHONY: build build-release run run-release check fmt fmt-check lint test test-v test-live coverage creds help

## Build
build:
	cargo build

build-release:
	cargo build --release

## Run
run:
	cargo run

run-release:
	cargo run --release

## Check (no codegen)
check:
	cargo check

## Format
fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

## Lint
lint:
	cargo clippy -- -D warnings

## Test
test:
	cargo test

test-v:
	cargo test -- --nocapture

test-live:
	cargo test -- --ignored

## Coverage (opens HTML report in browser)
coverage:
	cargo llvm-cov \
		--all-features \
		--workspace \
		--ignore-filename-regex "(tui/views/|tui/runner\.rs)" \
		--open

## CI (fmt-check + lint + test — mirrors the CI pipeline locally)
ci:
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test

## Derive Polymarket API credentials from PRIVATE_KEY in .env
creds:
	python scripts/derive_polymarket_creds.py

## Help
help:
	@grep -E '^##' Makefile | sed 's/^## //'
