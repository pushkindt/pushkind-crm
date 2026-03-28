# Plan: E2E User Story Coverage Expansion

## References
- Feature spec:
  [../specs/features/e2e-user-story-coverage.md](../specs/features/e2e-user-story-coverage.md)
- Source of truth:
  [../SPEC.md](../SPEC.md)

## Objective
Close the most important CRM user-story coverage gaps in `tests/e2e.rs` while
keeping the suite aligned with the existing end-to-end harness and the route and
worker contracts defined in `SPEC.md`.

## Work Items
1. Add route-driven coverage for cross-hub isolation.
2. Add route-driven coverage for admin-only APIs and mutations that are still
   uncovered.
3. Add route-driven coverage for manager assignment replacement and missing
   manager failures.
4. Add route-driven coverage for exact `public_id` filtering and pagination.
5. Add route-driven coverage for client-detail manager aggregation,
   sanitization, and newest-first event ordering.
6. Reuse the real `check_events` processing helpers in `tests/e2e.rs` to cover
   reply, unsubscribe, and task-event ingestion with the Diesel repository.

## Verification
- Run `cargo test --test e2e -- --ignored`.
