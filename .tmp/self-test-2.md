# CivicSort Static Delivery Acceptance & Architecture Audit (Self-Test #2)

## 1. Verdict
- Overall conclusion: **Fail**

## 2. Scope and Static Verification Boundary
- Reviewed: backend routes/services/db/models (`repo/src/**`), frontend app/pages/services (`repo/frontend/src/**`), migrations (`repo/migrations/**`), docs/config/scripts (`repo/README.md`, `repo/.env.example`, `repo/run_tests.sh`, `repo/API_tests/**`, `repo/unit_tests/**`).
- Not reviewed: compiled outputs and runtime state.
- Intentionally not executed: project startup, Docker, API calls, browser UI, unit/API tests.
- Manual verification required for runtime-only claims (actual auth/session timing behavior, anti-bot effectiveness under load, browser responsiveness/rendering).

## 3. Repository / Requirement Mapping Summary
- Prompt target: offline-first civic sanitation operations platform with role-based workspaces, KB fuzzy search, inspections/scheduling, review + COI/blind flow, admin KPIs/campaigns/reports, messaging queue exports, and strong security controls.
- Mapped implementation: Actix modular routes include auth/user/audit/device/kb/inspection/review/admin/messaging/bulk/disputes (`repo/src/main.rs:56`-`repo/src/main.rs:67`), Yew app shell with login + role nav (`repo/frontend/src/app.rs:14`-`repo/frontend/src/app.rs:124`), migrations up to disputed classifications and encrypted columns (`repo/migrations/008_disputed_classifications.sql:1`, `repo/migrations/009_encrypted_fields.sql:1`).
- Net result: major gaps from first report were reduced (frontend app shell and dispute module added), but material security/encryption/test-coverage gaps remain.

## 4. Section-by-section Review

### 4.1 Hard Gates

#### 4.1.1 Documentation and static verifiability
- Conclusion: **Pass**
- Rationale: central README now provides setup/run/config/migrations/tests and endpoint map, aligned with code layout.
- Evidence: `repo/README.md:21`, `repo/README.md:59`, `repo/README.md:72`, `repo/README.md:88`, `repo/README.md:141`

#### 4.1.2 Material deviation from Prompt
- Conclusion: **Partial Pass**
- Rationale: runtime frontend now mounts a single app shell with login and role-aware navigation across workspaces, matching prompt intent better; disputed classification route exists. Remaining deviations are mainly security/completeness quality, not architecture direction.
- Evidence: `repo/frontend/src/main.rs:10`, `repo/frontend/src/app.rs:66`, `repo/frontend/src/app.rs:89`, `repo/src/routes/dispute_routes.rs:138`

### 4.2 Delivery Completeness

#### 4.2.1 Core explicit requirements implemented
- Conclusion: **Partial Pass**
- Rationale: disputed-classification data/API flow is now present, but encryption-at-rest requirement is still only partially implemented (encrypted columns added but not consistently used; plaintext fallback still allowed).
- Evidence: `repo/src/routes/review_routes.rs:416`, `repo/src/routes/review_routes.rs:484`, `repo/migrations/009_encrypted_fields.sql:5`, `repo/src/db/devices.rs:32`, `repo/src/routes/inspection_routes.rs:521`

#### 4.2.2 Basic 0->1 end-to-end deliverable
- Conclusion: **Pass**
- Rationale: complete multi-module backend + frontend SPA + migrations + test suites + docs are present and statically coherent.
- Evidence: `repo/src/main.rs:52`, `repo/frontend/src/app.rs:83`, `repo/migrations/001_initial_schema.sql:1`, `repo/API_tests/run_api_tests.sh:42`

### 4.3 Engineering and Architecture Quality

#### 4.3.1 Structure and module decomposition
- Conclusion: **Pass**
- Rationale: clear domain decomposition by route/service/db/model with separate modules for security, scheduling, review, messaging, dedup, disputes.
- Evidence: `repo/src/routes/mod.rs:1`, `repo/src/db/mod.rs:1`, `repo/src/models/mod.rs:1`, `repo/src/risk/stepup.rs:10`

