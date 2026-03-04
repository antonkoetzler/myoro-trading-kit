# Testing & Quality

When touching any code, you must:

## 1. Keep Tests in Sync

- Add tests for new behaviour.
- Update tests that break due to your changes.
- Delete tests that cover removed code.
- Never leave tests commented out or skipped without justification.

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
