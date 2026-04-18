# CivicSort Static Delivery Acceptance & Architecture Audit (Self-Test #3)

## 1. Verdict
- Overall conclusion: **Partial Pass**

## 2. Scope and Static Verification Boundary
- Reviewed: backend code under `repo/src/**`, frontend code under `repo/frontend/src/**`, migrations `repo/migrations/**`, tests in `repo/unit_tests/**` and `repo/API_tests/**`, and documentation/config (`repo/README.md`, `repo/.env.example`, runners).
- Not reviewed: runtime behavior, deployment environment state, browser execution, DB runtime data.
- Intentionally not executed: server start, Docker, tests, external services.
- Manual verification required for throughput/load effects, anti-bot behavior under real traffic, and UI runtime rendering fidelity.

## 3. Repository / Requirement Mapping Summary
- Prompt target: one offline-first civic operations app with role-based workflows, KB fuzzy search, inspections/scheduling, review + COI/blind + disputes, admin analytics, offline messaging exports, and strict security controls.
- Implementation mapping: app shell + login/role nav in Yew (`repo/frontend/src/app.rs:14`), Actix scopes for auth/users/audit/devices/kb/inspection/reviews/admin/messaging/bulk/disputes (`repo/src/main.rs:56`), disputed classification module/migration (`repo/src/routes/dispute_routes.rs:143`, `repo/migrations/008_disputed_classifications.sql:11`), encryption migration (`repo/migrations/009_encrypted_fields.sql:5`).
- Net: significant security and requirement-fit improvements vs prior audits, but encryption-at-rest and some security/test controls are still incomplete.

## 4. Section-by-section Review

### 4.1 Hard Gates

#### 4.1.1 Documentation and static verifiability
- Conclusion: **Pass**
- Rationale: README now covers startup/config/migrations/tests/offline constraints and bootstrap registration policy.
- Evidence: `repo/README.md:21`, `repo/README.md:59`, `repo/README.md:69`, `repo/README.md:73`, `repo/README.md:88`

#### 4.1.2 Material deviation from Prompt
- Conclusion: **Pass**
- Rationale: delivered frontend now mounts a unified application shell with role-aware navigation, and backend includes disputes/workspaces expected by prompt.
- Evidence: `repo/frontend/src/main.rs:10`, `repo/frontend/src/app.rs:86`, `repo/src/main.rs:66`

### 4.2 Delivery Completeness

#### 4.2.1 Core explicit requirements implemented
- Conclusion: **Partial Pass**
- Rationale: disputed-classification flow is materially implemented and review-integrated, but encryption-at-rest requirement remains incomplete in data paths.
- Evidence: `repo/src/routes/review_routes.rs:416`, `repo/src/routes/review_routes.rs:484`, `repo/src/routes/dispute_routes.rs:27`, `repo/migrations/009_encrypted_fields.sql:5`, `repo/src/db/devices.rs:32`

#### 4.2.2 Basic 0→1 end-to-end deliverable
- Conclusion: **Pass**
- Rationale: repository is a full multi-module application (frontend/backend/migrations/tests/docs), not a fragment/demo.
- Evidence: `repo/src/main.rs:52`, `repo/frontend/src/app.rs:114`, `repo/API_tests/run_api_tests.sh:42`

### 4.3 Engineering and Architecture Quality

#### 4.3.1 Structure and module decomposition
- Conclusion: **Pass**
- Rationale: clear domain decomposition by routes/db/models/services; security/risk and dispute modules are separated cleanly.
- Evidence: `repo/src/routes/mod.rs:1`, `repo/src/db/mod.rs:1`, `repo/src/models/mod.rs:1`

#### 4.3.2 Maintainability/extensibility
- Conclusion: **Partial Pass**
- Rationale: architecture is extensible, but policy consistency gaps remain (audit coverage and encryption integration not uniform).
- Evidence: `repo/src/routes/messaging_routes.rs:322`, `repo/src/routes/bulk_data_routes.rs:372`, `repo/src/routes/review_routes.rs:391`

### 4.4 Engineering Details and Professionalism

#### 4.4.1 Error handling/logging/validation/API design
- Conclusion: **Partial Pass**
- Rationale: strong typed errors and validation are present; secure fail behavior improved for encrypted notes, but sensitive-field handling remains inconsistent across modules.
- Evidence: `repo/src/errors.rs:74`, `repo/src/routes/inspection_routes.rs:523`, `repo/src/routes/inspection_routes.rs:532`, `repo/src/db/devices.rs:32`

#### 4.4.2 Product-grade vs demo-grade
- Conclusion: **Partial Pass**
- Rationale: product-level breadth is present; unresolved high-security requirement (encryption-at-rest) still blocks full acceptance quality.
- Evidence: `repo/README.md:120`, `repo/migrations/009_encrypted_fields.sql:5`, `repo/src/routes/inspection_routes.rs:528`