#### 4.3.2 Maintainability/extensibility
- Conclusion: **Partial Pass**
- Rationale: architecture is extensible, but security policy enforcement and audit coverage are still inconsistent across endpoints, indicating governance debt.
- Evidence: `repo/src/routes/dispute_routes.rs:88`, `repo/src/routes/messaging_routes.rs:145`, `repo/src/routes/messaging_routes.rs:322`, `repo/src/routes/bulk_data_routes.rs:372`

### 4.4 Engineering Details and Professionalism

#### 4.4.1 Error handling/logging/validation/API design
- Conclusion: **Partial Pass**
- Rationale: strong typed errors and validation exist; however, some high-risk controls remain inconsistent (open registration, under-protected read endpoint, selective audit writes).
- Evidence: `repo/src/errors.rs:74`, `repo/src/routes/auth_routes.rs:27`, `repo/src/routes/dispute_routes.rs:83`, `repo/src/routes/messaging_routes.rs:319`, `repo/src/routes/bulk_data_routes.rs:372`

#### 4.4.2 Product-grade vs demo-grade
- Conclusion: **Partial Pass**
- Rationale: product-like breadth and workflows are present, but remaining security/test gaps are still significant for acceptance-grade delivery.
- Evidence: `repo/README.md:120`, `repo/src/routes/review_routes.rs:108`, `repo/API_tests/test_11_disputes.sh:14`

### 4.5 Prompt Understanding and Requirement Fit

#### 4.5.1 Business goal and constraints fit
- Conclusion: **Partial Pass**
- Rationale: requirement understanding improved (single app shell, disputes, role spaces), but key security and strict encryption constraints are still not fully met.
- Evidence: `repo/frontend/src/app.rs:86`, `repo/src/routes/dispute_routes.rs:27`, `repo/src/routes/inspection_routes.rs:519`, `repo/src/db/devices.rs:32`

### 4.6 Aesthetics (frontend/full-stack)

#### 4.6.1 Visual and interaction quality fit
- Conclusion: **Partial Pass**
- Rationale: static code shows role-nav, login screen, and page segmentation; browser-level responsiveness and rendering quality cannot be proven statically.
- Evidence: `repo/frontend/src/app.rs:86`, `repo/frontend/src/pages/login_page.rs:29`, `repo/frontend/index.html:97`
- Manual verification note: mobile/desktop rendering and interaction behavior require manual browser check.

## 5. Issues / Suggestions (Severity-Rated)

1) Severity: **High**
- Title: Open user registration endpoint allows unauthenticated account creation
- Conclusion: **Fail**
- Evidence: `repo/src/routes/auth_routes.rs:27`, `repo/src/routes/auth_routes.rs:188`
- Impact: unauthorized local users can self-provision privileged roles if request payload is accepted, weakening permission model and operations governance.
- Minimum actionable fix: restrict registration to privileged authenticated admins/managers (or disable public register and move user creation to protected admin path only).

2) Severity: **High**
- Title: Dispute detail endpoint lacks role/ownership authorization
- Conclusion: **Fail**
- Evidence: `repo/src/routes/dispute_routes.rs:83`, `repo/src/routes/dispute_routes.rs:88`, `repo/src/routes/dispute_routes.rs:103`
- Impact: any authenticated user can read arbitrary dispute records by ID, risking data disclosure.
- Minimum actionable fix: require reviewer/admin/manager role or dispute owner check before returning dispute detail.

3) Severity: **High**
- Title: Encryption-at-rest remains partial and bypassable
- Conclusion: **Fail**
- Evidence: encrypted columns added but unused in data access (`repo/migrations/009_encrypted_fields.sql:5`, `repo/src/db/devices.rs:32`), and submission notes fall back to plaintext on encryption error (`repo/src/routes/inspection_routes.rs:521`).
- Impact: sensitive fields can still be stored in plaintext, violating strict encryption-at-rest requirement.
- Minimum actionable fix: persist sensitive values to dedicated encrypted columns only, remove plaintext fallback, and enforce key presence for sensitive-write paths.

4) Severity: **Medium**
- Title: Audit logging still not universal for permissioned mutating actions
- Conclusion: **Partial Pass**
- Evidence: mutating paths without audit call include `repo/src/routes/messaging_routes.rs:319` (mark_failed), `repo/src/routes/bulk_data_routes.rs:372` (non-approval merge review branch), `repo/src/routes/review_routes.rs:391` (revoke_coi).
- Impact: compliance/audit completeness is weakened; important security-relevant actions can be missing from immutable trail.
- Minimum actionable fix: add `audit_action` to all state-changing permissioned handlers, including all branches.

