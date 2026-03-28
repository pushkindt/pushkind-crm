# Make Check Reporting

## Status
Stable

## Date
2026-03-28

## Summary
Adjust the repository `make check` workflow so it reports validation issues
instead of silently fixing them during the check run.

## Goals
- Ensure `make check` fails when formatting is out of date.
- Keep the existing lint and test coverage steps intact.
- Make the output identify which validation phase is currently running.

## Non-Goals
- Changing the underlying Rust validation commands beyond check-mode behavior.
- Introducing new CI stages or architecture changes.

## Acceptance Criteria
- `make check` uses `cargo fmt --all -- --check`.
- `make check` still runs clippy, the full test suite, and ignored e2e tests.
- The command output clearly labels each validation phase.
