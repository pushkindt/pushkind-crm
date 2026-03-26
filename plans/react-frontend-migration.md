# Plan: React Frontend Migration

## References
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)

## Objective
Introduce React for the `pushkind-crm` frontend while preserving the current UI
structure, Bootstrap styling, routes, embedded `pushkind-files` integration,
and backend-owned CRM business logic. The migration remains server-routed,
non-SPA, and converges on:
Vite-built static HTML for React-owned full pages,
specific `/api/v1/...` client data APIs,
structured JSON mutation responses,
and form-owned validation copy for React-owned flows.

## Fixed Implementation Decisions
- Frontend source code WILL live in `frontend/`.
- Production frontend build output WILL live in `assets/dist/`.
- The React toolchain WILL use `npm`, React, TypeScript, and Vite.
- The backend WILL continue to own routing, authentication, authorization,
  validation, redirects, queueing, and persistence.
- The application server WILL continue to serve compiled frontend assets from
  the existing `/assets` path.
- Vite WILL own the static HTML documents for React-owned full-page routes.
- React page initialization WILL fetch typed JSON data from backend endpoints;
  page data WILL NOT remain embedded into server-generated HTML in the target
  state.
- New GET endpoints introduced for React-owned page data WILL be versioned
  under `/api/v1/`.
- Validation copy for React-owned forms WILL live in `src/forms`.
- The top navigation user dropdown WILL align with the reusable auth/files
  pattern and hydrate menu items from the auth menu API without blocking the
  initial CRM page render.
- Tera WILL be used only as a temporary migration wrapper and WILL be removable
  from runtime paths once a page is fully React-owned.
- Regression verification WILL rely on backend contract tests, frontend
  component or integration tests, and targeted manual checks for
  authentication-dependent flows.

## Repository Layout
The implementation SHOULD create and use the following structure:

```text
frontend/
  package.json
  package-lock.json
  tsconfig.json
  vite.config.ts
  src/
    entries/
    components/
    pages/
    styles/
    lib/
assets/
  dist/
src/
  dto/
  routes/
  services/
  forms/
templates/
```

Directory intent:
- `frontend/src/entries/`:
  entrypoints for full-page CRM routes.
- `frontend/src/components/`:
  reusable shell, navbar, user-menu, modal, form, list, and card components.
- `frontend/src/pages/`:
  page-level React components for dashboard, client details, managers, and
  settings.
- `frontend/src/lib/`:
  typed payload readers, API clients, endpoint builders, Bootstrap adapters,
  and cross-service menu helpers.
- `frontend/src/styles/`:
  CSS imports preserving the current Bootstrap-based output.
- `assets/dist/`:
  compiled JavaScript, CSS, static HTML, and manifest output.

## Toolchain And Build Outputs

### Frontend Package Management
- Use `npm` as the package manager.
- Commit `frontend/package-lock.json`.
- Do not introduce `pnpm`, `yarn`, or an alternative JavaScript runtime.

### Build Tool
- Use Vite to build the React frontend.
- Configure Vite to emit compiled assets into `assets/dist/`.
- Configure Vite to emit a manifest file at `assets/dist/manifest.json`.
- Configure explicit entrypoints for the CRM full-page routes that are migrated
  to React.

### Required `package.json` Scripts
The frontend package MUST expose at least these scripts:
- `dev`
- `build`
- `preview`
- `test`
- `lint`
- `typecheck`
- `format`

### Source Control Hygiene
- Add `frontend/node_modules/` to `.gitignore`.
- Add `assets/dist/` to `.gitignore` unless deployment later requires committed
  build artifacts.

## Backend Integration

### Asset Serving
- Keep Actix static serving for `/assets` and ensure it covers `assets/dist/`.

### Built HTML Serving
- Add a backend helper that serves the built Vite HTML entry for each
  React-owned full-page route after authentication and authorization checks.
- Rust MUST stop assembling the full-page HTML document at request time once a
  route has been fully migrated.

### Frontend Helper Alignment
- Add a backend helper for opening built frontend HTML documents aligned with
  the pattern already used in `pushkind-auth` and `pushkind-files`.
- Avoid introducing CRM-specific frontend-loading abstractions unless they are
  clearly reusable.

### Client Data APIs
- Add typed DTOs under `src/dto/` for reusable CRM client data APIs.
- Prefer specific resource-style endpoints under `/api/v1/` over page-shaped
  bootstrap endpoints.
- The initial DTO surface SHOULD cover:
  current-user/session and shell data,
  client-list data,
  client-details data,
  manager-list data,
  manager modal data,
  settings data,
  and any select-option or supporting lookup data needed by React forms.
- Do not expose raw domain types directly to the frontend.

### Structured Mutation Responses
- Introduce auth/files-style JSON mutation response DTOs for React-owned CRM
  interactions.
- Field errors SHOULD use a stable field-addressable shape.
- Form validation copy MUST come from `src/forms`.

### Server-Rendered Shell During Migration
- During migration, the backend MAY render a minimal HTML shell that:
  includes the React entrypoint,
  includes compiled CSS,
  provides the mount node for React.
- Any such shell is transitional only. The target state for a migrated page is
  a Vite-built static HTML document, not a Rust-rendered page shell.

