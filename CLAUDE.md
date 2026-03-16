# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## AI Assistant Rules

**All AI assistant rules are centralized in `docs/ai-rules/`.** Read and follow these files:

- **[code-owner.md](docs/ai-rules/code-owner.md)** — You own the full development lifecycle. No "TODO" or "fix later" handoffs.
- **[polymarket-arbitrage.md](docs/ai-rules/polymarket-arbitrage.md)** — Building hands-off money printers. Edge first, automate everything, data over gut, risk-aware.
- **[rust-standards.md](docs/ai-rules/rust-standards.md)** — Follow [docs/standards/STANDARDS.md](docs/standards/STANDARDS.md) for all Rust code.
- **[concise-responses.md](docs/ai-rules/concise-responses.md)** — Be concise and to-the-point. No long descriptions.
- **[plans-contain-only-plan.md](docs/ai-rules/plans-contain-only-plan.md)** — Plan documents contain only concrete steps. No "TBD" sections.
- **[visual-and-themes.md](docs/ai-rules/visual-and-themes.md)** — When changing GUI colors, apply to ALL theme presets.
- **[file-size.md](docs/ai-rules/file-size.md)** — Max 300 lines per file. Split complex domains into subdirectory files. No stubs.
- **[testing-and-quality.md](docs/ai-rules/testing-and-quality.md)** — Keep tests in sync, run fmt/clippy, maintain line coverage threshold.
- **[bun-only.md](docs/ai-rules/bun-only.md)** — Use bun exclusively. Never npm, npx, yarn, or pnpm.

## Project Context

**Myoro Trading Kit** — Rust GUI for automated Polymarket trading across crypto, sports, and weather markets.

**Stack:** Rust (lib) + Tauri v2 + React + ShadCN + TypeScript (GUI). All trading logic in Rust; TypeScript is display only.

**Architecture:**
- `src/main.rs` — Tauri entry point; spawns 3 background threads (live poller, copy poller, MM cycle)
- `src/app_state.rs` — AppState (Arc-wrapped domain state for Tauri managed state)
- `src/commands/` — IPC bridge (one file per domain, thin wrappers over domain logic)
- `src/commands/dto/` — Serializable DTO structs for Tauri IPC (`#[derive(Serialize, Clone)]`)
- `src/config/` — Environment and configuration loading
- `src/pm/` — Polymarket client wrapper (CLOB, Gamma, Data, WebSocket)
- `src/strategies/` — Domain-specific strategies (crypto, sports, weather)
  - Each has `data/` (external feeds) and `backtest/` subdirectories
- `src/discover/` — Market discovery and search
- `src/copy_trading/` — Copy trader monitoring and execution
- `src/live/` — Live trading execution
- `src/shared/` — Shared utilities and types
- `ui/` — React 18 + ShadCN + Tailwind + ECharts frontend (display only; no trading logic)

**Key Design Patterns:**
- Domain code isolated under `src/strategies/<domain>/`
- Pluggable strategies and data sources via traits
- Paper vs Live execution mode via `EXECUTION_MODE` env var
- All credentials and config via `.env` file

## Development Commands

All commands are in the `Makefile`. Run `make help` to list them. Never run raw cargo commands — use `make` targets instead.

| Target | Description |
|---|---|
| `make build` | Debug build |
| `make build-release` | Release build |
| `make run` | Run debug |
| `make run-release` | Run release |
| `make check` | Type-check (no codegen) |
| `make fmt` | Format code |
| `make fmt-check` | Check formatting (CI) |
| `make lint` | Clippy, deny warnings |
| `make test` | Run all tests |
| `make test-v` | Run tests with output |
| `make test-live` | Run ignored (live) tests |
| `make coverage` | HTML coverage report |
| `make ci` | Full CI pipeline locally (fmt-check + lint + test) |
| `make ci-full` | Full CI + frontend (Rust + ui-test) |
| `make ui-install` | Install frontend deps (bun) |
| `make ui-dev` | Start Vite dev server |
| `make ui-build` | Build frontend for production |
| `make ui-test` | Run Vitest frontend tests |
| `make dev` | `cargo tauri dev` (full app) |
| `make build-tauri` | `cargo tauri build` (release) |
| `make creds` | Derive Polymarket API credentials |

## Environment Setup

See [docs/setup/CREDENTIALS.md](docs/setup/CREDENTIALS.md) for required credentials:
- `PRIVATE_KEY` — Ethereum wallet private key
- `FUNDER_ADDRESS` — Polymarket proxy (Safe) address
- `EXECUTION_MODE` — `paper` (default) or `live`
- Optional: `BINANCE_API_KEY`, `COPY_TRADERS_FILE`

## Key Documentation

- **[docs/standards/STANDARDS.md](docs/standards/STANDARDS.md)** — Full Rust standards and practices
- **[docs/setup/DATA_AND_CREDENTIALS.md](docs/setup/DATA_AND_CREDENTIALS.md)** — Detailed credential setup
- **[docs/setup/POLYMARKET_SETUP.md](docs/setup/POLYMARKET_SETUP.md)** — Polymarket integration details
- **[docs/setup/GETTING_STARTED.md](docs/setup/GETTING_STARTED.md)** — Quick start guide
- **[docs/ai-rules/](docs/ai-rules/)** — AI assistant rules (shared with Cursor)
