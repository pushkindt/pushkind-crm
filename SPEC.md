# pushkind-crm Specification

## Purpose

`pushkind-crm` is the CRM service used by Pushkind hubs to manage client
records, manager assignments, and client communication history. It provides
browser-based workflows for operations teams and a lightweight JSON API for
integrations.

## Planned Storefront Auth Contract

The feature spec [crm-direct-store-auth.md](/home/matrizaev/pushkind/specs/features/crm-direct-store-auth.md)
defines the target storefront-auth ownership for CRM.

- CRM is the planned owner of storefront auth/session/logout endpoints under `/api/v1/store/{hub_id}`.
- CRM is the planned issuer of the browser `store-session` cookie used by the storefront across CRM and Orders subdomains.
- The planned storefront JWT claim shape is:
  - `sub`: CRM client `public_id` UUID string
  - `hub_id`: hub identifier
  - `name`: client display name
  - `phone`: normalized E.164 phone number
  - `email`: optional client email
  - `exp`: expiration timestamp
- Orders will consume the same JWT contract to resolve or lazily create its internal customer copy.
- This section documents the target contract; the existing CRM implementation has not yet taken over storefront OTP/session behavior.

## Scope

### In scope

- Client directory with pagination and free-text search.
- Client profiles with core fields, custom fields, managers, and event timeline.
- Manager creation and assignment workflows.
- Bulk client import from CSV (up to 10 MB).
- Comment and email event logging, including outbound email queueing.
- Email event ingestion via ZeroMQ workers.
- JSON API for client list consumption.

### Out of scope

- Authentication provider implementation (delegated to `pushkind-common`).
- Email delivery (handled by external mailer over ZeroMQ).
- File storage service (handled by external files service).

## Architecture

The application is structured into layers with strict responsibilities:

- **Domain (`src/domain`)**: Strongly typed models (e.g., `ManagerEmail`,
  `HubId`, `ManagerName`) that enforce validation and normalization.
- **Repository (`src/repository`)**: Traits and Diesel-backed implementations.
  Repositories map between Diesel models and domain types.
- **Services (`src/services`)**: Business logic, orchestration, and validation.
  Services accept repository traits, return `ServiceResult<T>`, and convert
  domain data to DTOs.
- **DTOs (`src/dto`)**: Rendering/serialization-focused structs produced by
  services.
- **Forms (`src/forms`)**: Input validation, CSV parsing, and conversions into
  domain types using `validator` and `ammonia`.
- **Routes (`src/routes`)**: Thin Actix handlers that call services, translate
  errors into flash messages, and render/redirect.
- **Templates (`templates/`)**: Tera templates for HTML rendering.

## Core Workflows

1. **Client browsing**
   - User MUST have `SERVICE_ACCESS_ROLE` (`crm`).
   - The client list MUST be hub-scoped (`user.hub_id`).
   - On the HTML index page:
     - Users with `SERVICE_ADMIN_ROLE` (`crm_admin`) see all hub clients.
     - Users with `SERVICE_MANAGER_ROLE` (`crm_manager`) see only assigned clients.
     - Other users with `SERVICE_ACCESS_ROLE` see an empty list.
   - Search and pagination MAY be applied via query builders.
   - Optional filtering by `public_id` MAY be applied; invalid values MUST return an
     empty list without querying the repository.

2. **Client profile**
   - Aggregates core fields, custom fields, managers, and events.
   - Users with `SERVICE_MANAGER_ROLE` MUST be assigned to the client.
   - Event timeline MUST be ordered by `created_at` descending (newest first); ties are
     unspecified.
   - User-supplied rich-text content MUST be sanitized before storage/display.

3. **Manager assignment**
   - Workflow MUST require `SERVICE_ADMIN_ROLE` (`crm_admin`).
   - Managers MAY be created/updated by `(hub_id, email)` and assigned clients.
   - Assigning clients MUST replace the manager's existing assignments.
   - Missing managers MUST return `NotFound`.

4. **Bulk import**
   - Workflow MUST require `SERVICE_ADMIN_ROLE` (`crm_admin`).
   - CSV MUST be parsed at the boundary; extra columns MUST map into per-client custom
     fields.
   - Import MAY be best-effort: invalid records MAY be skipped; the handler returns
     flash messaging (no summary payload).

5. **Email events**
   - Outbound emails queued over ZeroMQ.
   - Inbound replies/unsubscribes ingested by `check_events` worker.
   - Events are normalized and added to client timeline.
6. **Task events**
   - Task create/update notifications are consumed from `zmq_tasks_sub` by the `check_events`
     worker and recorded as ClientEvents.

## Invariants

- A Client MUST belong to exactly one Hub.
- Client identity fields (email/phone), when present, MUST be unique per Hub.
- Client public IDs, when present, MUST be unique per Hub.
- A Manager MUST belong to exactly one Hub, and manager email MUST be unique per Hub.
- Client-manager assignments MUST NOT cross hub boundaries.
- Custom field keys MUST be unique per Client.
- ClientEvents MUST be append-only and immutable.
- Deleting a Client MUST delete associated `client_manager`, `client_fields`, and
  `client_events` records.

## Authorization Rules

- All access MUST be scoped to the user's Hub; cross-hub access MUST NOT occur.
- React shell bootstrap via `GET /api/v1/iam` MUST allow either `SERVICE_ACCESS_ROLE` (`crm`)
  or `SERVICE_ADMIN_ROLE` (`crm_admin`) so that admin-only users can load admin pages.
- `SERVICE_ACCESS_ROLE` (`crm`) MUST be present for:
  - Dashboard and client-detail pages
  - Dashboard/client page-data APIs
  - `/api/v1/clients`