### 4.5 Prompt Understanding and Requirement Fit

#### 4.5.1 Business goal and constraints fit
- Conclusion: **Partial Pass**
- Rationale: role workflows, disputes, and single-app delivery align much better; strict prompt constraint on sensitive fields encryption/masking is not fully realized across persistence and responses.
- Evidence: `repo/frontend/src/app.rs:89`, `repo/src/routes/dispute_routes.rs:97`, `repo/src/routes/device_routes.rs:78`, `repo/src/db/devices.rs:32`

### 4.6 Aesthetics (frontend/full-stack)

#### 4.6.1 Visual and interaction quality fit
- Conclusion: **Partial Pass**
- Rationale: static code indicates structured navigation/login/workspaces, but rendering quality and responsive behavior remain runtime-verification items.
- Evidence: `repo/frontend/src/app.rs:86`, `repo/frontend/src/pages/login_page.rs:29`, `repo/frontend/index.html:97`
- Manual verification note: browser/device check required.

## 5. Issues / Suggestions (Severity-Rated)

1) Severity: **High**
- Title: Encryption-at-rest is only partially integrated and sensitive fields still persist/read as plaintext paths
- Conclusion: **Fail**
- Evidence: encryption columns exist but are not used by device persistence (`repo/migrations/009_encrypted_fields.sql:8`, `repo/src/db/devices.rs:32`), and encrypted helpers are only used in submission notes path (`repo/src/routes/inspection_routes.rs:528`, `grep encrypt_field/decrypt_field results`).
- Impact: prompt-required protection (“sensitive fields encrypted at rest”) is not uniformly met; sensitive values may remain plaintext in DB and responses.
- Minimum actionable fix: migrate sensitive write/read paths to encrypted columns (`encrypted_fingerprint`, `encrypted_notes`, `encrypted_details`) with strict no-plaintext persistence and consistent masking on outward responses.

2) Severity: **Medium**
- Title: Permissioned mutating actions are still not universally audited
- Conclusion: **Partial Pass**
- Evidence: mutation handlers without audit events include `repo/src/routes/messaging_routes.rs:322`, `repo/src/routes/bulk_data_routes.rs:372`, `repo/src/routes/review_routes.rs:391`.
- Impact: immutable audit trail can miss privileged actions, weakening compliance and forensic traceability.
- Minimum actionable fix: add `audit_action` for all state-changing permissioned routes and branches.

3) Severity: **Medium**
- Title: Step-up critical action coverage remains weak in API tests
- Conclusion: **Insufficient**
- Evidence: no step-up test calls detected in `repo/API_tests/*.sh` (search for `/api/auth/stepup` / `stepup`), while step-up is required for exports (`repo/src/routes/admin_routes.rs:271`, `repo/src/routes/bulk_data_routes.rs:123`).
- Impact: step-up regressions could pass test suite undetected.
- Minimum actionable fix: add pre/post step-up API assertions for `export_csv`/`export_pdf`-gated endpoints.

4) Severity: **Medium**
- Title: Security tests still sparse for cross-object access beyond disputes
- Conclusion: **Insufficient**
- Evidence: dispute cross-user denial test exists (`repo/API_tests/test_11_disputes.sh:71`), but equivalent negatives for submission/review object reads are absent (`repo/API_tests/test_05_inspection.sh`, `repo/API_tests/test_06_reviews.sh`).
- Impact: object-level auth regressions in core flows can evade detection.
- Minimum actionable fix: add cross-user/cross-role 403 tests for `/api/inspection/submissions/{id}` and `/api/reviews/{id}`.

## 6. Security Review Summary

- authentication entry points: **Pass**
  - Evidence: registration now gated to admin except explicit first-boot bootstrap condition (`repo/src/routes/auth_routes.rs:27`, `repo/src/routes/auth_routes.rs:43`, `repo/src/routes/auth_routes.rs:48`).
- route-level authorization: **Pass**
  - Evidence: key sensitive endpoints have role checks in bulk/messaging/review/disputes (`repo/src/routes/bulk_data_routes.rs:233`, `repo/src/routes/messaging_routes.rs:171`, `repo/src/routes/review_routes.rs:115`, `repo/src/routes/dispute_routes.rs:64`).
- object-level authorization: **Partial Pass**
  - Evidence: ownership checks exist for submission/review/dispute detail (`repo/src/routes/inspection_routes.rs:621`, `repo/src/routes/review_routes.rs:348`, `repo/src/routes/dispute_routes.rs:97`).
  - Limitation: test coverage for these checks remains incomplete.
- function-level authorization: **Partial Pass**
  - Evidence: step-up verification is enforced on critical exports (`repo/src/routes/admin_routes.rs:271`, `repo/src/routes/bulk_data_routes.rs:123`).
  - Limitation: static tests do not prove end-to-end enforcement.
