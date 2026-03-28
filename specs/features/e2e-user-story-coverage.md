# E2E User Story Coverage Expansion

## Status
Stable

## Date
2026-03-28

## Summary
Expand `tests/e2e.rs` so the CRM end-to-end suite covers the remaining
high-value user stories and edge cases defined in `SPEC.md`, not only the
existing happy-path role stories.

## Goals
- Cover cross-hub isolation for HTML routes, JSON APIs, and destructive admin
  actions.
- Cover admin-only management APIs and mutations that were not exercised by the
  existing suite.
- Cover manager assignment replacement semantics and missing-manager failures.
- Cover exact `public_id` filtering and pagination through the public CRM APIs.
- Cover client-detail expectations for assigned managers, sanitized user
  content, and newest-first event ordering.
- Cover worker-driven client event ingestion for replies, unsubscribes, and
  task notifications.

## Non-Goals
- Replacing unit tests for forms, services, or repository builders.
- End-to-end verification of external mailer delivery itself.
- Browser-level assertions about Bootstrap widgets or visual appearance.

## Acceptance Criteria
- `tests/e2e.rs` contains route-driven scenarios for:
  - cross-hub isolation
  - admin-only management endpoints
  - manager assignment replacement and not-found handling
  - `public_id` filtering and pagination
  - client-detail sanitization and event ordering
- `tests/e2e.rs` contains worker-ingestion scenarios for:
  - reply events
  - unsubscribe events
  - task events
- New assertions use the real HTTP surface or the real Diesel repository used by
  the existing end-to-end harness.
