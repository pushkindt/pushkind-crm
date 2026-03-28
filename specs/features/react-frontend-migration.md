# React Frontend Migration Preserving Existing CRM UI

## Status
Stable

## Date
2026-03-27

## Summary
Migrate the current Tera-based `pushkind-crm` frontend to React-managed UI
components while preserving the existing Bootstrap styling, route structure,
user-visible copy, modal flows, embedded `pushkind-files` browser integration,
and backend-owned CRM business rules. This migration MUST follow the same
stable pattern already used in `pushkind-auth` and `pushkind-files`:
server-routed pages,
Vite-built static frontend documents for React-owned pages,
typed client data APIs under `/api/v1/`,
and form-owned validation copy for React-owned mutation flows.

`pushkind-crm` MUST NOT become a SPA.

## Problem
The current CRM UI is spread across Tera templates, inline JavaScript, HTMX
partial swaps, and Bootstrap lifecycle code embedded in templates. That keeps
the service operational, but it makes page behavior harder to compose, test,
and reuse as the dashboard, client details page, manager assignment flows, and
settings screens grow more complex.

The current implementation also mixes several UI ownership models:
- server-rendered full pages via Tera
- HTMX-loaded modal content
- inline DOM mutation for dynamic form fields
- embedded cross-service file browser mounting from `pushkind-files`

That fragmentation is the main reason to migrate.

## Goals
- Introduce React as the component model for CRM user-facing pages.
- Preserve the current Bootstrap-based visual design, route URLs, semantics,
  and Russian copy.
- Preserve current backend authorization, repository rules, persistence, and
  service-layer business logic.
- Replace inline JavaScript and HTMX-driven UI flows with React-owned
  components and typed data contracts where those pages are migrated.
- Keep `pushkind-crm` server-routed and non-SPA.
- Align frontend architecture with the migration pattern already established in
  `pushkind-auth` and `pushkind-files`.
- Make the top navigation user dropdown reusable and driven by the auth menu
  API in the same way as the newer React services.

## Non-Goals
- Introducing client-side routing for CRM pages.
- Redesigning the CRM UI, replacing Bootstrap, or changing the visual language.
- Moving validation, authorization, or persistence rules into the browser.
- Replacing the embedded files browser with a CRM-local file manager.
- Replacing the auth/session model with browser token storage.
- Changing TODO or files-service integration semantics beyond what React needs
  to preserve existing behavior.

## Current Baseline
The current frontend surface is implemented in Tera templates:
- `templates/base.html`
- `templates/components/navigation.html`
- `templates/main/index.html`
- `templates/main/add_client_modal.html`
- `templates/client/index.html`
- `templates/client/client_card.html`
- `templates/client/client_events.html`
- `templates/client/save_client_modal.html`
- `templates/client/attachment_modal.html`
- `templates/managers/index.html`
- `templates/managers/modal_body.html`
- `templates/settings/index.html`

Current client behavior is a mix of:
- Bootstrap JS for dropdowns, modals, popovers, and tooltips.
- HTMX for manager modal loading and swap-driven UI updates.
- Inline JavaScript for client-row navigation, custom-field row editing,
  search/filter helpers, markdown preview wiring, and event-type UI logic.
- Embedded cross-service file browser mounting through
  `{{files_service_url}}/assets/filebrowser.js`.

## In Scope
- The authenticated CRM index page at `GET /`.
- The client details page at `GET /client/{client_id}`.
- The managers page at `GET /managers`.
- The settings page at `GET /settings`.
- Shared shell concerns currently handled in `templates/base.html`, including
  flash messages, Bootstrap lifecycle wiring, and navigation.
- CRM interactions currently driven by inline JavaScript or HTMX, including:
  add-client flow,
  manager assignment flow,
  manager modal rendering,
  client save flow,
  comment/event creation flow,
  attachment flow,
  dynamic custom fields,
  markdown preview,
  and client-list navigation/filtering.
- Frontend asset build and delivery needed to run React in production and local
  development.

## Out Of Scope
- Changes to repository logic, Diesel models, or CRM schema beyond UI contract
  support.
- Store auth OTP flows under `/api/v1/store`.
- Replacing `pushkind-files` or `pushkind-todo` as external services.
- Replacing normal browser navigation for cross-page transitions that are
  already adequately server-routed.

## Functional Requirements

### 1. Rendering Model
- The application MUST keep the existing server-owned route model.
- The application MUST NOT introduce client-side routing for `/`,
  `/client/{client_id}`, `/managers`, or `/settings`.
- React MUST be introduced as page-level or island-level components mounted on
  the existing URLs.
- The long-term target MUST be React-owned page markup for CRM pages without
  runtime dependence on Tera-owned frontend markup for those migrated pages.
- Vite-built static frontend documents SHOULD become the target for full-page
  React-owned routes, following the same pattern used in `pushkind-auth` and
  `pushkind-files`.

### 2. Frontend Document Ownership
- The HTML documents for React-owned CRM pages SHOULD be authored in the
  frontend workspace and built by Vite.
- Rust MUST continue to own authentication and authorization checks before
  serving those built documents.
- Page initialization data MUST NOT remain embedded into server-generated HTML
  in the target state.
- During migration, Tera MAY remain only as a temporary wrapper until a page is
  fully React-backed.

### 3. Markup And Style Preservation
- Migrated React components MUST preserve the current Bootstrap-based layout,
  class structure, modal structure, and navigation hierarchy unless a specific
  deviation is documented.
- User-visible Russian copy SHOULD remain unchanged except for bug fixes or
  accessibility improvements.
- Existing Bootstrap Icons usage, popovers, tooltips, and flash presentation
  MUST continue to work.

### 4. Behavioral Parity
- `GET /` MUST continue to render the CRM client list, search form, add-client
  affordance, and pagination behavior.
