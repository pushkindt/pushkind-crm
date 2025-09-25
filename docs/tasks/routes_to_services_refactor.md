# Routes to Services Refactor Plan

This document outlines the high-level tasks required to move business logic out of
`src/routes` and into a new service layer under `src/services`. The goal is to make
HTTP handlers thin wrappers that delegate to services which encapsulate all
non-trivial processing.

## 1. Establish the Service Layer Infrastructure

1. Create a new `src/services` module with submodules that mirror the existing route
   areas (`api`, `client`, `main`, `managers`).
2. Define shared abstractions that services can use, such as:
   - A common `ServiceResult` type alias returning domain-specific outcomes that can
     be translated into Actix `HttpResponse`s by the routes.
   - Trait definitions for dependencies (e.g., repositories, template rendering,
     email queue sender) so services can be unit tested without Actix.
3. Move any reusable helper logic currently duplicated in routes (e.g., manager
   authorization checks, context building for templates) into dedicated service
   helpers.

## 2. Refactor the `main` Routes

1. Extract the logic in `show_index` to a service method that:
   - Validates the user's role and returns an authorization error when needed.
   - Builds the client query, performs lookups, and assembles the pagination data.
   - Prepares the template context so the handler can render it.
2. Extract the logic in `add_client` to a service method that:
   - Validates the form.
   - Creates `NewClient` values and persists them through the repository.
   - Returns success or failure outcomes including flash message metadata.
3. Extract the logic in `clients_upload` to a service method that:
   - Validates the multipart payload and converts it into domain data.
   - Persists the new clients and reports success/failure.
4. Update the corresponding handlers so they:
   - Parse Actix request extractors.
   - Call the service methods.
   - Convert the service outcomes into `HttpResponse`s and flash messages.

## 3. Refactor the `client` Routes

1. Create service functions to:
   - Load the client detail view with managers, events, and documents.
   - Persist client edits, comments, and attachments.
2. Move cross-cutting behaviors such as:
   - Role checks for manager access.
   - Sanitization and validation of forms.
   - Construction of `NewClientEvent` values and message queue interactions.
3. Adjust the Actix handlers to delegate to these services, keeping the HTTP layer
   responsible only for request extraction and translating service results into
   responses.

## 4. Refactor the `managers` Routes

1. Build service functions that encapsulate:
   - Listing managers with their clients and rendering context data.
   - Adding/assigning managers, including form validation and repository updates.
   - Loading modal data for a single manager.
2. Ensure the services perform role checks and return well-defined outcomes the
   handlers can map to redirects or rendered templates.

## 5. Refactor the `api` Routes

1. Create a service method for the `/api/v1/clients` endpoint that handles role
   validation, query building, and data retrieval.
2. Let the route handler translate the service result into the appropriate JSON
   response and status code.

## 6. Testing and Validation

1. Add unit tests for the new service modules that cover the extracted business
   logic without relying on Actix abstractions.
2. Update integration tests (if any) to ensure the routes still behave as expected
   after delegating to services.
3. Run the repository's standard checks (`cargo fmt`, `cargo clippy`, `cargo test`,
   `cargo build`) to confirm that the refactor does not introduce regressions.

## 7. Incremental Migration Strategy

1. Perform the refactor module by module to keep pull requests reviewable.
2. After each module is migrated:
   - Remove redundant helpers from the routes.
   - Ensure new services are wired into `main.rs` or other initialization code.
3. Once all routes are thin wrappers, document the new service layer conventions so
   future endpoints follow the same pattern.
