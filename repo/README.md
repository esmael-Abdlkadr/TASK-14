# CivicSort

Offline-first Operations Platform for city sanitation teams. Standardizes waste-sorting guidance, inspections, and performance oversight from a single web application.

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

## Quick Start (Docker)

```bash
docker compose up --build
```

| Service | URL | Purpose |
|---------|-----|---------|
| Frontend | http://localhost:3000 | Web application |
| Backend API | http://localhost:8080 | REST API (also proxied via :3000/api/) |
| PostgreSQL | localhost:5432 | Database |

Migrations run automatically on backend startup.

## Local Development

### Prerequisites

- Rust 1.82+
- PostgreSQL 16
- `trunk` (for frontend): `cargo install trunk`
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`

### Backend

```bash
cp .env.example .env
# Edit .env with your database URL and master key
cargo run
```

### Frontend

```bash
cd frontend
trunk serve --proxy-backend=http://localhost:8080/api/
```

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

**First-time setup**: Set `CIVICSORT_BOOTSTRAP_ADMIN=1`, start the server, register the first OperationsAdmin user, then unset the variable. All subsequent user creation requires admin authentication.

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

```bash
# All tests (unit + API)
./run_tests.sh

# Unit tests only (no running backend needed)
./run_tests.sh unit

# API tests only (requires running backend)
docker compose up -d
./run_tests.sh api
```

### Test Structure

```
unit_tests/              Unit test runner script
API_tests/               API integration test scripts (curl-based)
  test_01_health.sh      Health check & connectivity
  test_02_auth.sh        Authentication, registration lockdown, session lifecycle
  test_03_users.sh       User management & RBAC
  test_04_kb.sh          Knowledge base CRUD & search
  test_05_inspection.sh  Inspection tasks & scheduling
  test_06_reviews.sh     Review scorecards & COI
  test_07_admin.sh       Admin dashboard & campaigns
  test_08_messaging.sh   Messaging, notifications & auth negatives
  test_09_bulk_data.sh   Bulk import, dedup & auth negatives
  test_10_audit.sh       Audit log & integrity
  test_11_disputes.sh    Disputed classifications & cross-user auth
  test_12_stepup_and_crossobj.sh  Step-up enforcement, cross-object auth, encryption
run_tests.sh             Master test orchestrator
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