## Frontend Runtime Requirements

### Shared Shell
- Implement a shared React shell for navbar, flash presentation, layout wiring,
  Bootstrap lifecycle integration, and reusable user-menu behavior.
- The shared shell SHOULD align with the reusable dropdown/menu approach
  already established in `pushkind-auth` and `pushkind-files`.

### Bootstrap Integration
- Keep Bootstrap CSS and Bootstrap Icons in the rendered output.
- Preserve Bootstrap JS behavior for dropdowns, modals, popovers, and tooltips.
- Move inline Bootstrap lifecycle code into React-safe helpers under
  `frontend/src/lib/`.

### Data Loading
- React-owned full pages MUST fetch typed JSON data after the static HTML
  document loads.
- The frontend SHOULD use shared API helpers that compose page state from
  narrower resource endpoints.
- Cross-service menu loading from auth MUST happen after the main CRM page data
  is ready so auth slowness does not blank the CRM page.
- React MUST render explicit fatal error states for required data failures.

### Form And Action Handling
- React-owned mutation flows SHOULD use structured JSON request/response
  handling instead of redirect-plus-flash patterns.
- Native form submission MAY remain for interactions that are not yet migrated.
- Dynamic custom-field editing, markdown preview, manager assignment UI, and
  other inline-script behaviors SHOULD move into typed React components.
- The client attachment flow MUST preserve the embedded `pushkind-files`
  browser contract without CRM taking ownership of file-management rules.

## Migration Sequence

### Phase 1: Foundation
Deliverables:
- `frontend/` directory with React, TypeScript, and Vite configured.
- Build output emitted to `assets/dist/`.
- Backend helpers for serving built frontend HTML documents.
- Developer documentation for installing Node and building frontend assets.

Exit criteria:
- `npm run build` succeeds.
- The server can serve one Vite-built frontend document and load its compiled
  assets.

### Phase 2: Shared Shell And Navigation
Deliverables:
- Shared React shell for flashes, navbar layout, and Bootstrap lifecycle
  integration.
- Reusable React user dropdown aligned with auth/files.
- Auth menu hydration after initial page render, with resilient fallback to the
  always-present `Домой` link and logout action.

Exit criteria:
- Shared shell behavior no longer depends on inline JavaScript in
  `templates/base.html`.
- A React-owned navbar/user-menu can render without blocking on auth menu data.

### Phase 3: Full-Page Document Serving And Page Data APIs
Deliverables:
- Vite-managed HTML entries for CRM pages selected for early migration.
- Typed `/api/v1/...` page-data endpoints for shell, dashboard, client, manager
  modal, and settings needs.
- Typed frontend payload readers and API clients.

Exit criteria:
- At least one CRM page can be served from a Vite-built HTML document and
  initialize entirely from typed client data APIs.

### Phase 4: Dashboard Migration
Deliverables:
- React-backed `GET /` dashboard preserving client list, search form, row
  navigation, pagination, and add-client affordances.
- Replacement for dashboard inline JavaScript with React-owned behavior.
- Structured JSON handling for React-owned add-client interactions if that flow
  is migrated in this phase.

Exit criteria:
- The dashboard page is React-rendered with visual and behavioral parity.

### Phase 5: Client Page Migration
Deliverables:
- React-backed `GET /client/{client_id}` preserving client card, events,
  available fields, important fields, custom-field editing, markdown preview,
  and external-service links.
- React replacement for inline dynamic custom-field and event-type logic.
- Attachment modal integration preserved with the embedded files browser.
- Structured JSON handling for React-owned save/comment/attachment flows.

Exit criteria:
- The client details page works end to end through React-owned UI without
  depending on Tera-owned page markup or inline scripts.

### Phase 6: Managers And Settings Migration
Deliverables:
- React-backed `GET /managers` page preserving manager list and assignment
  behavior.
- React replacement for HTMX manager modal loading using typed data APIs.
- React-backed `GET /settings` page preserving important-field and cleanup
  tooling.

Exit criteria:
- HTMX is no longer required for managers/settings interactions that have been
  migrated.

### Phase 7: Legacy Frontend Removal
Deliverables:
- Remove obsolete Tera page templates and fragments no longer used for React
  pages.
- Remove inline scripts and HTMX wiring no longer needed at runtime.
- Remove temporary migration wrappers once all targeted CRM pages are
  React-backed.

Exit criteria:
- No targeted CRM page depends on HTMX, page-specific inline scripts, or
  Tera-owned page markup at runtime.

## Verification Strategy
- Add backend tests for built-HTML route selection, client data DTOs, and
  structured JSON mutation responses.
- Add frontend unit tests for payload parsing, API clients, Bootstrap helpers,
  and local interactive UI behavior.
- Add frontend component or integration tests for dashboard, client page,
  managers, settings, and user-menu behavior.
- Use targeted manual verification for flows coupled to external authentication
  or cross-service integrations.

## Required Commands
- `cargo build --all-features --verbose`
- `cargo test --all-features`
- `cargo clippy --all-features --tests -- -Dwarnings`
- `cargo fmt --all -- --check`
- `cd frontend && npm run typecheck`
- `cd frontend && npm run test`
- `cd frontend && npm run build`
