# AGENTS.md

This document provides guidance to AI code generators when working in this
repository. Follow these practices so that new code matches the established
architecture and conventions.

## Project Context

`pushkind-crm` is a Rust 2024 Actix Web application that uses Diesel with
SQLite, Tera templates, and the shared `pushkind-common` crate. The codebase is
layered into domain models, repository traits and implementations, service
modules, DTOs, Actix routes, forms, and templates. Business logic belongs entirely in
the service layer; handlers and repositories must stay thin and focused on I/O
concerns.

## Development Commands

Use these commands to verify your changes before committing:

**Build**
```bash
cargo build --all-features --verbose
```

**Run Tests**
```bash
cargo test --all-features --verbose
```

**Lint (Clippy)**
```bash
cargo clippy --all-features --tests -- -Dwarnings
```

**Format**
```bash
cargo fmt --all -- --check
```

## Coding Standards

- Use idiomatic Rust; avoid `unwrap` and `expect` in production paths.
- Keep modules focused: domain types in `src/domain`, Diesel models in
  `src/models`, DTOs in `src/dto`, and conversions implemented via `From`/`Into`.
- Domain structs should expose strongly typed fields (e.g., `ManagerEmail`,
  `HubId`, `ManagerName`) that encode validation constraints and normalization.
  Construct these types at the boundaries (forms/services) so domain data is
  always trusted and cannot represent invalid input.
- Define error enums with `thiserror` inside the crate that owns the failure and
  return `RepositoryResult<T>` / `ServiceResult<T>` from repository and service
  functions.
- Services should return DTO-level structs when handing data to routes; perform
  domain-to-DTO conversion inside the service layer to keep handlers thin. DTOs
  live in `src/dto` and are optimized for template rendering or JSON serialization.
- Service functions should accept trait bounds (e.g., `ClientReader + ClientWriter`)
  so the `DieselRepository` and `mockall`-powered fakes remain interchangeable.
- Return domain structs or `()` from services; leave flash messaging and
  redirect selection to the HTTP layer.
- Push all branching, validation, and orchestration into services; routes exist
  only to call a service, translate its data or errors into flash messages, and
  redirect.
- Sanitize and validate user input early using `validator` and `ammonia` helpers
  from the form layer.
- Prefer dependency injection through function parameters over global state.
- For Diesel update models, avoid nested optionals; prefer single-layer `Option<T>`
  fields and rely on `#[diesel(treat_none_as_null = true)]` when nullable columns
  need to be cleared.
- Document all public APIs and any breaking changes.

## Database Guidelines

- Use Diesel’s query builder APIs with the generated `schema.rs` definitions; do
  not write raw SQL.
- Translate between Diesel structs (`src/models`) and domain types inside the
  repository layer using explicit `From` implementations.
- Reuse the filtering builders in `ClientListQuery`/`ClientEventListQuery` when
  adding new queries and extend those structs rather than duplicating logic.
- Check related records (e.g., users) before inserts or updates and convert
  missing dependencies into `RepositoryError::NotFound` instead of panicking.

## HTTP and Template Guidelines

- Keep Actix handlers in `src/routes` as thin wrappers that extract inputs,
  invoke a service, then render or redirect; no business logic belongs in the
  route layer.
- Keep services free of HTTP presentation concerns; handlers are responsible
  for flash messaging and redirects.
- Render templates with Tera contexts that only expose sanitized data. Use the
  existing component templates under `templates/` for shared UI.
- Respect the authorization checks via `pushkind_common::routes::ensure_role` and
  the `SERVICE_ACCESS_ROLE` constant.

## Testing Expectations

- Add unit tests for new service and form logic. When hitting the database, use
  Diesel migrations and helper constructors rather than hard-coded SQL strings.
- Use the mock repository module (`src/repository/mock.rs`) to isolate service
  tests from Diesel.
- Ensure new functionality is covered by tests before opening a pull request.

By following these principles the generated code will align with the project’s
architecture, technology stack, and long-term maintainability goals.
