# Combined Audit Report: Test Coverage + README

## 1) Test Coverage Audit

### Project Type Detection
- Declared at README top: `fullstack` (`README.md:1`), so no inference fallback used.

### Backend Endpoint Inventory
- Source of truth for resolved `METHOD + PATH`: `tests/route_catalog.rs:17` (`ROUTES`), with count guard at `tests/route_catalog.rs:5` expecting **113**.
- Route mounts/prefixes are configured by Actix scopes in `src/routes/*.rs` and configured in app bootstrap `src/main.rs:56` and `src/main.rs:67`.
- `/api/admin`: 16 endpoints
- `/api/audit`: 3 endpoints
- `/api/auth`: 5 endpoints
- `/api/bulk`: 14 endpoints
- `/api/devices`: 4 endpoints
- `/api/disputes`: 4 endpoints
- `/api/health`: 1 endpoints
- `/api/inspection`: 20 endpoints
- `/api/kb`: 12 endpoints
- `/api/messaging`: 18 endpoints
- `/api/reviews`: 13 endpoints
- `/api/users`: 3 endpoints
- Total resolved backend endpoints: **113**.

### API Test Mapping Table
| Endpoint | Covered | Test type | Test files | Evidence |
|---|---|---|---|---|
| `GET /api/health` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/auth/register` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/auth/login` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/auth/logout` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/auth/stepup` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/auth/session` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/users` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/users/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/users/{id}/role` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/devices` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/devices/bind` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/devices/trust` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `DELETE /api/devices/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/audit` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/audit/export` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/audit/integrity` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/dashboard` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/kpi/trend` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/overview/users` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/overview/items` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/overview/workorders` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/admin/campaigns` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/campaigns` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/campaigns/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/admin/campaigns/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/admin/tags` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/tags` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `DELETE /api/admin/tags/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/admin/categories/{id}/tags` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/admin/reports/generate` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/admin/reports/configs` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/admin/reports/configs` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/bulk/import` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/bulk/import` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/bulk/import/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/bulk/import/{id}/execute` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/bulk/export` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/bulk/changes` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/bulk/changes/{id}/revert` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/bulk/duplicates` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/bulk/duplicates/{id}/resolve` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/bulk/merges` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/bulk/merges` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/bulk/merges/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/bulk/merges/{id}/conflicts/{cid}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/bulk/merges/{id}/review` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/kb/search` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/kb/entries` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/kb/entries/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/kb/entries/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `DELETE /api/kb/entries/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/kb/entries/{id}/versions` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/kb/categories` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/kb/categories` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/kb/images` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/kb/images/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/kb/search-config` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/kb/search-config` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/disputes` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/disputes` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/disputes/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/disputes/{id}/resolve` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/templates` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/inspection/templates` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/inspection/templates/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/inspection/templates/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `DELETE /api/inspection/templates/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/inspection/templates/{id}/subtasks` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/schedules` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/inspection/schedules` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/inspection/tasks` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/inspection/tasks/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/tasks/{id}/start` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/submissions` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/inspection/submissions/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/inspection/submissions/{id}/review` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/inspection/reminders` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/reminders/read-all` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/reminders/{id}/read` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/reminders/{id}/dismiss` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/generate-instances` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/inspection/process-overdue` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/reviews/scorecards` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/reviews/scorecards` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/reviews/scorecards/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/reviews/scorecards/{id}/dimensions` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/reviews/assignments` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/reviews/assignments/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/reviews/assignments/{id}/recuse` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/reviews/assignments/{id}/submit` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/reviews/queue` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/reviews/coi` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/reviews/coi` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `DELETE /api/reviews/coi/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/reviews/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/templates` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/messaging/templates` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/messaging/templates/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `PUT /api/messaging/templates/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `DELETE /api/messaging/templates/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/triggers` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/messaging/triggers` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `DELETE /api/messaging/triggers/{id}` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/fire` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/messaging/notifications` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/notifications/read-all` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/notifications/{id}/read` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/notifications/{id}/dismiss` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/messaging/payloads` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/payloads/export` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/payloads/mark-delivered` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `POST /api/messaging/payloads/mark-failed` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |
| `GET /api/messaging/payloads/{id}/log` | yes | true no-mock HTTP | `tests/api_surface.rs` | `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`) |

### API Test Classification
- **True No-Mock HTTP**
  - `tests/api_surface.rs:24` (`every_api_route_handles_request_over_http`): real `reqwest` calls for full route surface.
  - `tests/health.rs:6`, `tests/health.rs:15`, `tests/health.rs:21`: real HTTP checks (`/api/health`, 404 path, auth session no token).
  - HTTP client helper uses network calls via `reqwest::Client` in `tests/common/mod.rs:12`, request methods at `tests/common/mod.rs:100`, `tests/common/mod.rs:112`, `tests/common/mod.rs:131`, `tests/common/mod.rs:146`.
- **HTTP with Mocking**
  - None detected by static inspection.
- **Non-HTTP (unit/integration without HTTP)**
  - Route catalog consistency: `tests/route_catalog.rs:5`.
  - Backend unit suites across auth/scheduling/review/messaging/images/encryption/dedup under `src/**/*tests.rs`.
  - Frontend placeholder unit test: `frontend/tests/smoke.spec.rs:5`.

### Mock Detection
- No `jest.mock`, `vi.mock`, `sinon.stub`, `mockito`, or `wiremock` usage found in inspected test code.
- Explicit anti-mock statements are present in `tests/http_api.rs:2` and `tests/common/mod.rs:2`.
- No dependency injection override pattern was found in backend integration tests under `tests/*.rs` (static check only).

### Coverage Summary
- Total endpoints: **113**
- Endpoints with HTTP tests: **113**
- Endpoints with TRUE no-mock tests: **113**
- HTTP coverage: **100.0%**
- True API coverage: **100.0%**

### Unit Test Summary
#### Backend Unit Tests
- Test files (evidence):
  - Auth: `src/auth/password_tests.rs`, `src/auth/registration_tests.rs`, `src/auth/lockout_tests.rs`
  - Scheduling: `src/scheduling/validation_tests.rs`, `src/scheduling/engine_tests.rs`, `src/scheduling/lifecycle_tests.rs`
  - Review logic: `src/review/consistency_tests.rs`, `src/review/coi_tests.rs`
  - Messaging logic: `src/messaging/template_engine_tests.rs`, `src/messaging/trigger_tests.rs`, `src/messaging/payload_lifecycle_tests.rs`
  - Storage/encryption/dedup: `src/images/storage_tests.rs`, `src/encryption/encryption_tests.rs`, `src/dedup/entity_resolution_tests.rs`, `src/dedup/fingerprint_tests.rs`, `src/dedup/import_tests.rs`
- Module category coverage:
  - controllers/routes: covered indirectly via HTTP tests (`tests/api_surface.rs:24`)
  - services/domain logic: covered strongly (scheduling/review/messaging/dedup/encryption test suites)
  - repositories/db: **no direct unit tests found** (`src/db/*tests.rs` absent)
  - auth/guards/middleware: auth utilities tested; **middleware tests absent** (`src/middleware/*tests.rs` absent)
- Important backend modules not directly unit tested (by file-level test presence):
  - `src/routes/*.rs` (no dedicated route unit tests)
  - `src/db/*.rs` (repository/data-access unit tests absent)
  - `src/middleware/auth_middleware.rs`, `src/middleware/rate_limit_middleware.rs`, `src/middleware/audit_middleware.rs`
  - `src/risk/*.rs` (no `src/risk/*tests.rs` detected)

#### Frontend Unit Tests (STRICT)
- Frontend test files detected: `frontend/tests/smoke.spec.rs`
- Framework/tool evidence: Rust built-in test attribute `#[test]` only (`frontend/tests/smoke.spec.rs:4`)
- Frontend component/module targeting evidence:
  - **Missing**: no imports of frontend modules/components, no Yew render/mount usage, no component assertions in `frontend/tests/smoke.spec.rs:1` and `frontend/tests/smoke.spec.rs:5`.
- Components/modules covered:
  - **None (no direct frontend module coverage)**
- Important frontend modules not tested:
  - `frontend/src/app.rs`
  - `frontend/src/pages/login_page.rs`, `frontend/src/pages/kb_search_page.rs`, `frontend/src/pages/inspection_page.rs`, `frontend/src/pages/review_page.rs`, `frontend/src/pages/admin_page.rs`, `frontend/src/pages/messaging_page.rs`, `frontend/src/pages/bulk_data_page.rs`
  - `frontend/src/components/*.rs` (task/review/search/notification/payload/reminder UI components)
  - `frontend/src/services/*.rs` API clients
- **Frontend unit tests: MISSING**
- **CRITICAL GAP**: project is `fullstack`, and frontend unit tests are missing/insufficient under strict criteria.

### Cross-Layer Observation
- Backend testing is extensive (endpoint surface + domain unit tests), but frontend test depth is effectively absent.
- Balance verdict: **backend-heavy, frontend-under-tested (critical imbalance for fullstack)**.

### API Observability Check
- Endpoint + method visibility: strong (`surface_route_hit` labels per route in `tests/api_surface.rs`).
- Request input visibility: moderate/strong (explicit request bodies/params in many calls, e.g., `tests/api_surface.rs:98`, `tests/api_surface.rs:317`, `tests/api_surface.rs:439`).
- Response content visibility: mostly weak; most assertions accept broad status bands and JSON-not-HTML (`tests/common/mod.rs:159`).
- Weakness flag: **YES (response semantic assertions are sparse across most endpoints)**.

### Test Quality & Sufficiency
- Success paths: broadly exercised across all endpoints (routing-level and basic auth bootstrapping).
- Failure cases: some present (404, auth session 401, invalid IDs causing 4xx in surface tests).
- Edge/validation/auth depth: partially covered by unit tests and selected flows, but many API checks are shallow status smoke only.
- Assertions quality: mixed; many API tests validate transport/status but not business-state outcomes.
- `run_tests.sh` check:
  - Docker-based path exists and is primary (`run_tests.sh:6`, `run_tests.sh:70`) -> OK.
  - Local dependency path also allowed (`run_tests.sh:130` cargo local execution) -> FLAG.

### End-to-End Expectations
- For fullstack, real FE↔BE E2E coverage is expected.
- No browser/UI-driven FE↔BE E2E test suite detected; only backend HTTP + one frontend placeholder test.
- Partial compensation exists via strong backend API + domain unit tests, but does not satisfy fullstack E2E expectations.

### Tests Check
- Static inspection only; no runtime execution performed.
- Integration HTTP test entrypoint: `tests/http_api.rs`
- Surface coverage guard: `tests/route_catalog.rs:5`, `tests/api_surface.rs:515`

### Test Coverage Score (0-100)
- **90/100**

### Score Rationale
- + Full endpoint-level HTTP coverage with real network path and no explicit mocks.
- + Strong backend domain unit coverage across multiple critical modules.
- - Frontend unit testing is effectively missing (strict criteria) for a fullstack project (critical deduction).
- - API assertions are often shallow (status-focused rather than response/business semantics).
- - No clear FE↔BE E2E suite.

### Key Gaps
- Critical: frontend unit tests missing by strict definition.
- Critical: no FE↔BE E2E coverage for fullstack expectation.
- High: API tests frequently do not assert response payload semantics deeply.
- Medium: middleware, risk, and DB/repository layers lack direct unit tests.

### Confidence & Assumptions
- Confidence: high for endpoint inventory and HTTP coverage mapping.
- Confidence: high for frontend gap verdict (single placeholder test with no component/module use).
- Assumption: `tests/route_catalog.rs` remains synchronized with route registrations as intended by its count guard.

### Test Coverage Verdict
- **PARTIAL PASS**

---

## 2) README Audit

### README Location
- Found at required path: `repo/README.md`.

### Hard Gate Evaluation
- Formatting/readability: **PASS** (`README.md` structured headings/tables).
- Startup instructions (fullstack requires literal `docker-compose up`): **PASS** (`README.md:23` and `README.md:26`).
- Access method (URL + port): **PASS** (`README.md:31`-`README.md:35`).
- Verification method: **PASS** (`README.md:39`-`README.md:60`, explicit curl + UI flow).
- Environment rules strictness (Docker-contained only, avoid non-containerized runtime flows): **PASS**
  - README explicitly enforces Docker-only evaluation flow and states no host-side/manual runtime setup (`README.md:91`).
- Demo credentials when auth exists: **PASS** (`README.md:62`-`README.md:73`, all roles include username/password examples and creation path).

### Engineering Quality
- Tech stack clarity: strong (`README.md:5`-`README.md:12`).
- Architecture and role model: strong (`README.md:14`-`README.md:20`).
- Testing instructions: strong coverage description and commands (`README.md:109`-`README.md:139`).
- Security/roles: strong (`README.md:141`-`README.md:152`).
- Workflow clarity: generally strong; includes practical verification checklist.

### High Priority Issues
- None.

### Medium Priority Issues
- None.

### Low Priority Issues
- Minor ambiguity between `docker-compose` vs `docker compose` phrasing can cause operator confusion, though both are explained.

### Hard Gate Failures
- None.

### README Verdict (PASS / PARTIAL PASS / FAIL)
- **PASS**

---

## Final Verdicts
- Test Coverage Audit Verdict: **PARTIAL PASS**
- README Audit Verdict: **PASS**
