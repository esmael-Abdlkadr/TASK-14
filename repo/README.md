# fullstack

CivicSort — offline-first Operations Platform for city sanitation teams. Standardizes waste-sorting guidance, inspections, and performance oversight from a single web application.

## Architecture

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Frontend | Yew (Rust → WebAssembly) | Single-page application with role-aware navigation |
| Backend | Actix-web (Rust) | REST API with auth, audit, rate limiting |
| Database | PostgreSQL 16 | Users, KB, inspections, reviews, messaging, bulk data |
| File Storage | Local disk | Images, exported payloads (no network dependency) |

## User Roles

- **Field Inspector** – Search KB, complete inspections, view reminders
- **Reviewer** – Score submissions and disputed classifications via scorecards
- **Operations Admin** – System config, campaigns, templates, messaging, bulk data
- **Department Manager** – KPI dashboards, team oversight, merge approvals

## Quick Start (Docker — required for evaluation)

Runtime and builds are expected to run via Docker Compose. From this directory, the supported start sequence includes the literal phrase **`docker-compose up`** (flags may follow, e.g. `--build -d`):

```bash
docker-compose up --build -d
```

If your Docker installation exposes Compose only as a plugin, use the same subcommand and flags with that entrypoint (keep service definitions in `docker-compose.yml` unchanged).

| Service | URL | Purpose |
|---------|-----|---------|
| Frontend | http://127.0.0.1:3000 | Web application |
| Backend API | http://127.0.0.1:8080 | REST API (also proxied via :3000/api/) |
| PostgreSQL | internal only (`db:5432` in Compose) | Not published to host by default (avoids port conflicts). Optional host access: `docker-compose -f docker-compose.yml -f docker-compose.db-host.yml up -d` → `localhost:15432` |

Migrations run automatically on backend startup.

## Verify the system works (after `docker-compose up`)

Use this checklist on a fresh stack (empty DB volume). Equivalent requests work in Postman or any HTTP client.

1. **API health (expect HTTP 200 and JSON `"status":"healthy"`):**
   ```bash
   curl -sS http://127.0.0.1:8080/api/health
   ```
2. **Bootstrap admin (empty DB only; no `Authorization` header):** JSON uses PascalCase enum values (`OperationsAdmin`, not snake_case).
   ```bash
   curl -sS -X POST http://127.0.0.1:8080/api/auth/register \
     -H 'Content-Type: application/json' \
     -d '{"username":"eval_admin","password":"EvalPass1!demo","role":"OperationsAdmin"}'
   ```
3. **Session:** expect `session_token` in the body.
   ```bash
   curl -sS -X POST http://127.0.0.1:8080/api/auth/login \
     -H 'Content-Type: application/json' \
     -d '{"username":"eval_admin","password":"EvalPass1!demo"}'
   ```
4. **UI:** open http://127.0.0.1:3000 — log in with the same username/password; the frontend proxies `/api/` to the backend.
5. **Role sanity:** after logging in as `eval_admin`, use the admin UI or authenticated `POST /api/auth/register` to create the other example accounts below (not pre-seeded in the database).

## Demo / evaluation credentials (examples; authentication required)

**Authentication is required** for the application: there is **no** “no authentication required” / anonymous mode for normal use. There are **no pre-seeded users**. Use the table for **example** accounts to create on a fresh volume: row 1 via bootstrap `register`; rows 2–4 via Operations Admin (`Authorization: Bearer <token>` from `eval_admin`).

| Role | Example username | Example password | How it is created |
|------|------------------|------------------|-------------------|
| Operations Admin | `eval_admin` | `EvalPass1!demo` | Bootstrap `POST /api/auth/register` when DB is empty |
| Field Inspector | `eval_inspector` | `EvalPass1!demo2` | Admin `POST /api/auth/register` with `"role":"FieldInspector"` |
| Reviewer | `eval_reviewer` | `EvalPass1!demo3` | Admin register with `"role":"Reviewer"` |
| Department Manager | `eval_manager` | `EvalPass1!demo4` | Admin register with `"role":"DepartmentManager"` |

**Password rules:** minimum 12 characters with upper, lower, digit, and special character (see API validation). Each example password above satisfies them.

For API-only checks after bootstrap: `register` (admin) → `login` → call protected routes with `Authorization: Bearer <session_token>`.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | (required) | PostgreSQL connection string |
| `BIND_ADDRESS` | `0.0.0.0:8080` | Backend listen address |
| `RUST_LOG` | `info` | Log level |
| `CIVICSORT_MASTER_KEY` | (required for encryption) | 64-char hex AES-256 master key |
| `CIVICSORT_IMAGE_DIR` | `./data/images` | Local image storage path |
| `CIVICSORT_EXPORT_DIR` | `./data/exports` | Exported payload file path |
| `CIVICSORT_BOOTSTRAP_ADMIN` | `0` | Set to `1` to allow first-user registration without auth (empty DB only) |

