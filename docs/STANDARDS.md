# Rust standards and practices

Follow this doc for all code in this repo. The Cursor rule references it; keep one canonical way of doing things.

## Principles

- **Uniform** — One preferred way per concern. Don’t introduce a second style without a documented reason.
- **Anti-fragile** — Prefer types and invariants that make invalid state unrepresentable. Use `Result`/`Option`, avoid panics in library code, validate at boundaries.
- **Scalable** — Structure so we can add domains (crypto/sports/weather) and strategies without rewriting. Shared traits and modules; domain-specific code under `src/strategies/<domain>/`.
- **Flexible** — Config and behaviour via env and config types, not hardcoding. Strategies and data sources pluggable (traits, optional deps).
- **Consistent** — Same patterns everywhere: error handling, naming, module layout, tests.

## Layout

- **Modules:** One main idea per file. `mod.rs` re-exports or orchestrates; avoid huge files.
- **Naming:** `snake_case` for items; `PascalCase` for types. Names reflect purpose (e.g. `parse_execution_mode`, `Executor`, `PmClient`).
- **Paths:** Domain data and backtest live under `src/strategies/<domain>/data/` and `.../backtest/`. Shared code in `src/shared/`.

## Errors and results

- Use `Result<T, E>` with `anyhow::Result` in app code, `thiserror` for library-style error types when callers need to match.
- Propagate with `?`; convert at boundaries (e.g. `anyhow::Context`).
- No `unwrap()`/`expect()` in library or hot paths unless documented (e.g. “invariant guaranteed by caller”). In binaries, prefer logging and early exit.

## Async and concurrency

- Prefer `tokio`; use `async` only where I/O or timers are needed.
- Prefer structured concurrency (tasks, channels) over raw threads. Share state via `Arc`/`RwLock` or message passing.

## Testing

- Unit tests in the same crate: `#[cfg(test)] mod tests { ... }` in the module.
- Test critical behaviour: config parsing, paper vs live execution gate, strategy output shape. Prefer testing pure logic and boundaries; mock or stub I/O when needed.
- Run `cargo test` and `cargo clippy -- -D warnings` before merging.

## Dependencies

- Add deps only when needed. Prefer std and widely used crates. Pin versions in `Cargo.toml`; avoid wildcards.

## Documentation

- Public API: doc comments on types and public functions. Keep them short; point to `docs/` for design.
- Inline comments only for non-obvious “why”, not “what”.

## Cursor

- The rule `.cursor/rules/rust-standards.mdc` points to this doc. When in doubt, follow this file and keep the rule in sync.
