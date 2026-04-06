# CivicSort Operations Platform -- Design Document

| Field          | Value                                |
|----------------|--------------------------------------|
| Version        | 1.0                                  |
| Last Updated   | 2026-04-06                           |
| Status         | Draft                                |
| Platform       | Fully Offline, On-Premises           |
| Domain         | City Sanitation Operations           |

---

## Table of Contents

1. [Introduction & Scope](#1-introduction--scope)
2. [System Architecture Overview](#2-system-architecture-overview)
3. [Technology Stack](#3-technology-stack)
4. [Database Design](#4-database-design)
5. [Authentication & Session Management](#5-authentication--session-management)
6. [Authorization & Role Permissions](#6-authorization--role-permissions)
7. [Core Module Designs](#7-core-module-designs)
8. [Messaging & Notification Center](#8-messaging--notification-center)
9. [Risk Control & Device Binding](#9-risk-control--device-binding)
10. [Audit & Compliance](#10-audit--compliance)
11. [File Storage Strategy](#11-file-storage-strategy)
12. [Security Measures](#12-security-measures)
13. [Data Model Conventions](#13-data-model-conventions)
14. [Deployment](#14-deployment)

---

## 1. Introduction & Scope

### 1.1 Purpose

CivicSort is a fully offline Operations Platform designed for city sanitation teams. It provides waste-sorting guidance, inspection management, performance review workflows, and operational oversight capabilities. The system operates entirely on local infrastructure with no dependency on external internet connectivity.

### 1.2 Problem Statement

City sanitation departments require a reliable, always-available system to manage the complexities of waste-sorting compliance, field inspections, and staff performance evaluation. Internet connectivity in municipal facilities and field locations is often unreliable or restricted by policy. CivicSort addresses this by delivering all functionality through an air-gapped, on-premises deployment.

### 1.3 Scope

The platform covers the following operational areas:

- **Waste-Sorting Knowledge Base** — searchable reference of disposal rules, categories, and guidance materials with fuzzy matching and regional rule versioning.
- **Inspection Task Engine** — template-driven scheduling, assignment, and tracking of field inspections with fault tolerance and make-up logic.
- **Review Workspace** — structured performance evaluation using configurable scorecards, blind review, and conflict-of-interest safeguards.
- **Admin Console** — centralized management of users, items, work orders, KPI dashboards, education campaigns, and audit-ready report exports.
- **Messaging & Notification Center** — offline notification pipeline with template rendering, trigger rules, and queued payloads for manual external delivery.
- **Authentication & Risk Control** — local credential management, device binding, rate limiting, and step-up verification.
- **Audit Trail** — immutable, exportable log of all permissioned actions.

### 1.4 Out of Scope

- Real-time internet-based integrations (cloud sync, third-party APIs).
- Mobile-native applications (the Yew WASM client runs in any modern browser on the local network).
- Automated SMS/email/push delivery (payloads are generated as files for manual transfer to connected systems).

---

## 2. System Architecture Overview

### 2.1 Component Diagram Description

The CivicSort platform consists of four primary components arranged in a single-server, multi-client topology:

```
+-------------------------------------------------------+
|                   Client Devices                       |
|  (Browsers on desktops, tablets, field laptops)        |
|                                                        |
|   +------------------------------------------------+  |
|   |          Yew (Rust/WASM) Frontend              |  |
|   |  - SPA compiled to WebAssembly                 |  |
|   |  - Served as static assets by Actix-web        |  |
|   |  - Client-side routing, form validation        |  |
|   |  - Offline-capable after initial load           |  |
|   +------------------------------------------------+  |
+------------------------------|-------------------------+
                               | HTTP (LAN only)
+------------------------------|-------------------------+
|                    On-Premises Server                  |
|                                                        |
|   +------------------------------------------------+  |
|   |         Actix-web (Rust) API Server            |  |
|   |  - REST API endpoints                          |  |
|   |  - Session management                          |  |
|   |  - Business logic & validation                 |  |
|   |  - Template rendering engine                   |  |
|   |  - Rate limiting & risk control middleware     |  |
|   |  - Audit logging middleware                    |  |
|   +------------------------------------------------+  |
|          |                          |                  |
|   +------v--------+     +----------v-----------+      |
|   |  PostgreSQL   |     |   Local File System  |      |
|   |  Database     |     |   (Disk Storage)     |      |
|   |               |     |                      |      |
|   | - All domain  |     | - Uploaded images    |      |
|   |   data        |     | - Export files       |      |
|   | - Audit logs  |     | - Notification       |      |
|   | - Sessions    |     |   payload files      |      |
|   +--------------+      +----------------------+      |
+-------------------------------------------------------+
```

### 2.2 Communication Flow

1. The Yew WASM client is compiled to static assets (HTML, JS, WASM binary) and served by the Actix-web server over the local network.
2. All client-server communication uses standard HTTP REST calls over LAN. No external network egress occurs.
3. The Actix-web server manages all business logic, persists data to PostgreSQL, and reads/writes files to local disk.
4. PostgreSQL handles relational data, full-text and trigram search indexes, and transactional integrity.
5. The local file system stores binary assets (images), generated export files (CSV/PDF), and notification payload files queued for manual transfer.

### 2.3 Network Topology

- The server and all clients reside on a single isolated LAN segment.
- No internet gateway is required or expected.
- TLS may be configured using a self-signed certificate authority managed by the IT department for LAN encryption.

---

## 3. Technology Stack

| Layer            | Technology              | Version / Notes                                   |
|------------------|-------------------------|---------------------------------------------------|
| Frontend         | Yew (Rust to WASM)      | Compiled to WebAssembly; SPA architecture          |
| Build Tooling    | Trunk                   | Builds and bundles the Yew application             |
| Backend          | Actix-web (Rust)        | High-performance async HTTP server                 |
| ORM / DB Access  | Diesel or SQLx          | Compile-time checked SQL queries                   |
| Database         | PostgreSQL              | 15+ with pg_trgm extension for fuzzy search        |
| File Storage     | Local disk              | Structured directory layout under a configured root |
| PDF Generation   | printpdf or wkhtmltopdf | Server-side PDF rendering for exports              |
| CSV Generation   | csv crate (Rust)        | Streaming CSV serialization                        |
| Password Hashing | argon2 crate (Rust)     | Argon2id variant                                   |
| Encryption       | aes-gcm crate (Rust)    | AES-256-GCM for encryption at rest                 |
| Session Tokens   | CSPRNG + HMAC-SHA256    | Opaque session identifiers                         |
| Search           | PostgreSQL pg_trgm      | Trigram-based fuzzy matching                        |

### 3.1 Rationale

- **Rust end-to-end** eliminates context-switching between languages, shares types/validation between client and server, and provides memory safety guarantees without a garbage collector.
- **Yew/WASM** delivers near-native performance in the browser, compiles from the same Rust codebase, and loads entirely from local assets after the initial page fetch.
- **Actix-web** is one of the highest-throughput Rust web frameworks, well-suited for handling concurrent inspection submissions from field devices.
- **PostgreSQL** provides mature support for trigram indexes, JSONB columns for flexible metadata, and robust transactional guarantees required by the audit trail.
- **Local disk storage** avoids object-store dependencies and keeps the system fully self-contained.

---

## 4. Database Design

### 4.1 Entity-Relationship Summary

The database is organized into the following entity groups:

- **Identity & Access**: users, roles, device_bindings
- **Knowledge Base**: knowledge_base_entries, kb_versions, regions
- **Inspections**: task_templates, subtasks, inspection_submissions
- **Reviews**: review_assignments, scorecards, scorecard_dimensions
- **Messaging**: notifications, notification_templates, campaigns
- **System**: audit_log, files

### 4.2 Entity Definitions

#### 4.2.1 users

| Column              | Type         | Constraints                        | Description                              |
|---------------------|--------------|------------------------------------|------------------------------------------|
| id                  | UUID         | PK                                 | Unique user identifier                   |
| username            | VARCHAR(100) | UNIQUE, NOT NULL                   | Login name                               |
| display_name        | VARCHAR(200) | NOT NULL                           | Full name for display                    |
| email               | VARCHAR(255) | NULLABLE                           | Email (masked in UI, encrypted at rest)  |
| phone               | VARCHAR(30)  | NULLABLE                           | Phone (masked in UI, encrypted at rest)  |
| password_hash       | TEXT         | NOT NULL                           | Argon2id hash                            |
| role_id             | UUID         | FK -> roles.id, NOT NULL           | Assigned role                            |
| failed_login_count  | INT          | DEFAULT 0                          | Consecutive failed login attempts        |
| locked_until        | TIMESTAMPTZ  | NULLABLE                           | Account lock expiry timestamp            |
| last_login_at       | TIMESTAMPTZ  | NULLABLE                           | Timestamp of most recent login           |
| is_active           | BOOLEAN      | DEFAULT true                       | Soft-disable flag                        |
| created_at          | TIMESTAMPTZ  | NOT NULL, DEFAULT now()            | Record creation timestamp                |
| updated_at          | TIMESTAMPTZ  | NOT NULL, DEFAULT now()            | Last modification timestamp              |
| deleted_at          | TIMESTAMPTZ  | NULLABLE                           | Soft delete timestamp                    |

#### 4.2.2 roles

| Column      | Type         | Constraints            | Description                          |
|-------------|--------------|------------------------|--------------------------------------|
| id          | UUID         | PK                     | Unique role identifier               |
| name        | VARCHAR(50)  | UNIQUE, NOT NULL       | Role name (e.g., field_inspector)    |
| description | TEXT         | NULLABLE               | Human-readable role description      |
| permissions | JSONB        | NOT NULL               | Structured permission set            |
| created_at  | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Record creation timestamp           |
| updated_at  | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Last modification timestamp         |

#### 4.2.3 device_bindings

| Column       | Type         | Constraints                       | Description                            |
|--------------|--------------|-----------------------------------|----------------------------------------|
| id           | UUID         | PK                                | Unique binding identifier              |
| user_id      | UUID         | FK -> users.id, NOT NULL          | Bound user                             |
| device_fingerprint | VARCHAR(512) | NOT NULL                   | Browser/device fingerprint hash        |
| device_label | VARCHAR(200) | NULLABLE                          | Human-readable device name             |
| is_active    | BOOLEAN      | DEFAULT true                      | Whether binding is currently valid      |
| bound_at     | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | When the binding was created           |
| last_seen_at | TIMESTAMPTZ  | NULLABLE                          | Last successful auth from this device  |
| created_at   | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | Record creation timestamp              |

**Constraint**: A composite unique index on (user_id, device_fingerprint) prevents duplicate bindings.

#### 4.2.4 knowledge_base_entries

| Column            | Type         | Constraints                       | Description                               |
|-------------------|--------------|-----------------------------------|-------------------------------------------|
| id                | UUID         | PK                                | Unique entry identifier                   |
| item_name         | VARCHAR(300) | NOT NULL                          | Primary item name                         |
| aliases           | TEXT[]       | DEFAULT '{}'                      | Array of alternative names/misspellings   |
| category_id       | UUID         | FK -> categories.id, NULLABLE     | Sorting category                          |
| region_id         | UUID         | FK -> regions.id, NOT NULL        | Applicable region                         |
| current_version_id| UUID         | FK -> kb_versions.id, NULLABLE    | Pointer to the active version             |
| search_vector     | TSVECTOR     | Generated column                  | Full-text search index column             |
| search_weight     | REAL         | DEFAULT 1.0                       | Configurable ranking weight               |
| is_published      | BOOLEAN      | DEFAULT false                     | Visibility flag                           |
| created_at        | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | Record creation timestamp                 |
| updated_at        | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | Last modification timestamp               |
| deleted_at        | TIMESTAMPTZ  | NULLABLE                          | Soft delete timestamp                     |

**Indexes**:
- GIN index on `search_vector` for full-text search.
- GIN trigram index on `item_name` using `gin_trgm_ops` for fuzzy matching.
- GIN trigram index on `aliases` (unnested) for alias fuzzy matching.

#### 4.2.5 kb_versions

| Column              | Type         | Constraints                              | Description                              |
|---------------------|--------------|------------------------------------------|------------------------------------------|
| id                  | UUID         | PK                                       | Unique version identifier                |
| entry_id            | UUID         | FK -> knowledge_base_entries.id, NOT NULL | Parent entry                             |
| version_number      | INT          | NOT NULL                                 | Monotonically increasing version         |
| disposal_requirements | TEXT       | NOT NULL                                 | Disposal instructions text               |
| rule_reference      | VARCHAR(200) | NULLABLE                                 | Regulation or rule code reference        |
| rule_version        | VARCHAR(50)  | NULLABLE                                 | Version of the governing rule set        |
| reference_image_ids | UUID[]       | DEFAULT '{}'                             | Array of FK references to files.id       |
| notes               | TEXT         | NULLABLE                                 | Editor notes for this version            |
| created_by          | UUID         | FK -> users.id, NOT NULL                 | Author of this version                   |
| created_at          | TIMESTAMPTZ  | NOT NULL, DEFAULT now()                  | When this version was created            |

**Constraint**: UNIQUE on (entry_id, version_number).

#### 4.2.6 regions

| Column      | Type         | Constraints             | Description                        |
|-------------|--------------|-------------------------|------------------------------------|
| id          | UUID         | PK                      | Unique region identifier           |
| name        | VARCHAR(200) | UNIQUE, NOT NULL        | Region display name                |
| code        | VARCHAR(50)  | UNIQUE, NOT NULL        | Short code (e.g., DIST-01)        |
| description | TEXT         | NULLABLE                | Optional description               |
| is_active   | BOOLEAN      | DEFAULT true            | Whether region is currently in use |
| created_at  | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Record creation timestamp          |
| updated_at  | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Last modification timestamp        |

#### 4.2.7 task_templates

| Column             | Type         | Constraints                       | Description                                  |
|--------------------|--------------|-----------------------------------|----------------------------------------------|
| id                 | UUID         | PK                                | Unique template identifier                   |
| name               | VARCHAR(300) | NOT NULL                          | Template display name                        |
| description        | TEXT         | NULLABLE                          | Template purpose and instructions            |
| region_id          | UUID         | FK -> regions.id, NULLABLE        | Scoped region, NULL = all regions            |
| cycle_type         | VARCHAR(30)  | NOT NULL                          | daily, weekly, biweekly, monthly, one_time   |
| time_window_start  | TIME         | NOT NULL, DEFAULT '08:00'         | Earliest permitted start time                |
| time_window_end    | TIME         | NOT NULL, DEFAULT '18:00'         | Latest permitted end time                    |
| fault_tolerance_missed | INT      | DEFAULT 1                         | Max missed inspections per tolerance window  |
| fault_tolerance_window_days | INT | DEFAULT 30                        | Rolling window for fault tolerance (days)    |
| makeup_deadline_hours | INT       | DEFAULT 48                        | Hours allowed for a make-up inspection       |
| is_active          | BOOLEAN      | DEFAULT true                      | Whether template is available for scheduling |
| created_by         | UUID         | FK -> users.id, NOT NULL          | Template author                              |
| created_at         | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | Record creation timestamp                    |
| updated_at         | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | Last modification timestamp                  |
| deleted_at         | TIMESTAMPTZ  | NULLABLE                          | Soft delete timestamp                        |

#### 4.2.8 subtasks

| Column        | Type         | Constraints                           | Description                            |
|---------------|--------------|---------------------------------------|----------------------------------------|
| id            | UUID         | PK                                    | Unique subtask identifier              |
| template_id   | UUID         | FK -> task_templates.id, NOT NULL     | Parent template                        |
| group_name    | VARCHAR(200) | NOT NULL                              | Logical grouping label                 |
| title         | VARCHAR(300) | NOT NULL                              | Subtask title                          |
| description   | TEXT         | NULLABLE                              | Detailed instructions                  |
| sort_order    | INT          | NOT NULL, DEFAULT 0                   | Display ordering within the group      |
| is_required   | BOOLEAN      | DEFAULT true                          | Whether this subtask must be completed |
| created_at    | TIMESTAMPTZ  | NOT NULL, DEFAULT now()               | Record creation timestamp              |
| updated_at    | TIMESTAMPTZ  | NOT NULL, DEFAULT now()               | Last modification timestamp            |

#### 4.2.9 inspection_submissions

| Column           | Type         | Constraints                           | Description                                 |
|------------------|--------------|---------------------------------------|---------------------------------------------|
| id               | UUID         | PK                                    | Unique submission identifier                |
| template_id      | UUID         | FK -> task_templates.id, NOT NULL     | Template this inspection is based on        |
| inspector_id     | UUID         | FK -> users.id, NOT NULL              | Assigned field inspector                    |
| scheduled_date   | DATE         | NOT NULL                              | Original scheduled date                     |
| scheduled_window_start | TIME   | NOT NULL                              | Start of permitted time window              |
| scheduled_window_end   | TIME   | NOT NULL                              | End of permitted time window                |
| status           | VARCHAR(30)  | NOT NULL, DEFAULT 'scheduled'         | scheduled, in_progress, completed, missed, makeup_pending, makeup_completed, excused |
| is_makeup        | BOOLEAN      | DEFAULT false                         | Whether this is a make-up inspection        |
| original_submission_id | UUID   | FK -> inspection_submissions.id, NULLABLE | Links make-up to original missed inspection |
| makeup_deadline  | TIMESTAMPTZ  | NULLABLE                              | Deadline for completing make-up             |
| started_at       | TIMESTAMPTZ  | NULLABLE                              | When inspector began the inspection         |
| completed_at     | TIMESTAMPTZ  | NULLABLE                              | When inspector submitted results            |
| subtask_results  | JSONB        | DEFAULT '{}'                          | Subtask-level completion data               |
| validation_errors| JSONB        | NULLABLE                              | Any validation feedback from the system     |
| notes            | TEXT         | NULLABLE                              | Inspector free-text notes                   |
| photo_ids        | UUID[]       | DEFAULT '{}'                          | Array of FK references to files.id          |
| created_at       | TIMESTAMPTZ  | NOT NULL, DEFAULT now()               | Record creation timestamp                   |
| updated_at       | TIMESTAMPTZ  | NOT NULL, DEFAULT now()               | Last modification timestamp                 |

#### 4.2.10 review_assignments

| Column              | Type         | Constraints                                  | Description                                  |
|---------------------|--------------|----------------------------------------------|----------------------------------------------|
| id                  | UUID         | PK                                           | Unique assignment identifier                 |
| submission_id       | UUID         | FK -> inspection_submissions.id, NOT NULL     | Inspection being reviewed                    |
| reviewer_id         | UUID         | FK -> users.id, NOT NULL                      | Assigned reviewer                            |
| scorecard_id        | UUID         | FK -> scorecards.id, NOT NULL                 | Scorecard template to use                    |
| assignment_method   | VARCHAR(20)  | NOT NULL                                      | automatic or manual                          |
| is_blind            | BOOLEAN      | DEFAULT true                                  | Whether inspector identity is hidden         |
| status              | VARCHAR(30)  | NOT NULL, DEFAULT 'pending'                   | pending, in_progress, completed, recused     |
| recusal_reason      | TEXT         | NULLABLE                                      | Reason if reviewer recused themselves        |
| dimension_scores    | JSONB        | NULLABLE                                      | Scores per scorecard dimension               |
| overall_score       | REAL         | NULLABLE                                      | Computed weighted overall score              |
| reviewer_comments   | TEXT         | NULLABLE                                      | General review comments                      |
| completed_at        | TIMESTAMPTZ  | NULLABLE                                      | When the review was finalized                |
| created_at          | TIMESTAMPTZ  | NOT NULL, DEFAULT now()                       | Record creation timestamp                    |
| updated_at          | TIMESTAMPTZ  | NOT NULL, DEFAULT now()                       | Last modification timestamp                  |

#### 4.2.11 scorecards

| Column              | Type         | Constraints             | Description                                  |
|---------------------|--------------|-------------------------|----------------------------------------------|
| id                  | UUID         | PK                      | Unique scorecard identifier                  |
| name                | VARCHAR(300) | NOT NULL                | Scorecard display name                       |
| description         | TEXT         | NULLABLE                | Purpose and usage notes                      |
| requires_comments   | BOOLEAN      | DEFAULT true            | Whether reviewer must provide comments       |
| min_rating          | INT          | NOT NULL, DEFAULT 1     | Minimum allowed rating value                 |
| max_rating          | INT          | NOT NULL, DEFAULT 5     | Maximum allowed rating value                 |
| consistency_threshold | REAL       | DEFAULT 0.20            | Max allowed deviation between dimension scores before warning |
| is_active           | BOOLEAN      | DEFAULT true            | Whether scorecard is available for use       |
| created_by          | UUID         | FK -> users.id, NOT NULL| Author                                       |
| created_at          | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Record creation timestamp                    |
| updated_at          | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Last modification timestamp                  |

#### 4.2.12 scorecard_dimensions

| Column        | Type         | Constraints                       | Description                           |
|---------------|--------------|-----------------------------------|---------------------------------------|
| id            | UUID         | PK                                | Unique dimension identifier           |
| scorecard_id  | UUID         | FK -> scorecards.id, NOT NULL     | Parent scorecard                      |
| name          | VARCHAR(200) | NOT NULL                          | Dimension label (e.g., "Thoroughness")|
| description   | TEXT         | NULLABLE                          | What this dimension measures          |
| weight        | REAL         | NOT NULL, DEFAULT 1.0             | Weight in overall score calculation   |
| requires_comment | BOOLEAN   | DEFAULT false                     | Whether a per-dimension comment is required |
| sort_order    | INT          | NOT NULL, DEFAULT 0               | Display ordering                      |
| created_at    | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | Record creation timestamp             |

#### 4.2.13 notifications

| Column          | Type         | Constraints                              | Description                                 |
|-----------------|--------------|------------------------------------------|---------------------------------------------|
| id              | UUID         | PK                                       | Unique notification identifier              |
| recipient_id    | UUID         | FK -> users.id, NOT NULL                 | Target user                                 |
| template_id     | UUID         | FK -> notification_templates.id, NULLABLE| Source template, if any                      |
| channel         | VARCHAR(20)  | NOT NULL                                 | in_app, sms, email, push                    |
| subject         | VARCHAR(500) | NULLABLE                                 | Subject line (email/push)                   |
| body            | TEXT         | NOT NULL                                 | Rendered message body                       |
| variables_used  | JSONB        | NULLABLE                                 | Snapshot of substitution variables           |
| trigger_event   | VARCHAR(100) | NULLABLE                                 | Event that triggered this notification       |
| status          | VARCHAR(20)  | NOT NULL, DEFAULT 'pending'              | pending, delivered, queued_for_transfer, transferred, failed, retrying |
| delivery_attempts | INT        | DEFAULT 0                                | Number of delivery or transfer attempts      |
| last_attempt_at | TIMESTAMPTZ  | NULLABLE                                 | Timestamp of most recent attempt             |
| payload_file_id | UUID         | FK -> files.id, NULLABLE                 | Generated payload file for external channels |
| created_at      | TIMESTAMPTZ  | NOT NULL, DEFAULT now()                  | Record creation timestamp                    |
| updated_at      | TIMESTAMPTZ  | NOT NULL, DEFAULT now()                  | Last modification timestamp                  |

#### 4.2.14 notification_templates

| Column          | Type         | Constraints             | Description                                  |
|-----------------|--------------|-------------------------|----------------------------------------------|
| id              | UUID         | PK                      | Unique template identifier                   |
| name            | VARCHAR(200) | UNIQUE, NOT NULL        | Template name for admin reference            |
| channel         | VARCHAR(20)  | NOT NULL                | Target channel (in_app, sms, email, push)    |
| subject_template| VARCHAR(500) | NULLABLE                | Subject with {{variable}} placeholders       |
| body_template   | TEXT         | NOT NULL                | Body with {{variable}} placeholders          |
| available_variables | TEXT[]   | DEFAULT '{}'            | Documented list of supported variables       |
| trigger_event   | VARCHAR(100) | NULLABLE                | Event this template is bound to              |
| is_active       | BOOLEAN      | DEFAULT true            | Whether template is in use                   |
| created_by      | UUID         | FK -> users.id, NOT NULL| Author                                       |
| created_at      | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Record creation timestamp                    |
| updated_at      | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Last modification timestamp                  |

#### 4.2.15 campaigns

| Column          | Type         | Constraints                       | Description                              |
|-----------------|--------------|-----------------------------------|------------------------------------------|
| id              | UUID         | PK                                | Unique campaign identifier               |
| name            | VARCHAR(300) | NOT NULL                          | Campaign display name                    |
| type            | VARCHAR(30)  | NOT NULL                          | education or promo                       |
| description     | TEXT         | NULLABLE                          | Campaign details                         |
| start_date      | DATE         | NOT NULL                          | Campaign start date                      |
| end_date        | DATE         | NOT NULL                          | Campaign end date                        |
| target_roles    | TEXT[]       | DEFAULT '{}'                      | Roles this campaign targets              |
| target_regions  | UUID[]       | DEFAULT '{}'                      | Regions this campaign targets            |
| content         | JSONB        | NOT NULL                          | Structured campaign content              |
| status          | VARCHAR(20)  | NOT NULL, DEFAULT 'draft'         | draft, active, paused, completed, archived |
| created_by      | UUID         | FK -> users.id, NOT NULL          | Author                                   |
| created_at      | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | Record creation timestamp                |
| updated_at      | TIMESTAMPTZ  | NOT NULL, DEFAULT now()           | Last modification timestamp              |
| deleted_at      | TIMESTAMPTZ  | NULLABLE                          | Soft delete timestamp                    |

**Constraint**: CHECK (end_date >= start_date).

#### 4.2.16 audit_log

| Column         | Type         | Constraints             | Description                                  |
|----------------|--------------|-------------------------|----------------------------------------------|
| id             | UUID         | PK                      | Unique log entry identifier                  |
| actor_id       | UUID         | FK -> users.id, NOT NULL| User who performed the action                |
| action         | VARCHAR(100) | NOT NULL                | Action identifier (e.g., user.create, kb.publish) |
| resource_type  | VARCHAR(100) | NOT NULL                | Entity type affected                         |
| resource_id    | UUID         | NULLABLE                | Specific entity affected                     |
| details        | JSONB        | NULLABLE                | Additional context (before/after snapshots)  |
| ip_address     | VARCHAR(45)  | NULLABLE                | Client IP on local network                   |
| device_fingerprint | VARCHAR(512) | NULLABLE             | Device fingerprint at time of action         |
| created_at     | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Immutable event timestamp                    |

**Design note**: This table is append-only. No UPDATE or DELETE operations are permitted at the application layer. The database role used by the application has INSERT-only privileges on this table. A separate read-only role is used for queries and exports.

#### 4.2.17 files

| Column          | Type         | Constraints             | Description                                 |
|-----------------|--------------|-------------------------|---------------------------------------------|
| id              | UUID         | PK                      | Unique file identifier                      |
| original_name   | VARCHAR(500) | NOT NULL                | Original uploaded filename                  |
| stored_name     | VARCHAR(500) | NOT NULL                | Name on disk (UUID-based)                   |
| mime_type       | VARCHAR(50)  | NOT NULL                | MIME type (image/jpeg or image/png)         |
| file_size_bytes | BIGINT       | NOT NULL                | Size in bytes                               |
| sha256_checksum | VARCHAR(64)  | NOT NULL                | SHA-256 hash of file contents               |
| storage_path    | TEXT         | NOT NULL                | Relative path from storage root             |
| uploaded_by     | UUID         | FK -> users.id, NOT NULL| Uploader                                    |
| is_duplicate    | BOOLEAN      | DEFAULT false           | Whether this is a dedup reference           |
| duplicate_of    | UUID         | FK -> files.id, NULLABLE| Original file if this is a duplicate        |
| created_at      | TIMESTAMPTZ  | NOT NULL, DEFAULT now() | Upload timestamp                            |
| deleted_at      | TIMESTAMPTZ  | NULLABLE                | Soft delete timestamp                       |

**Index**: UNIQUE index on sha256_checksum for fingerprint-based deduplication.

### 4.3 Key Relationships

| Relationship                                    | Cardinality | Description                                          |
|-------------------------------------------------|-------------|------------------------------------------------------|
| users -> roles                                  | Many-to-One | Each user has exactly one role                       |
| users -> device_bindings                        | One-to-Many | A user may be bound to multiple devices              |
| knowledge_base_entries -> kb_versions           | One-to-Many | Each entry has a version history                     |
| knowledge_base_entries -> regions               | Many-to-One | Each entry belongs to one region                     |
| task_templates -> subtasks                      | One-to-Many | Templates contain multiple subtasks                  |
| task_templates -> inspection_submissions        | One-to-Many | A template generates many inspections over time      |
| inspection_submissions -> review_assignments    | One-to-Many | An inspection may be reviewed multiple times         |
| review_assignments -> scorecards                | Many-to-One | Each review uses one scorecard                       |
| scorecards -> scorecard_dimensions              | One-to-Many | A scorecard has multiple weighted dimensions         |
| notification_templates -> notifications         | One-to-Many | A template generates many notifications              |
| inspection_submissions -> inspection_submissions| Self-ref    | Make-up inspections reference the original missed one|

---

## 5. Authentication & Session Management

### 5.1 Credential Storage

- Passwords are hashed using **Argon2id** with the following parameters:
  - Memory cost: 64 MB
  - Time cost: 3 iterations
  - Parallelism: 4 lanes
  - Salt: 16 bytes, generated per-user via CSPRNG
  - Output hash length: 32 bytes
- The full Argon2 encoded string (including algorithm, parameters, salt, and hash) is stored in the `password_hash` column.

### 5.2 Password Policy

- Minimum length: **12 characters**.
- No maximum length restriction (up to 128 characters to prevent DoS via extremely long inputs).
- Must contain at least one uppercase letter, one lowercase letter, one digit, and one special character.
- Password history: the system stores the last 5 password hashes per user and rejects reuse.

### 5.3 Login Flow

1. User submits username and password.
2. Server retrieves the user record by username.
3. If the user does not exist, the server performs a dummy Argon2 hash to prevent timing attacks, then returns a generic "invalid credentials" error.
4. If `locked_until` is in the future, return "account locked" with the remaining lock duration.
5. Verify the submitted password against the stored hash.
6. On failure: increment `failed_login_count`. If count reaches **5**, set `locked_until` to now + **15 minutes** and log the event.
7. On success: reset `failed_login_count` to 0, clear `locked_until`, update `last_login_at`, generate a new session token, and check device binding.
8. Return the session token as an HttpOnly, Secure, SameSite=Strict cookie.

### 5.4 Session Management

- Session tokens are 256-bit values generated by a CSPRNG, stored server-side in a sessions table (or in-memory store backed by periodic persistence).
- Each session record includes: token hash, user_id, device_fingerprint, created_at, last_active_at, expires_at.
- **Idle timeout**: 30 minutes of inactivity. Each authenticated request refreshes `last_active_at`.
- **Absolute timeout**: 12 hours regardless of activity.
- On logout, the session record is deleted immediately.
- Session tokens are transmitted only via HttpOnly cookies; they never appear in URLs or response bodies.

### 5.5 Sensitive Field Masking

- Fields designated as sensitive (email, phone) are masked in all API responses and UI displays unless the requesting user has explicit permission or is viewing their own profile.
- Masking format: first 2 characters visible, remainder replaced with asterisks, domain visible for email (e.g., `jo****@city.gov`).

---

## 6. Authorization & Role Permissions

### 6.1 Role Definitions

| Role                | Description                                                                 |
|---------------------|-----------------------------------------------------------------------------|
| Field Inspector     | Conducts field inspections, submits results, searches the knowledge base.   |
| Reviewer            | Evaluates inspection submissions using scorecards, provides scores/feedback.|
| Operations Admin    | Manages users, templates, knowledge base, campaigns, and system settings.   |
| Department Manager  | Views dashboards, KPIs, audit logs, and approves high-impact operations.    |

### 6.2 Permission Matrix

| Permission                          | Field Inspector | Reviewer | Operations Admin | Department Manager |
|-------------------------------------|:---:|:---:|:---:|:---:|
| **Knowledge Base**                  |     |     |     |     |
| Search / view entries               | Yes | Yes | Yes | Yes |
| Create / edit entries               | No  | No  | Yes | No  |
| Publish / unpublish entries         | No  | No  | Yes | Yes |
| View version history                | No  | Yes | Yes | Yes |
| **Inspections**                     |     |     |     |     |
| View own assigned inspections       | Yes | No  | Yes | Yes |
| Submit inspection results           | Yes | No  | No  | No  |
| View all inspection submissions     | No  | Yes | Yes | Yes |
| Create / edit task templates        | No  | No  | Yes | No  |
| Schedule inspections                | No  | No  | Yes | Yes |
| Excuse missed inspections           | No  | No  | Yes | Yes |
| **Reviews**                         |     |     |     |     |
| View own review assignments         | No  | Yes | Yes | Yes |
| Submit review scores                | No  | Yes | No  | No  |
| Recuse from assignment              | No  | Yes | No  | No  |
| Configure scorecards                | No  | No  | Yes | Yes |
| Override review assignments         | No  | No  | Yes | Yes |
| View review results (all)           | No  | No  | Yes | Yes |
| **Admin Console**                   |     |     |     |     |
| Manage users                        | No  | No  | Yes | Yes |
| Manage categories / tags            | No  | No  | Yes | No  |
| View KPI dashboards                 | No  | No  | Yes | Yes |
| Manage campaigns                    | No  | No  | Yes | Yes |
| Export reports (CSV/PDF)            | No  | No  | Yes | Yes |
| **Messaging**                       |     |     |     |     |
| View own notifications              | Yes | Yes | Yes | Yes |
| Manage notification templates       | No  | No  | Yes | No  |
| View notification queue status      | No  | No  | Yes | Yes |
| **Audit**                           |     |     |     |     |
| View audit logs                     | No  | No  | No  | Yes |
| Export audit logs                   | No  | No  | No  | Yes |
| **System**                          |     |     |     |     |
| Upload files                        | Yes | Yes | Yes | Yes |
| Manage device bindings              | No  | No  | Yes | Yes |
| Rollback rule versions              | No  | No  | Yes | Yes |

### 6.3 Permission Enforcement

- Permissions are enforced at the API layer via Actix-web middleware that extracts the session, resolves the user's role, and checks the role's JSONB permission set against the requested action.
- The Yew frontend also checks permissions client-side to control UI visibility, but this is cosmetic only; the server is the single source of truth.
- Any permission check failure results in an HTTP 403 response and an audit log entry.

---

## 7. Core Module Designs

### 7.1 Knowledge Base & Fuzzy Search

#### 7.1.1 Search Architecture

The search system combines PostgreSQL full-text search with trigram similarity to handle exact queries, partial matches, misspellings, and alias lookups.

**Query processing pipeline**:

1. Normalize the input: lowercase, trim whitespace, strip special characters.
2. Execute a combined query that unions three result sets:
   - **Exact full-text match** against the `search_vector` column using `plainto_tsquery`, boosted by a configurable weight (default 3.0x).
   - **Trigram similarity** on `item_name` using `similarity()` from pg_trgm, with a threshold of 0.3.
   - **Trigram similarity** on unnested `aliases` array elements, with a threshold of 0.25 (lower because aliases include expected misspellings).
3. Combine results, deduplicate by entry ID, and rank by the weighted sum of match scores multiplied by the entry's `search_weight`.
4. Return top N results (default 20) with highlighted match context.

**Index configuration**:

- `CREATE INDEX idx_kb_search_vector ON knowledge_base_entries USING GIN (search_vector);`
- `CREATE INDEX idx_kb_item_name_trgm ON knowledge_base_entries USING GIN (item_name gin_trgm_ops);`
- A functional index on unnested aliases for trigram search.

#### 7.1.2 Weight Configuration

Operations Admins can adjust the following weights via the admin console:

| Weight Parameter          | Default | Description                                         |
|---------------------------|---------|-----------------------------------------------------|
| full_text_boost           | 3.0     | Multiplier for exact full-text matches              |
| item_name_trgm_boost     | 2.0     | Multiplier for item name trigram similarity          |
| alias_trgm_boost         | 1.5     | Multiplier for alias trigram similarity              |
| entry_base_weight         | 1.0     | Per-entry configurable weight (common items boosted) |

These weights are stored in a system configuration table and applied at query time.

#### 7.1.3 Versioning

- Every modification to a knowledge base entry creates a new row in `kb_versions`.
- The `current_version_id` on the parent entry points to the active version.
- Older versions remain accessible for audit and comparison purposes.
- Region and rule version metadata travel with each kb_version, allowing inspectors to see which regulatory version was active at any point in time.
- Publishing a new version is a permissioned action that triggers a notification to affected field inspectors.

### 7.2 Inspection Task Engine

#### 7.2.1 Template System

Task templates define the structure and rules for recurring inspections:

- Each template contains one or more **subtask groups**, each group containing ordered subtasks.
- Subtasks can be marked as required or optional.
- Templates specify the inspection cycle (daily, weekly, biweekly, monthly, one-time) and the permitted time window (default 8:00 AM to 6:00 PM).
- Templates are reusable: a single template can generate inspections across multiple dates and inspectors.

#### 7.2.2 Scheduling

The scheduling engine runs as a periodic background task within the Actix-web server (using Actix's built-in scheduler or a Rust async task):

1. For each active template, determine the next scheduled date based on the cycle type.
2. Generate `inspection_submissions` records with status `scheduled` for assigned inspectors.
3. Set the `scheduled_window_start` and `scheduled_window_end` from the template.
4. Send in-app reminder notifications (via the notification engine) at a configurable lead time before the window opens.

#### 7.2.3 Fault Tolerance State Machine

Each inspection submission follows this state machine:

```
                    +------------+
                    |  scheduled |
                    +-----+------+
                          |
              +-----------+-----------+
              |                       |
        (inspector starts)    (window expires)
              |                       |
       +------v------+        +------v------+
       | in_progress  |        |   missed    |
       +------+------+        +------+------+
              |                       |
       (submits results)     (within tolerance?)
              |                 /          \
       +------v------+    Yes /            \ No
       |  completed   |      /              \
       +-------------+  +---v--------+ +----v------+
                         |makeup_pend.| | (escalate)|
                         +---+--------+ +-----------+
                             |
                    (completed within 48hr?)
                       /              \
                  Yes /                \ No
            +-------v--------+    +----v------+
            |makeup_completed|    | (escalate)|
            +----------------+    +-----------+
```

**Tolerance logic**: The system counts missed inspections within the rolling `fault_tolerance_window_days`. If the count (excluding excused) is within `fault_tolerance_missed`, a make-up opportunity is generated. If exceeded, the system escalates to the Operations Admin dashboard.

#### 7.2.4 Make-Up Rules

- When an inspection is marked `missed` and is within tolerance, the system creates a new `inspection_submissions` record with `is_makeup = true`, linking it to the original via `original_submission_id`.
- The `makeup_deadline` is set to `missed_detection_time + makeup_deadline_hours` (default 48 hours).
- If the make-up is completed before the deadline, its status becomes `makeup_completed`.
- If the deadline passes without completion, the make-up submission is also marked `missed` and the fault tolerance counter increments.

#### 7.2.5 Overdue Detection

A background task runs at a configurable interval (default: every 15 minutes):

1. Find all submissions with status `scheduled` or `in_progress` whose `scheduled_date + scheduled_window_end` is in the past.
2. Transition them to `missed`.
3. Evaluate fault tolerance and generate make-up opportunities or escalations.
4. Log all transitions in the audit log.

#### 7.2.6 Validation Feedback

When a field inspector submits results:

- The system validates that all required subtasks are completed.
- Checks that submission time falls within the permitted window (or within make-up deadline for make-up inspections).
- Validates that attached photos meet file requirements (JPEG/PNG, max 5 MB).
- Returns structured validation errors in the `validation_errors` JSONB column and surfaces them in the UI for correction before final submission.

### 7.3 Review Workspace

#### 7.3.1 Scorecard Engine

- Scorecards are configurable templates with multiple weighted dimensions.
- Each dimension has a name, description, weight, and an optional requirement for per-dimension comments.
- The overall score is calculated as: `SUM(dimension_score * dimension_weight) / SUM(dimension_weight)`.
- Rating values are bounded by `min_rating` and `max_rating` on the scorecard (e.g., 1-5).
- If `requires_comments` is true on the scorecard, the reviewer must provide at least one non-empty comment before submission.

#### 7.3.2 Assignment Algorithm

Review assignments are created automatically when an inspection submission reaches `completed` status:

1. Query all users with the Reviewer role who are active and not on leave.
2. Exclude any reviewer whose `user_id` matches the inspector who submitted (conflict-of-interest rule).
3. Exclude any reviewer who has an active recusal for the same inspector or region (stored in a recusal registry).
4. From the eligible pool, select the reviewer with the lowest current assignment count (load balancing).
5. Ties are broken by the reviewer who has been idle the longest (greatest time since last assignment).
6. Create the `review_assignments` record with `assignment_method = 'automatic'` and `is_blind = true`.

Operations Admins may also create manual assignments, overriding the automatic algorithm.

#### 7.3.3 Blind Review Implementation

When `is_blind = true` on a review assignment:

- The API endpoint serving review details to the Reviewer role strips all inspector-identifying information: `inspector_id`, inspector name, and any metadata that could identify the inspector.
- The submission's `notes` field is scanned for patterns resembling names or IDs and redacted with placeholder text (best-effort, logged for admin review).
- Photo EXIF data is stripped server-side before being served to reviewers.
- Blind status is recorded in the audit log and cannot be toggled by the reviewer.

#### 7.3.4 Recusal Logic

- A reviewer may recuse themselves from any assignment by setting the status to `recused` and providing a `recusal_reason`.
- Recusal triggers the assignment algorithm to run again for the same submission, excluding the recused reviewer.
- Recusal events are logged in the audit trail.
- Operations Admins can view all recusals and follow up if patterns emerge (e.g., a reviewer recusing excessively).

#### 7.3.5 Consistency Validation

Before a reviewer can finalize their scores:

- The system computes the standard deviation of the normalized dimension scores.
- If the deviation exceeds the scorecard's `consistency_threshold` (default 0.20), the reviewer receives a warning: "Your scores vary significantly across dimensions. Please confirm this is intentional."
- The reviewer may acknowledge the warning and proceed, or adjust their scores. The warning acknowledgment is recorded in the audit log.

### 7.4 Admin Console & Dashboards

#### 7.4.1 Overviews

The admin console provides paginated, filterable, and sortable list views for:

- **Users**: status, role, last login, device count, assignment load.
- **Knowledge Base Items**: published/draft status, region, last updated, version count.
- **Work Orders** (Inspections): status distribution, upcoming schedule, overdue items, make-up queue.

#### 7.4.2 KPI Dashboards

Dashboards present the following metrics with selectable time ranges (30, 60, 90 days):

| KPI                          | Calculation                                                                                   |
|------------------------------|-----------------------------------------------------------------------------------------------|
| Correct Sorting Conversion   | (Inspections with all sorting subtasks passed) / (Total completed inspections) * 100          |
| Template Reuse Rate          | (Inspections generated from templates used more than once) / (Total inspections) * 100        |
| 30-Day Retention             | (Inspectors active in the last 30 days) / (Total active inspectors) * 100                    |
| 60-Day Retention             | (Inspectors active in the last 60 days) / (Total active inspectors) * 100                    |
| 90-Day Retention             | (Inspectors active in the last 90 days) / (Total active inspectors) * 100                    |
| Inspection Completion Rate   | (Completed + makeup_completed) / (Total scheduled excluding excused) * 100                   |
| Average Review Score         | Mean of overall_score across completed reviews in the period                                  |
| Review Turnaround Time       | Median time from inspection completion to review completion                                   |
| Make-Up Success Rate         | (makeup_completed) / (makeup_pending created) * 100                                          |

Dashboard data is computed via materialized views refreshed on a configurable schedule (default: every hour) to avoid expensive real-time aggregation.

#### 7.4.3 Campaign Lifecycle

Campaigns (education and promotional) follow this lifecycle:

1. **Draft**: Admin creates the campaign with name, type, content, target roles/regions, and date range.
2. **Active**: When the start date arrives (checked by a background task), the campaign becomes visible to targeted users. Notifications are sent.
3. **Paused**: Admin may pause a running campaign, hiding it from users temporarily.
4. **Completed**: When the end date passes, the campaign auto-transitions to completed.
5. **Archived**: Admin may archive completed campaigns to remove them from active lists.

#### 7.4.4 Report Exports

- Reports can be exported in **CSV** or **PDF** format.
- Export is a permissioned action requiring step-up verification (re-enter password).
- CSV exports use streaming serialization to handle large datasets without excessive memory.
- PDF exports use a server-side rendering pipeline (printpdf or wkhtmltopdf).
- Generated files are stored in the local file system with an entry in the `files` table and a reference in the audit log.
- Available reports: inspection summary, review scores, KPI snapshots, user activity, audit log extract.

---

## 8. Messaging & Notification Center

### 8.1 Template Engine

The notification template engine is a simple, offline-capable variable substitution system:

- Templates use `{{variable_name}}` syntax for placeholders.
- Available variables are documented per template (stored in the `available_variables` column).
- The engine performs a single-pass substitution, replacing each `{{variable_name}}` with its resolved value from the trigger context.
- Unresolved variables are replaced with an empty string and logged as warnings.
- No nested templates or conditional logic; templates are intentionally simple to ensure reliability in an offline environment.

### 8.2 Trigger Mapping

Notifications are triggered by system events. The mapping is configured in the `notification_templates` table via the `trigger_event` column.

| Trigger Event                    | Description                                          | Default Channel |
|----------------------------------|------------------------------------------------------|-----------------|
| inspection.scheduled             | New inspection assigned to an inspector              | in_app          |
| inspection.reminder              | Upcoming inspection reminder (configurable lead time)| in_app          |
| inspection.missed                | Inspection window expired without submission         | in_app          |
| inspection.makeup_created        | Make-up opportunity generated                        | in_app          |
| inspection.makeup_deadline       | Make-up deadline approaching (4 hours before)        | in_app          |
| review.assigned                  | New review assignment                                | in_app          |
| review.completed                 | Review finalized (sent to admin/manager)             | in_app          |
| kb.version_published             | Knowledge base entry updated                         | in_app          |
| campaign.launched                | New campaign targeting the user's role/region         | in_app          |
| account.locked                   | Account locked due to failed login attempts          | in_app, email   |
| account.password_expiring        | Password approaching expiry (if policy enabled)      | in_app          |
| appeal.submitted                 | Inspector appeals a review (sent to admin)           | in_app          |
| schedule.rescheduled             | Inspection rescheduled by admin                      | in_app          |

### 8.3 Queue Management

#### In-App Notifications

- Written directly to the `notifications` table with `channel = 'in_app'` and `status = 'delivered'`.
- The Yew client polls the notifications endpoint on a configurable interval (default: 60 seconds) or receives them on page navigation.
- Users see an unread count badge and a notification inbox.

#### External Channel Notifications (SMS, Email, Push)

Since the system is fully offline, external notifications cannot be delivered directly. Instead:

1. The notification is created with `status = 'queued_for_transfer'`.
2. The system generates a **payload file** containing the notification details in a structured format.
3. The payload file is stored on disk and linked via `payload_file_id`.
4. An Operations Admin periodically transfers these payload files to a connected system (e.g., via USB drive) for actual delivery.
5. After transfer, the admin marks the notifications as `transferred` in the system.
6. If delivery confirmation is received (manually entered), the status updates to `delivered` or `failed`.

### 8.4 Payload File Format

Payload files are JSON-formatted for easy parsing by external delivery systems:

```
Filename: notif_batch_{timestamp}_{channel}.json

Structure:
{
  "batch_id": "<UUID>",
  "channel": "sms|email|push",
  "generated_at": "<ISO 8601 timestamp>",
  "notifications": [
    {
      "notification_id": "<UUID>",
      "recipient": {
        "user_id": "<UUID>",
        "email": "<decrypted email, if email channel>",
        "phone": "<decrypted phone, if sms channel>"
      },
      "subject": "<rendered subject>",
      "body": "<rendered body>",
      "priority": "normal|high"
    }
  ]
}
```

### 8.5 Delivery and Retry Tracking

- Each notification tracks `delivery_attempts` and `last_attempt_at`.
- For external channels, a "retry" means generating a new payload file for re-transfer.
- Maximum retry count: 3. After 3 failures, the notification moves to `failed` status and an alert is surfaced in the admin console.
- All status transitions are recorded in the audit log.

---

## 9. Risk Control & Device Binding

### 9.1 Rate Limiting

- **Per-user rate limit**: 60 requests per minute per authenticated user.
- Implemented via a sliding window counter stored in an in-memory data structure (e.g., a dashmap keyed by user_id with timestamped request counts).
- When the limit is exceeded, the server returns HTTP 429 with a `Retry-After` header.
- Rate limit violations are logged in the audit trail.

### 9.2 Anti-Bot Throttling

- Unauthenticated endpoints (login, password reset) are additionally throttled at **10 requests per minute per IP address**.
- After 3 consecutive failed login attempts from the same IP (regardless of username), a progressive delay is introduced: 2 seconds after the 3rd failure, 5 seconds after the 4th, 10 seconds after the 5th.
- These delays are enforced server-side; the response is simply held before returning.

### 9.3 Anomalous Login Detection

The system flags logins as anomalous if any of the following conditions are met:

- Login from a device fingerprint that has never been associated with the user.
- Login occurring outside the user's typical time window (based on historical login times, using a +/- 3 hour buffer around the median).
- Login after a period of inactivity exceeding 30 days.

Anomalous logins are:

1. Recorded with a flag in the audit log.
2. Surfaced as alerts in the Operations Admin and Department Manager dashboards.
3. Optionally (configurable), the session is created in a restricted mode requiring step-up verification before accessing sensitive operations.

### 9.4 Device-Account Binding

- On first login from a new device, the system collects a browser fingerprint (user agent, screen resolution, installed fonts hash, WebGL renderer hash, timezone) and stores it in `device_bindings`.
- Operations Admins can view and manage device bindings per user.
- If a user attempts to log in from an unbound device:
  - If the user has fewer than the maximum allowed bindings (configurable, default 3), the new device is automatically bound and the login proceeds with an anomalous login flag.
  - If the user has reached the maximum, the login is rejected with a message to contact an administrator.
- Device bindings can be revoked by an Operations Admin, forcing re-binding on next login.

### 9.5 Step-Up Verification

Certain sensitive operations require the user to re-enter their password, even within an active session:

| Operation                        | Required Re-authentication |
|----------------------------------|---------------------------|
| Export reports (CSV/PDF)         | Yes                       |
| Rollback knowledge base versions | Yes                       |
| Publish review results           | Yes                       |
| Manage device bindings           | Yes                       |
| Bulk user management actions     | Yes                       |

The step-up flow:

1. User initiates the sensitive operation.
2. The frontend displays a password re-entry modal.
3. The password is verified against the stored hash.
4. A short-lived step-up token (5 minutes) is issued and must accompany the sensitive API request.
5. The step-up event is recorded in the audit log.

---

## 10. Audit & Compliance

### 10.1 Immutable Audit Log

- Every permissioned action in the system generates an entry in the `audit_log` table.
- The application database role for writes has **INSERT-only** privileges on this table; UPDATE and DELETE are denied at the database level.
- Each entry captures: who (actor_id), what (action, resource_type, resource_id), when (created_at), where (ip_address, device_fingerprint), and context (details JSONB with before/after snapshots where applicable).

### 10.2 Logged Actions

The following categories of actions generate audit entries:

- **Authentication**: login success, login failure, logout, account lock, account unlock, password change, step-up verification.
- **Knowledge Base**: entry created, entry updated, version published, version rolled back, search weight changed.
- **Inspections**: inspection scheduled, inspection started, inspection submitted, inspection missed, make-up created, make-up completed, inspection excused.
- **Reviews**: assignment created, assignment recused, review submitted, scorecard created, scorecard modified, review results published.
- **Admin**: user created, user deactivated, role changed, campaign created/modified/archived, category/tag changed, report exported.
- **Messaging**: template created/modified, notification triggered, payload file generated, delivery status updated.
- **Risk Control**: rate limit exceeded, anomalous login detected, device bound, device revoked, step-up verification performed.
- **Files**: file uploaded, file deduplicated, file integrity check failed.

### 10.3 Audit Log Export

- Department Managers can export audit logs in **CSV** or **PDF** format.
- Exports support date range filters, action type filters, and actor filters.
- Export generation requires step-up verification.
- The export action itself is recorded in the audit log (meta-audit).
- No network connection is required; exports are generated server-side and stored as local files.

### 10.4 Retention

- Audit log entries are retained indefinitely by default.
- A configurable archival policy allows entries older than a threshold (e.g., 2 years) to be exported and then marked as archived (a soft flag; rows are never deleted).

---

## 11. File Storage Strategy

### 11.1 Accepted Formats

- **JPEG** (image/jpeg) and **PNG** (image/png) only.
- All other formats are rejected at upload time with a descriptive error message.
- MIME type is validated both by file extension and by inspecting the file's magic bytes.

### 11.2 Size Limits

- Maximum file size: **5 MB** per file.
- Enforced at the Actix-web layer via request body size limits and validated again after the file is fully received.

### 11.3 Storage Layout

Files are stored on local disk under a configurable root directory with the following structure:

```
{storage_root}/
  uploads/
    {YYYY}/
      {MM}/
        {UUID}.{ext}
  exports/
    {YYYY}/
      {MM}/
        {UUID}.{ext}
  payloads/
    {YYYY}/
      {MM}/
        notif_batch_{timestamp}_{channel}.json
```

- The `stored_name` in the `files` table uses the UUID-based filename.
- Year/month partitioning keeps directory sizes manageable.

### 11.4 Fingerprint Deduplication

- On upload, the server computes the **SHA-256 checksum** of the file contents.
- The checksum is checked against the unique index on `files.sha256_checksum`.
- If a match is found:
  - A new `files` record is created with `is_duplicate = true` and `duplicate_of` pointing to the original file's ID.
  - The physical file is **not** written to disk again.
  - The original file's storage path is used for serving.
- This saves disk space when inspectors upload the same reference images repeatedly.

### 11.5 Integrity Verification

- A background task periodically (configurable, default: nightly) scans all non-duplicate file records:
  - Recomputes the SHA-256 checksum of the physical file.
  - Compares it to the stored `sha256_checksum`.
  - If they do not match, the file is flagged, an alert is raised in the admin console, and an audit log entry is created.
- This detects accidental or malicious file corruption on disk.

---

## 12. Security Measures

### 12.1 CSRF Protection

- All state-changing API requests require a CSRF token.
- The token is generated server-side, embedded in the initial HTML page load, and sent as a custom HTTP header (`X-CSRF-Token`) with every mutating request.
- The server validates the CSRF token against the session on every POST, PUT, PATCH, and DELETE request.
- Since the Yew WASM client is a single-page application served from the same origin, SameSite=Strict cookies provide an additional layer of CSRF defense.

### 12.2 XSS Prevention

- The Yew framework renders all user-supplied content through its virtual DOM, which escapes HTML by default.
- The server sets the following response headers:
  - `Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;`
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
- Any API endpoint that returns user-supplied text ensures it is properly escaped for the context (JSON strings in API responses).
- Rich text is not supported; all text fields are plain text only.

### 12.3 SQL Injection Prevention

- All database queries use parameterized statements via Diesel or SQLx.
- No raw string interpolation into SQL is permitted.
- The Rust type system enforces this at compile time when using Diesel's query DSL or SQLx's compile-time checked queries.
- Dynamic search queries (fuzzy search with configurable weights) use parameterized prepared statements with weight values passed as parameters, never interpolated.

### 12.4 Encryption at Rest

- Sensitive database fields (email, phone) are encrypted using **AES-256-GCM** before storage.
- Encryption keys are managed by the application:
  - A master key is stored in a protected configuration file on the server, readable only by the application's OS user.
  - Per-field encryption uses a derived key (HKDF from the master key with a field-specific context).
  - Key rotation: when the master key is rotated, a background process re-encrypts all sensitive fields. Both old and new keys are kept active during the rotation window.
- The PostgreSQL data directory should additionally reside on an encrypted filesystem (LUKS or equivalent), managed by the IT department.

### 12.5 Field Masking

- Sensitive fields are masked in API responses by default.
- The masking middleware intercepts outgoing responses and applies field-specific masking rules:
  - Email: `jo****@city.gov` (first 2 characters of local part visible, domain visible).
  - Phone: `***-***-1234` (last 4 digits visible).
- Unmasked values are only returned when:
  - The user is viewing their own profile.
  - The user has the explicit `view_sensitive_fields` permission (Operations Admin, Department Manager).
  - The request includes a valid step-up token.

### 12.6 Additional Measures

- **HTTP Strict Transport Security (HSTS)**: Enabled if TLS is configured on the LAN.
- **Request size limits**: 10 MB global maximum, 5 MB for file uploads specifically.
- **Input validation**: All API inputs are validated using Rust's serde deserialization with strict type enforcement. Invalid payloads are rejected with HTTP 400.
- **Dependency auditing**: `cargo audit` is run as part of the build process to detect known vulnerabilities in dependencies.
- **No external dependencies at runtime**: The system makes no outbound network calls, eliminating supply-chain attack vectors during operation.

---

## 13. Data Model Conventions

### 13.1 Primary Keys

- All tables use **UUIDs** (v4) as primary keys.
- UUIDs are generated server-side using a CSPRNG.
- This avoids sequential ID enumeration and simplifies potential future data merging across deployments.

### 13.2 Timestamps

- All tables include `created_at` (TIMESTAMPTZ, NOT NULL, DEFAULT now()).
- Mutable tables include `updated_at` (TIMESTAMPTZ, NOT NULL, DEFAULT now()), maintained via a database trigger that sets `updated_at = now()` on every UPDATE.
- All timestamps are stored in UTC. The Yew client converts to the local timezone for display.

### 13.3 Soft Deletes

- Tables representing business entities (users, knowledge_base_entries, task_templates, campaigns) use soft deletes via a `deleted_at` (TIMESTAMPTZ, NULLABLE) column.
- A non-null `deleted_at` indicates the record is logically deleted.
- All queries include a `WHERE deleted_at IS NULL` filter by default (enforced via application-layer query builders or database views).
- The `audit_log` table does **not** support soft deletes or any form of deletion.

### 13.4 JSONB Usage

JSONB columns are used for:

- `roles.permissions`: structured permission sets that can evolve without schema migrations.
- `inspection_submissions.subtask_results`: flexible subtask completion data.
- `inspection_submissions.validation_errors`: structured validation feedback.
- `review_assignments.dimension_scores`: per-dimension score storage.
- `campaigns.content`: flexible campaign content structure.
- `audit_log.details`: context-specific event data.

All JSONB columns have documented schemas enforced at the application layer.

### 13.5 Naming Conventions

- Table names: `snake_case`, plural (e.g., `users`, `task_templates`).
- Column names: `snake_case` (e.g., `created_at`, `file_size_bytes`).
- Foreign keys: `{referenced_table_singular}_id` (e.g., `user_id`, `template_id`).
- Indexes: `idx_{table}_{column(s)}` (e.g., `idx_kb_item_name_trgm`).
- Constraints: `chk_{table}_{description}` (e.g., `chk_campaigns_date_range`).

---

## 14. Deployment

### 14.1 Deployment Model

CivicSort is deployed entirely on-premises with no internet connectivity required at any point during operation. The deployment target is a single server (physical or virtual) on the municipal LAN.

### 14.2 Server Requirements

| Resource    | Minimum          | Recommended       |
|-------------|------------------|-------------------|
| CPU         | 4 cores          | 8 cores           |
| RAM         | 8 GB             | 16 GB             |
| Storage     | 100 GB SSD       | 500 GB SSD        |
| OS          | Linux (Debian/Ubuntu LTS, RHEL) | Same   |
| Network     | LAN connectivity | Gigabit LAN       |

### 14.3 Build Process

Since the deployment environment has no internet access, the build process occurs on a separate build machine with internet connectivity:

1. **Rust toolchain setup**: Install Rust stable toolchain with the `wasm32-unknown-unknown` target.
2. **Frontend build**: Use Trunk to compile the Yew application to WASM and bundle static assets.
3. **Backend build**: Use Cargo to compile the Actix-web server as a statically linked binary (using musl for Linux targets).
4. **Database migrations**: Package Diesel or SQLx migration files alongside the binary.
5. **Bundle**: Package the following into a single archive:
   - Server binary
   - Frontend static assets (HTML, JS, WASM, CSS)
   - Migration files
   - Configuration template
   - Systemd service file
   - Installation script

### 14.4 Transfer and Installation

1. The build archive is transferred to the target server via secure physical media (USB drive) or a trusted internal file share.
2. The installation script:
   - Installs PostgreSQL if not already present (from bundled packages).
   - Runs database migrations.
   - Places the server binary and frontend assets in the appropriate directories.
   - Configures the systemd service for automatic startup and restart on failure.
   - Sets file permissions (application user owns the binary and storage directories; restricted access to the configuration file containing the encryption master key).
3. The administrator edits the configuration file to set:
   - Database connection string.
   - File storage root path.
   - Encryption master key (generated during initial setup).
   - LAN bind address and port.
   - TLS certificate and key paths (if applicable).

### 14.5 Updates

- Updates follow the same build-transfer-install process.
- The installation script detects an existing installation and runs only new database migrations.
- A pre-update backup of the database and file storage is strongly recommended (the script can automate this).
- Rollback is supported by restoring the previous binary and running reverse migrations (if provided).

### 14.6 Backup Strategy

- **Database**: PostgreSQL `pg_dump` run nightly via cron, stored to a separate local disk or external media.
- **File storage**: rsync-based incremental backup to a secondary storage location.
- **Backup verification**: Weekly restore test to a separate environment (recommended but managed by IT operations, not the application).

### 14.7 Monitoring

Since no external monitoring services are available:

- The Actix-web server exposes a `/health` endpoint returning system status (database connectivity, disk space, last backup timestamp).
- Application logs are written to structured JSON files, rotated daily, with configurable retention.
- The admin dashboard includes a system health panel showing: server uptime, database size, file storage usage, active sessions, and recent error counts.