Generate a master key: `openssl rand -hex 32`

**First-time setup (Docker-only):** Use `docker-compose up` from this repo; bootstrap registration is already enabled in `docker-compose.yml` for the empty-DB case. Do not use host-side/manual runtime setup for evaluation.

## Database Migrations

Migrations are in `migrations/` and run automatically on startup:

| # | File | Content |
|---|------|---------|
| 001 | `001_initial_schema.sql` | Users, sessions, auth, audit log, encryption keys |
| 002 | `002_knowledge_base.sql` | KB entries, versions, aliases, images, fuzzy search |
| 003 | `003_inspection_tasks.sql` | Task templates, schedules, instances, submissions, reminders |
| 004 | `004_review_scorecards.sql` | Scorecards, dimensions, reviews, COI, assignments |
| 005 | `005_admin_dashboard.sql` | Campaigns, tags, KPI snapshots, report configs |
| 006 | `006_messaging.sql` | Notification templates, triggers, payloads, delivery tracking |
| 007 | `007_bulk_data.sql` | Import jobs, change history, fingerprints, duplicates, merges |
| 008 | `008_disputed_classifications.sql` | Classification dispute workflow |
| 009 | `009_encrypted_fields.sql` | Encrypted columns for sensitive data |

## Running Tests

All test execution goes through the repo script (starts the stack unless you opt out):

```bash
# Full suite: docker-compose build/up, wait for health, unit + all HTTP integration tests
./run_tests.sh

# Unit tests only (no API; no stack required)
./run_tests.sh unit

# Integration tests only (needs API; script starts stack unless SKIP_DOCKER=1)
docker-compose up -d
./run_tests.sh api
```

### Test structure and coverage

- **Backend library / doctests:** `cargo test --lib`, `cargo test --doc` (see `./run_tests.sh`).
- **HTTP API (no mocks):** `tests/http_api.rs` drives `tests/api_surface.rs`, which issues **113** real `reqwest` calls—one per route in `tests/route_catalog.rs` (full REST surface). Helpers live in `tests/common/mod.rs`.
- **Frontend (Yew):** `frontend/tests/smoke.spec.rs` runs with `cargo test` in `frontend/` (native smoke). The WASM UI is still primarily verified via Docker build + the manual/UI checklist above; deeper browser or `wasm-bindgen-test` coverage is optional.

```
tests/
  http_api.rs       Integration test binary root
  api_surface.rs    One hit per Actix route (canonical count = route_catalog)
  route_catalog.rs  METHOD + path list + count guard
  health.rs         Health / 404 / session smoke tests
  common/mod.rs     reqwest client; CIVICSORT_API_URL (default http://127.0.0.1:8080)
run_tests.sh        Single shell entrypoint for CI/local
```

## Security

- **Authentication**: Argon2id password hashing, 12-char minimum with complexity
- **Account lockout**: 5 failed attempts → 15 minute lock
- **Session management**: SHA-256 hashed tokens, 30-minute idle timeout
- **Rate limiting**: 60 requests/minute/user, anti-bot burst detection
- **Step-up verification**: Password re-entry for exports, role changes, rollbacks
- **Encryption at rest**: AES-256-GCM for sensitive fields (notes, device fingerprints)
- **Audit log**: Immutable, hash-chained, append-only with DB triggers
- **Blind review**: Anonymizes submitter identity for assigned reviewers
- **COI enforcement**: Department and declared conflicts block reviewer assignment

## Offline Constraints

- No internet dependency — all operations run locally
- Local authentication only (no external OAuth/SSO)
- Images stored on local disk with SHA-256 deduplication
- External message payloads (SMS/email/push) queued as JSON files for manual transfer
- All exports (CSV/PDF) generated locally
- Report generation works without network connectivity

## API Endpoints

The backend exposes REST APIs under these scopes:

- `/api/auth/*` – Register, login, logout, session, step-up
- `/api/users/*` – User management (admin/manager)
- `/api/kb/*` – Knowledge base search, entries, categories, images
- `/api/inspection/*` – Templates, schedules, tasks, submissions, reminders
- `/api/reviews/*` – Scorecards, assignments, reviews, COI
- `/api/disputes/*` – Classification dispute workflow
- `/api/admin/*` – Dashboard, campaigns, tags, reports
- `/api/messaging/*` – Templates, triggers, notifications, payloads
- `/api/bulk/*` – Import, export, change history, duplicates, merges
- `/api/audit/*` – Audit log query, export, integrity check
- `/api/devices/*` – Device binding and trust
- `/api/health` – Health check

Canonical route list and count (**113**) are in `tests/route_catalog.rs`.
# TASK-14
