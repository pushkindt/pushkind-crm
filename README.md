# pushkind-crm

`pushkind-crm` is the customer relationship management service used by Pushkind
hubs. For the authoritative behavior contract (invariants, auth rules, and HTTP
semantics), see `SPEC.md`.

## Technology Stack

- Rust 2024 edition
- [Actix Web](https://actix.rs/) with identity and session middleware
- [Diesel](https://diesel.rs/) ORM with SQLite and connection pooling via r2d2
- React + Vite static frontend documents styled with Bootstrap 5.3
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
- Node.js for the React/Vite frontend.
  Install via `mise`:

```bash
mise use -g node@22
```

### Configuration

Settings are layered via the [`config`](https://crates.io/crates/config) crate in the following order (later entries override earlier ones):

1. `config/default.yaml` (checked in)
2. `config/{APP_ENV}.yaml` where `APP_ENV` defaults to `local`
3. Environment variables prefixed with `APP_` (loaded automatically from a `.env` file via `dotenvy`)

Key settings you may want to override:

| Environment variable | Description | Default |
| --- | --- | --- |
| `APP_APP__SECRET` | Secret used to sign CRM session/storefront cookies | _required_ |
| `APP_APP__DATABASE_URL` | Path to the SQLite database file | `app.db` |
| `APP_SERVER__ADDRESS` | Interface to bind | `127.0.0.1` |
| `APP_SERVER__PORT` | HTTP port | `80` (override to `8079` in local.yaml) |
| `APP_APP__DOMAIN` | Cookie domain (without protocol) | _required_ |
| `APP_APP__ZMQ_EMAILER_PUB` | ZeroMQ PUB endpoint for outgoing email events | `tcp://127.0.0.1:5557` |
| `APP_APP__ZMQ_EMAILER_SUB` | ZeroMQ PUB endpoint for inbound email events | `tcp://127.0.0.1:5558` |
| `APP_APP__ZMQ_CLIENTS_SUB` | ZeroMQ PUB endpoint for inbound client events | `tcp://127.0.0.1:5566` |
| `APP_APP__ZMQ_REPLIER_SUB` | ZeroMQ PUB endpoint for inbound email reply events | `tcp://127.0.0.1:5560` |
| `APP_APP__AUTH_SERVICE_URL` | URL of the Pushkind authentication service | _required_ |
| `APP_APP__TODO_SERVICE_URL` | Base URL of the manager TODO service used for quick links | _required_ |
| `APP_APP__FILES_SERVICE_URL` | Base URL of the file storage service used for uploading files | _required_ |

Switch to the production profile with `APP_ENV=prod` or provide your own
`config/{env}.yaml`. Environment variables always win over YAML values, so a
local `.env` file containing `APP_APP__SECRET=<64-byte key>` (generate with
`openssl rand -base64 64`) and any overrides will take effect without changing
the checked-in config files.

### Database

Run the Diesel migrations before starting the server:

```bash
diesel setup
cargo install diesel_cli --no-default-features --features sqlite # only once
diesel migration run
```

A SQLite file will be created at the location given by `APP_APP__DATABASE_URL`
or the `app.database_url` config value.

## Running the Application

Start the HTTP server with:

```bash
cargo run
```

The server listens on `http://127.0.0.1:80` by default and serves static
assets from `./assets` in addition to the built React HTML documents.

## React Frontend

The frontend workspace lives in `frontend/`. Install dependencies and build the
Vite output with:

```bash
cd frontend
npm install
npm run build
```

That produces the compiled frontend assets and static HTML documents under
`assets/dist/`, which are served by the real CRM routes.

For iterative frontend work during the migration:

```bash
cd frontend
npm run dev
```

## Quality Gates

The project treats formatting, linting, and tests as required gates before
opening a pull request. Use the following commands locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-features --tests -- -Dwarnings
cargo test --all-features --verbose
cargo build --all-features --verbose
cd frontend && npm run typecheck
cd frontend && npm run format:check
cd frontend && npm run test
cd frontend && npm run build
```

Alternatively, the `make check` target will format the codebase, run clippy, and
execute the test suite in one step.

## Testing

Unit tests exercise the service and form layers directly, while integration
tests live under `tests/`. Repository tests rely on Diesel’s query builders and
should avoid raw SQL strings whenever possible. Use the mock repository module to
isolate services from the database when writing new tests.