5) Severity: **Medium**
- Title: Test coverage still misses critical security paths
- Conclusion: **Fail**
- Evidence: no API step-up tests present (`API_tests` contains no stepup invocations), and object-level auth negatives for sensitive resources are sparse (`repo/API_tests/test_05_inspection.sh:120`, `repo/API_tests/test_06_reviews.sh:78`).
- Impact: severe authz/step-up regressions could pass CI undetected.
- Minimum actionable fix: add API tests for step-up required actions (pre/post verification), cross-user access denial for submission/review/dispute detail endpoints, and privileged mutation authz checks.

6) Severity: **Low**
- Title: Messaging trigger listing may overexpose operational rule configuration
- Conclusion: **Partial Pass**
- Evidence: `repo/src/routes/messaging_routes.rs:142`-`repo/src/routes/messaging_routes.rs:149` authenticates but does not role-restrict trigger listing.
- Impact: internal workflow metadata visible to non-admin users.
- Minimum actionable fix: restrict trigger listing to operations/admin roles unless business policy explicitly allows broad read access.

## 6. Security Review Summary

- authentication entry points: **Partial Pass**
  - Evidence: local auth flow, lockout/session logic and anti-bot login check exist (`repo/src/routes/auth_routes.rs:51`, `repo/src/db/users.rs:47`, `repo/src/db/sessions.rs:7`, `repo/src/routes/auth_routes.rs:62`), but public register path remains exposed (`repo/src/routes/auth_routes.rs:27`).
- route-level authorization: **Partial Pass**
  - Evidence: many sensitive modules now require roles (`repo/src/routes/bulk_data_routes.rs:89`, `repo/src/routes/messaging_routes.rs:171`), but dispute detail lacks role restriction (`repo/src/routes/dispute_routes.rs:83`).
- object-level authorization: **Partial Pass**
  - Evidence: submission/review object checks added (`repo/src/routes/inspection_routes.rs:608`, `repo/src/routes/review_routes.rs:348`), but dispute object access is not constrained by owner/role.
- function-level authorization: **Partial Pass**
  - Evidence: step-up checks used for exports (`repo/src/routes/admin_routes.rs:271`, `repo/src/routes/bulk_data_routes.rs:123`), but missing test evidence and some privileged reads remain broader than necessary.
- tenant / user isolation: **Partial Pass**
  - Evidence: per-user checks exist in inspections/reviews/devices (`repo/src/routes/inspection_routes.rs:488`, `repo/src/routes/review_routes.rs:251`, `repo/src/db/devices.rs:80`), yet dispute detail isolation gap remains.
- admin / internal / debug protection: **Partial Pass**
  - Evidence: admin/audit/bulk/messaging mutations mostly role-guarded (`repo/src/routes/admin_routes.rs:34`, `repo/src/routes/audit_routes.rs:31`, `repo/src/routes/messaging_routes.rs:255`), with exceptions noted above.

## 7. Tests and Logging Review

- Unit tests: **Partial Pass**
  - Existence improved with new files (`repo/src/auth/lockout_tests.rs:1`, `repo/src/scheduling/lifecycle_tests.rs:1`, `repo/src/review/coi_tests.rs:1`, `repo/src/messaging/payload_lifecycle_tests.rs:1`), but several are logic-level approximations and do not verify route/security integration.
- API / integration tests: **Partial Pass**
  - Suite breadth expanded (`repo/API_tests/test_11_disputes.sh:1`), but critical step-up/object-auth paths remain under-covered.
- Logging categories / observability: **Partial Pass**
  - Structured auth/audit/anomaly logs are present (`repo/src/routes/auth_routes.rs:67`, `repo/src/middleware/audit_middleware.rs:9`), but not all privileged mutations are audited.
- Sensitive-data leakage risk in logs/responses: **Partial Pass**
  - Password hash not serialized (`repo/src/models/user.rs:27`) and device list masks fingerprint (`repo/src/routes/device_routes.rs:38`), but bind/trust endpoints still return full device object and encryption controls are inconsistent.

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview
- Unit tests exist via Rust harness (`repo/unit_tests/run_unit_tests.sh:26`).
- API tests exist via shell/curl suite (`repo/API_tests/run_api_tests.sh:42`).
- Test entry points and commands are documented (`repo/README.md:88`, `repo/README.md:92`, `repo/README.md:99`).

