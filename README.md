# pushkind-crm

`pushkind-crm` is the customer relationship management service used by Pushkind
hubs. It centralises client records, manager assignments, and conversation
history while leveraging Actix Web, Diesel, Tera, and the shared
`pushkind-common` crate for authentication, configuration, and reusable UI
helpers. The app delivers browser-based workflows for operations teams and
exposes lightweight APIs for downstream integrations.

## Features

- **Role-scoped client directory** – Hub members with `SERVICE_ACCESS_ROLE` can
  browse clients with pagination and free-text search while respecting manager
  visibility rules.
- **Rich client profiles** – Each profile aggregates core fields, optional custom
  metadata, assigned managers, and a chronological event timeline for comments,
  emails, document links, and other touchpoints.
- **Manager assignment workflow** – Administrators can invite managers, review
  their portfolios, and assign clients directly from the web UI.
- **Bulk client import** – CSV uploads (up to 10 MB) create multiple clients at
  once, mapping extra columns into structured custom fields.
- **Conversation logging & outreach** – Sanitised comments become timeline
  events, and outbound email is queued over ZeroMQ so the Pushkind mailer can
  deliver updates while keeping history intact.
- **Automated email ingestion** – The `check_events` worker listens on ZeroMQ
  channels to record replies and unsubscribe notices as structured client events.
- **JSON client API** – `/api/v1/clients` exposes the filtered client list for
  partner systems that need machine-readable access.

## Architecture at a Glance

The codebase follows a clean, layered structure so that business logic can be
exercised and tested without going through the web framework:

- **Domain (`src/domain`)** – Type-safe models for clients, client events, and
  managers. Builder-style helpers make it easy to construct new payloads while
  capturing timestamps, normalising contact data, and sanitising inputs early.
- **Repository (`src/repository`)** – Traits that describe the persistence
  contract and a Diesel-backed implementation (`DieselRepository`) that speaks to
  a SQLite database. Each module translates between Diesel models and domain
  types and exposes strongly typed query builders.
- **Services (`src/services`)** – Application use-cases that orchestrate domain
  logic, repository traits, and Pushkind authentication helpers. Services return
  `ServiceResult<T>` and map infrastructure errors into well-defined service
  errors.
- **Forms (`src/forms`)** – `serde`/`validator` powered structs that handle
  request payload validation, CSV parsing, and transformation into domain types.
- **Routes (`src/routes`)** – Actix Web handlers that wire HTTP requests into the
  service layer and render Tera templates or redirect with flash messages.
- **Templates (`templates/`)** – Server-rendered UI built with Tera and
  Bootstrap 5, backed by sanitized HTML rendered via `ammonia` when necessary.

Because the repository traits live in `src/repository/mod.rs`, service functions
accept generic parameters that implement those traits. This makes unit tests easy
by swapping in the `mockall`-based fakes from `src/repository/mock.rs`.

## Technology Stack

- Rust 2024 edition
- [Actix Web](https://actix.rs/) with identity, session, and flash message
  middleware
- [Diesel](https://diesel.rs/) ORM with SQLite and connection pooling via r2d2
- [Tera](https://tera.netlify.app/) templates styled with Bootstrap 5.3
- [`pushkind-common`](https://github.com/pushkindt/pushkind-common) shared crate
  for authentication guards, configuration, database helpers, and reusable
  patterns
- Supporting crates: `chrono`, `validator`, `serde`, `ammonia`, `csv`, and
  `thiserror`

## Getting Started

### Prerequisites

- Rust toolchain (install via [rustup](https://www.rust-lang.org/tools/install))
- `diesel-cli` with SQLite support (`cargo install diesel_cli --no-default-features --features sqlite`)
- SQLite 3 installed on your system

### Environment

The web application and companion event worker read configuration from
environment variables. Key values include:

| Variable | Description | Default |
| --- | --- | --- |
| `DATABASE_URL` | SQLite database path used by the Actix server and workers | `app.db` |
| `SECRET_KEY` | 32-byte secret for signing cookies and sessions | generated at runtime |
| `AUTH_SERVICE_URL` | Base URL of the Pushkind authentication service | _required_ |
| `PORT` | HTTP port for the Actix server | `8080` |
| `ADDRESS` | Interface to bind the Actix server | `127.0.0.1` |
| `DOMAIN` | Cookie domain (without protocol) | `localhost` |
| `ZMQ_EMAILER_PUB` | ZeroMQ endpoint for queuing outbound emails | `tcp://127.0.0.1:5557` |
| `ZMQ_EMAILER_SUB` | ZeroMQ endpoint for inbound email events (`check_events`) | `tcp://127.0.0.1:5558` |
| `ZMQ_REPLIER_SUB` | ZeroMQ endpoint for reply/unsubscribe events (`check_events`) | `tcp://127.0.0.1:5560` |
| `ZMQ_CLIENTS_SUB` | ZeroMQ endpoint for client upsert events (`check_events`) | `tcp://127.0.0.1:5562` |

Create a `.env` file if you want these values loaded automatically via
[`dotenvy`](https://crates.io/crates/dotenvy).

### Database

Run the Diesel migrations before starting the server:

```bash
diesel setup
cargo install diesel_cli --no-default-features --features sqlite # only once
diesel migration run
```

A SQLite file will be created at the location given by `DATABASE_URL`.

## Running the Application

Start the HTTP server with:

```bash
cargo run
```

The server listens on `http://127.0.0.1:8080` by default and serves static
assets from `./assets` in addition to the Tera-powered HTML pages. Authentication
and authorization are enforced via the Pushkind auth service and the
`SERVICE_ACCESS_ROLE` constant.

## Quality Gates

The project treats formatting, linting, and tests as required gates before
opening a pull request. Use the following commands locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-features --tests -- -Dwarnings
cargo test --all-features --verbose
cargo build --all-features --verbose
```

Alternatively, the `make check` target will format the codebase, run clippy, and
execute the test suite in one step.

## Testing

Unit tests exercise the service and form layers directly, while integration
tests live under `tests/`. Repository tests rely on Diesel’s query builders and
should avoid raw SQL strings whenever possible. Use the mock repository module to
isolate services from the database when writing new tests.

## Project Principles

- **Domain-driven**: keep business rules in the domain and service layers and
  translate to/from external representations at the boundaries.
- **Explicit errors**: use `thiserror` to define granular error types and convert
  them into `ServiceError`/`RepositoryError` variants instead of relying on
  `anyhow`.
- **No panics in production paths**: avoid `unwrap`/`expect` in request handlers,
  services, and repositories—propagate errors instead.
- **Security aware**: sanitize any user-supplied HTML using `ammonia`, validate
  inputs with `validator`, and always enforce role checks with
  `pushkind_common::routes::check_role`.
- **Testable**: accept traits rather than concrete types in services and prefer
  dependency injection so the mock repositories can be used in tests.

Following these guidelines will help new functionality slot seamlessly into the
existing architecture and keep the service reliable in production.
