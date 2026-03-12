# Testing & Quality

When touching any code, you must:

## 1. Keep Tests in Sync

- Add tests for new behaviour.
- Update tests that break due to your changes.
- Delete tests that cover removed code.
- Never leave tests commented out or skipped without justification.
- **New public functions MUST have tests.** No exceptions. If a function is public, it is tested.
- **New behaviour MUST have tests.** A PR that adds a feature without tests is incomplete.
- **Tests MUST NOT be deleted to fix coverage.** Fix the code instead.
- Run `make test` before considering any task complete.
- `make ci` must pass before any commit.

## 2. Lint & Format

Before finishing, run:

```bash
make fmt
make lint
```

All code must be warning-free and formatted to project style.

## 3. Line Coverage

Line coverage must remain at or above the threshold defined in `.github/workflows/ci.yml` (the `--fail-under-lines` value in the `coverage` job). Do not merge changes that would drop coverage below that threshold. `tui/runner.rs` is the only exclusion — see that file for the regex used.

If your change cannot reasonably be covered (e.g. error branches that require hardware faults), document why in a comment next to the code.

## 4. Full CI Gate

Run `make ci` (= `make fmt-check && make lint && make test`) before declaring any task done. This is non-negotiable. A task is not complete until `make ci` exits 0.

## 5. Frontend Tests

All new React components MUST have Vitest + Testing Library tests. Run `make ui-test` before any PR. Frontend line coverage must stay at or above 80% (enforced in CI). New tab components need at minimum: a render test and a key action test (e.g., toggle fires invoke). `make ci-full` (= `make ci && make ui-test`) is the full gate for frontend changes.