- `GET /client/{client_id}` MUST continue to render the client profile,
  events timeline, available fields, comment flow, attachment flow, and related
  external-service links.
- `GET /managers` MUST continue to render the managers list, assignment flow,
  and manager detail modal behavior.
- `GET /settings` MUST continue to render important-field configuration and any
  existing cleanup tools.
- Dropdowns, modals, markdown preview, popovers, and tooltips MUST continue to
  work with Bootstrap behavior.
- The embedded `pushkind-files` browser in the attachment modal MUST continue
  to work without CRM taking ownership of file-management logic.

### 5. Client Data API Model
- React-owned page initialization MUST prefer specific client data APIs over
  HTML-embedded bootstrap data or HTMX partial rendering.
- New GET endpoints introduced for React-owned CRM data MUST be versioned under
  `/api/v1/`.
- The target state SHOULD prefer reusable resource-style APIs over page-shaped
  bootstrap endpoints where practical.
- Initial React-owned data SHOULD be composed from typed APIs such as:
  current-user/session and navigation data,
  client-list data,
  client-details data,
  manager-list data,
  manager modal data,
  settings data,
  and reusable select-option datasets.

### 6. Backend Boundary
- Authorization, validation, persistence, queueing, and repository access MUST
  remain in Rust service and repository code.
- Routes MUST expose typed DTOs or page-model payloads to React rather than
  leaking raw domain types directly into the frontend.
- HTMX-driven interactions MUST move to typed JSON or equivalent structured
  responses before a migrated interaction is considered complete.

### 7. Mutation And Validation Semantics
- React-owned mutation flows SHOULD use structured JSON success/error responses
  instead of flash-message-driven redirects or HTML partial swaps.
- Field-level validation errors MUST be addressable so the frontend can render
  them inline.
- Validation copy for React-owned forms MUST be owned by `src/forms`, not by
  route-level string matching.
- Existing redirect-based form posts MAY remain only for interactions that have
  not yet moved under React ownership.

### 8. Shared Navigation And User Menu
- The top navigation MUST remain visually consistent with the current CRM UI.
- The user dropdown SHOULD align with the reusable React dropdown already used
  in `pushkind-auth` and `pushkind-files`.
- CRM-local dropdown items MUST render before items fetched from the auth menu
  API.
- Menu items beyond the always-present `Домой` link SHOULD come from the auth
  menu API.
- Failure to load the auth menu MUST NOT make `pushkind-crm` unavailable; the
  page MUST still render and keep the `Домой` link and logout action.
- The logout action MUST always render as the final dropdown action even if a
  fetched menu payload contains a logout-like entry.

### 9. External Service Integration
- The CRM client page MUST preserve links into `pushkind-todo` where currently
  available.
- The attachment flow MUST preserve embedding of the `pushkind-files` browser
  through its established mount contract.
- Cross-service UI integrations MUST remain resilient to slow or unavailable
  secondary services; they MUST NOT unnecessarily block the initial CRM page
  render.

### 10. Frontend Tooling
- The repository MUST gain a supported frontend toolchain for React and
  TypeScript source code.
- Production builds MUST emit versioned static assets and any required static
  HTML documents that can be served from the existing `/assets` path.
- The server MUST serve the compiled frontend assets directly.
- Local development MUST support efficient frontend iteration without manual
  asset copying.

## Data Requirements
- React pages and islands MUST initialize from typed DTO contracts.
- DTO structs for React-owned data MUST live under `src/dto/` or a clearly
  equivalent backend-facing UI model module.
- DTOs MUST expose only validated, UI-ready data rather than raw domain
  internals.
- The target state SHOULD avoid one-off page bootstrap payloads when the same
  result can be composed from narrower reusable APIs.

## Migration Requirements
- The migration MUST be incremental.
- Shared React shell components SHOULD be introduced first for navigation,
  flashes, modal wrappers, and user-menu behavior.
- The migration SHOULD converge on:
  Vite-built static HTML for full-page React routes,
  specific `/api/v1/...` client data APIs,
  structured JSON mutation responses,
  and form-owned validation messages.
- Tera MAY remain only as a temporary migration wrapper and SHOULD be removed
  from runtime paths once a page is fully React-owned.
- Inline JavaScript and HTMX usage SHOULD be removed only after equivalent
  React behavior is verified.
- Regression verification SHOULD rely on backend contract tests, frontend
  component or integration tests, and targeted manual checks for
  authentication-dependent flows.

## Acceptance Criteria
- The same URLs continue to serve the corresponding CRM pages and actions.
- Visual appearance remains substantially unchanged for navigation, dashboard,
  client details, managers, modals, and settings.
- React-owned pages are served from Vite-built frontend documents after backend
  access checks.
- Page data comes from typed client data APIs rather than HTML-embedded
  bootstrap payloads.
- React-owned mutations return structured success/error responses with
  field-addressable validation errors.
- The reusable user dropdown behaves consistently with `pushkind-auth` and
  `pushkind-files`.
- The reusable user dropdown renders local CRM items before fetched auth-menu
  items and keeps logout last.
- The embedded files browser still works inside the client attachment modal.
- No backend business rule is moved to the client.
- The React frontend builds reproducibly and its assets are served by the
  application runtime.
- Regression coverage exists for backend page-data contracts and critical
  frontend behavior.

## Risks
- CRM has more mixed UI ownership than auth/files because it combines full
  pages, HTMX modal flows, inline script behavior, and embedded cross-service
  widgets.
- Client-page parity can drift during migration because it mixes CRM-owned UI
  with files/todo integrations.
- Initial render can regress if cross-service navigation or menu calls are kept
  on the critical path instead of hydrating after the main CRM data is ready.
