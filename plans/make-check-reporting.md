# Plan: Make Check Reporting

## References
- Feature spec:
  [../specs/features/make-check-reporting.md](../specs/features/make-check-reporting.md)

## Objective
Make the repository `check` target behave like a validation command: report
issues and fail fast, rather than mutating the working tree.

## Work Items
1. Update `Makefile` to run `cargo fmt` in `--check` mode.
2. Add short progress labels for each step so failures are attributable at a
   glance.
3. Re-run `make check` to verify the target still succeeds when the tree is
   clean.