### 8.2 Coverage Mapping Table

| Requirement / Risk Point | Mapped Test Case(s) | Key Assertion / Fixture / Mock | Coverage Assessment | Gap | Minimum Test Addition |
|---|---|---|---|---|---|
| Auth lifecycle + 401 basics | `repo/API_tests/test_02_auth.sh:60`, `repo/API_tests/test_02_auth.sh:72`, `repo/API_tests/test_02_auth.sh:94` | Status assertions on login/session/logout | basically covered | Lockout timer and idle-timeout behavior not API-verified | Add API tests for 5-failure lock, 15-minute unlock, idle timeout expiry |
| Password policy boundaries | `repo/src/auth/password_tests.rs:20`, `repo/src/auth/lockout_tests.rs:23` | unit policy assertions | basically covered | No integration evidence that policy is uniformly enforced on all creation paths | Add API negative tests for each role create path and malformed payload matrix |
| Step-up required actions | none in `API_tests` | N/A | missing | No pre/post stepup assertions for export/report actions | Add tests: export/report returns step-up-required before `/api/auth/stepup`, succeeds after valid stepup |
| Object-level auth for submissions/reviews/disputes | limited (`repo/API_tests/test_03_users.sh:48`) | user profile 403 only | insufficient | No cross-user denial tests for `/api/inspection/submissions/{id}`, `/api/reviews/{id}`, `/api/disputes/{id}` | Add cross-user and cross-role 403/404 cases for each object endpoint |
| Bulk/messaging privileged operations | `repo/API_tests/test_09_bulk_data.sh:92`, `repo/API_tests/test_08_messaging.sh:83` | some forbidden checks | basically covered | Missing negative tests for trigger listing/fire and payload mutation edge roles | Add explicit 403 tests for non-admin on trigger list/fire/export/mark-failed |
| KB fuzzy ranking by weights/misspellings | `repo/API_tests/test_04_kb.sh:72` | contains-based search check | insufficient | No deterministic ranking-weight assertions | Add fixture-driven assertions for exact vs prefix vs fuzzy vs alias ordering |
| Scheduling edge semantics (miss/makeup/overdue) | `repo/src/scheduling/lifecycle_tests.rs:14` | date arithmetic checks | insufficient | Lacks integration with real scheduler state transitions | Add unit+API tests that exercise overdue/makeup transitions through route/db flows |
| Review consistency 422/409 flows | `repo/src/review/coi_tests.rs:54`, `repo/src/review/coi_tests.rs:69` | consistency function outputs only | insufficient | No API submit assertions for 422/409/acknowledged warning pass | Add API tests on `/api/reviews/assignments/{id}/submit` for each branch |
| Payload export/retry/failure lifecycle | `repo/src/messaging/payload_lifecycle_tests.rs:20` | template rendering only | insufficient | No API/state tests for exported->delivered->retry->failed lifecycle | Add integration tests covering payload status transitions and delivery log entries |

### 8.3 Security Coverage Audit
- authentication: **basically covered** for core login/logout/session, not for lockout/timeout timing semantics.
- route authorization: **insufficient** due limited negative coverage on newer dispute/messaging/bulk edges.
- object-level authorization: **insufficient**; current API tests do not meaningfully cover sensitive object ownership constraints.
- tenant / data isolation: **insufficient**; isolation checks are not comprehensively tested.
- admin / internal protection: **basically covered** for several modules, but still incomplete for step-up and some privileged reads.

### 8.4 Final Coverage Judgment
**Partial Pass**

Major risks covered: baseline auth flow, broad module smoke tests, some role-forbidden checks.

Major risks still under-covered: step-up enforcement, object-level authorization for sensitive entities, deterministic KB ranking behavior, and full payload/scheduling lifecycle transitions.

## 9. Final Notes
- This report is static-only and evidence-traceable.
- Significant progress is visible versus the prior self-test (README, app shell, dispute module, improved endpoint guards).
- Remaining acceptance blockers are primarily security hardening and high-risk test coverage depth.
