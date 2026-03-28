# API V1 Clients Admin Access

## Status
Stable

## Date
2026-03-28

## Summary
Ensure `GET /api/v1/clients` explicitly preserves the documented behavior that
both `crm` and `crm_admin` users may access the endpoint, and cover that
contract in the end-to-end suite.

## Goals
- Verify `crm_admin`-only users can fetch `GET /api/v1/clients`.
- Keep the HTTP contract documentation aligned with the implementation.

## Non-Goals
- Broadening access beyond the roles already documented in `SPEC.md`.
- Changing the endpoint payload shape.

## Acceptance Criteria
- `tests/e2e.rs` asserts that a `crm_admin`-only user receives `200 OK` from
  `GET /api/v1/clients`.
- The route documentation for `GET /api/v1/clients` no longer implies that the
  `crm` role is required when `crm_admin` is also valid.