- tenant / user isolation: **Partial Pass**
  - Evidence: per-user constraints exist in devices and object access checks (`repo/src/db/devices.rs:80`, `repo/src/routes/dispute_routes.rs:98`).
  - Limitation: no tenant model and incomplete isolation tests for all sensitive resources.
- admin / internal / debug protection: **Pass**
  - Evidence: privileged modules guarded for mutation/visibility in admin/audit/bulk/messaging core endpoints.

## 7. Tests and Logging Review

- Unit tests: **Partial Pass**
  - Present and expanded (`repo/src/auth/lockout_tests.rs:1`, `repo/src/scheduling/lifecycle_tests.rs:1`, `repo/src/review/coi_tests.rs:1`, `repo/src/messaging/payload_lifecycle_tests.rs:1`) but many are logic-unit approximations vs route/integration behavior.
- API / integration tests: **Partial Pass**
  - Broad suite including disputes (`repo/API_tests/test_11_disputes.sh:1`) and registration lockdown path (`repo/API_tests/test_02_auth.sh:24`) but still missing step-up and some object-level negative paths.
- Logging categories / observability: **Partial Pass**
  - Good base logging and audit framework present (`repo/src/main.rs:24`, `repo/src/middleware/audit_middleware.rs:9`), yet some mutating handlers lack audit calls.
- Sensitive-data leakage risk in logs/responses: **Partial Pass**
  - Masking exists for listed devices (`repo/src/routes/device_routes.rs:38`), but write/read paths still include plaintext storage patterns in devices table and response objects (`repo/src/db/devices.rs:32`, `repo/src/routes/device_routes.rs:78`).

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview
- Unit tests exist and are runnable via cargo test wrapper (`repo/unit_tests/run_unit_tests.sh:26`).
- API integration tests exist and include 11 suites (`repo/API_tests/run_api_tests.sh:43`).
- Test commands and structure are documented (`repo/README.md:88`, `repo/README.md:104`).

### 8.2 Coverage Mapping Table

| Requirement / Risk Point | Mapped Test Case(s) | Key Assertion / Fixture / Mock | Coverage Assessment | Gap | Minimum Test Addition |
|---|---|---|---|---|---|
| Registration lockdown + role-based user creation | `repo/API_tests/test_02_auth.sh:24`, `repo/API_tests/test_02_auth.sh:42`, `repo/API_tests/test_02_auth.sh:63` | anonymous denied; admin allowed; inspector denied | basically covered | bootstrap path depends on env/empty-DB assumptions not independently asserted | add explicit bootstrap-env setup assertion or dedicated bootstrap integration test harness |
| Dispute object-level auth | `repo/API_tests/test_11_disputes.sh:71` | inspector denied on other user dispute detail | sufficient | only dispute object tested | extend same pattern to reviews/submissions |
| Submission/review object-level auth | none specific in API tests | N/A | missing | no cross-user object denial assertions | add negative tests for `/api/inspection/submissions/{id}` and `/api/reviews/{id}` |
| Step-up enforcement on exports | none in API tests | N/A | missing | no test for pre-stepup denial and post-stepup success | add tests invoking `/api/auth/stepup` then export endpoints |
| Encryption-at-rest behavior | none proving encrypted column usage | N/A | missing | no static tests verify encrypted columns are used or plaintext blocked | add unit/integration tests around encrypted persistence and response masking |
| KB fuzzy ranking quality | `repo/API_tests/test_04_kb.sh:72` | search contains expected item | insufficient | no weighted order assertions for alias/misspelling scenarios | add deterministic ranking assertions with fixtures |
| Scheduling edge semantics | `repo/src/scheduling/lifecycle_tests.rs:14` | date count/arithmetic checks | basically covered | limited integration with route/db lifecycle transitions | add API-driven overdue/makeup transition tests |

### 8.3 Security Coverage Audit
- authentication: **basically covered** (improved registration tests included).
- route authorization: **basically covered** for many modules.
- object-level authorization: **insufficient** (only dispute path has explicit cross-user API test).
- tenant / data isolation: **insufficient** (partial object tests only).
- admin / internal protection: **basically covered**, but step-up test gap remains.

### 8.4 Final Coverage Judgment
**Partial Pass**

Major risks covered: auth lifecycle, broad module smoke checks, dispute object denial.

Major uncovered risks: step-up enforcement tests, encrypted-at-rest correctness tests, and cross-object authorization negatives in review/inspection flows.

## 9. Final Notes
- Static evidence shows meaningful fixes from earlier rounds (registration lockdown model, dispute detail auth, encryption fail-closed for submission notes).
- Remaining blocker to full acceptance is consistent prompt-level security completion, especially encrypted-at-rest implementation breadth.
