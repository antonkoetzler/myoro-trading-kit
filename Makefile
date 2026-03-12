.PHONY: build build-release run run-release check fmt fmt-check lint test test-v test-live coverage \
        ui-install ui-dev ui-build ui-test ui-coverage dev build-tauri ci ci-full creds help

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
		--ignore-filename-regex "(commands/|app_state\.rs)" \
		--open

## Frontend
ui-install:
	cd ui && npm install

ui-dev:
	cd ui && npm run dev

ui-build:
	cd ui && npm run build

ui-test:
	cd ui && npm run test

ui-coverage:
	cd ui && npm run coverage

## Tauri
dev:
	cargo tauri dev

build-tauri:
	cargo tauri build

## CI (fmt-check + lint + test — mirrors the CI pipeline locally)
ci:
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test

## Full CI (Rust + frontend)
ci-full:
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) ui-test

## Derive Polymarket API credentials from PRIVATE_KEY in .env
creds:
	python scripts/derive_polymarket_creds.py

## Help
help:
	@grep -E '^##' Makefile | sed 's/^## //'
