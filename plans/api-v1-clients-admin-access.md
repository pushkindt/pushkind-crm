# Plan: API V1 Clients Admin Access

## References
- Feature spec:
  [../specs/features/api-v1-clients-admin-access.md](../specs/features/api-v1-clients-admin-access.md)
- Source of truth:
  [../SPEC.md](../SPEC.md)

## Objective
Close the remaining contract gap for `GET /api/v1/clients` by verifying
`crm_admin`-only access end to end and aligning the route documentation with the
actual authorization behavior.

## Work Items
1. Update the route comment for `GET /api/v1/clients`.
2. Extend the admin-only e2e story with a `GET /api/v1/clients` assertion.
3. Run the targeted ignored e2e test.