- `SERVICE_ADMIN_ROLE` (`crm_admin`) MUST be present for:
  - Managers and settings pages
  - Managers/settings page-data APIs
  - Client creation and bulk import
  - Manager administration (create/assign)
  - Important field configuration
- `SERVICE_MANAGER_ROLE` (`crm_manager`) MUST restrict access to assigned clients on the
  client detail and mutation endpoints.
- `/api/v1/clients` currently enforces `SERVICE_ACCESS_ROLE` only and returns all clients
  in the user's Hub.

## Data Model

- **Hub**: top-level tenant boundary; all Clients and Managers MUST be scoped to a Hub.
- **Client**: MUST belong to one Hub; MAY have zero or more Managers; MUST own zero or
  more ClientEvents; MUST contain core contact fields plus optional custom fields; MAY
  include an optional public ID used for external lookup.
- **Manager**: MUST belong to one Hub; MAY manage zero or more Clients; `(hub_id, email)` is
  unique.
- **ClientEvent**: MUST belong to one Client; MUST be immutable after creation; MUST be
  ordered by `created_at` descending with ties left unspecified.
- **Custom fields**: stored as key/value pairs keyed by `(client_id, field)` and MUST be
  unique per client; a denormalized `clients.fields` string MAY be maintained for search.

### ClientEvent event_data JSON

`client_events.event_data` stores a JSON object as text. The following formats are
produced by current writers and should be preserved for compatibility:

- **Comment / Call / Other**: free-form note text.
  - Shape: `{"text": "<message>"}`.
- **Task**: task entry with optional metadata.
  - Shape: `{"public_id": "<task public id>", "text": "<description-or-null>", "subject": "<title>", "track": "<track-or-null>", "priority": "<priority>", "status": "<status>", "assignee": null | {"name": "<name>", "email": "<email>"}}` where `assignee` is either null or fully populated.
- **Email (manual comment)**: comment-driven email entry.
  - Shape: `{"text": "<message>", "subject": "<subject>"}` with `subject` optional.
- **Email (outbound worker)**: ZeroMQ email queue events.
  - Shape: `{"text": "<subject-or-null>"}` where `text` is the email subject (or `null`).
- **DocumentLink**: attachment/link added via UI.
  - Shape: `{"text": "<label>", "url": "<absolute-url>"}`.
- **Reply**: inbound reply from mailer.
  - Shape: `{"subject": "<subject>", "text": "<sanitized-body>"}`.
- **Unsubscribed**: inbound unsubscribe notification.
  - Shape: `{"text": "<reason>"}`.

These schemas are not enforced by the type system; keep any new writers aligned
with the shapes above or update this section when introducing new formats.

## API Surface

### HTML

- React-owned pages served as built static HTML documents from Vite output.
- Bootstrap 5 remains the UI foundation for the current React pages.
- HTML document routes SHOULD perform only the minimal access/existence checks needed before
  serving the built document; page data MUST be fetched through `/api/v1/...` by the React page.
- Initial page state is loaded from typed JSON APIs under `/api/v1/...`.
- Mutating React-owned forms submit to existing POST endpoints and receive JSON
  mutation responses instead of redirect+flash flows.

### JSON

- `GET /api/v1/clients`
  - Returns filtered client list in JSON for integrations.
  - Access controlled by `SERVICE_ACCESS_ROLE`.
  - Query parameters:
    - `public_id`: optional UUID string for exact match filtering.

## HTTP Error Semantics

### `GET /api/v1/clients`

| Condition | Status | Body |
| --- | --- | --- |
| Success | 200 | JSON array of clients |
| Missing/invalid auth or missing `SERVICE_ACCESS_ROLE` | 401 | Empty body |
| Query deserialization failure | 400 | Framework default |
| Other failures | 500 | Empty body |

### HTML endpoints (non-API)

| Condition | Status | Behavior |
| --- | --- | --- |
| Missing/invalid auth | 303 | Redirect to auth service (`next` param) |
| Missing required role | 303 | Redirect to `/na` or `/` depending on handler |
| Built frontend document missing | 503 | Plain-text service unavailable response |
| Other failures | 500 | Empty body or framework default |

### React mutation endpoints

| Condition | Status | Body |
| --- | --- | --- |
| Success | 200/201 | JSON `{ message, redirect_to }` |
| Validation failure | 400 | JSON `{ message, field_errors }` |
| Missing required role | 403 | JSON error envelope |
| Missing resource | 404 | JSON error envelope |
| Conflict | 409 | JSON error envelope |
| Other failures | 500 | JSON error envelope or empty body |

## Error Handling

- Repositories return `RepositoryResult<T>` with `RepositoryError` variants.
- Services return `ServiceResult<T>` with `ServiceError` variants.
- No `unwrap` or `expect` in production paths.
- Missing dependencies map to `RepositoryError::NotFound`.

## Validation & Sanitization

- User input MUST be validated and normalized at the boundary (forms/services), using
  `validator` where applicable and domain value objects for constraints.
- User-supplied rich-text content MUST be sanitized with `ammonia` (e.g., comment bodies
  and inbound reply payloads).
- Domain types MUST enforce invariants so domain data is always trusted.

## Partial Failure Semantics

- Bulk import MAY be partial; clients MUST NOT assume all-or-nothing behavior unless
  explicitly documented by the endpoint response.

## Contributor Notes
Contributor guidance, including testing expectations, lives in
`CONTRIBUTING.md`.

## Operational Requirements

- Configuration via `config/` YAML plus `APP_` environment variables.
- SQLite database managed by Diesel migrations.
- ZeroMQ endpoints for mailer and ingestion workers.
- Authorization enforced via `pushkind_common::routes::ensure_role`.
