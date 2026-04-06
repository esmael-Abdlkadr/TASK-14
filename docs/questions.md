# CivicSort Operations Platform — Business Logic Questions Log

---

## 1. Fuzzy Search Weight Configuration and Ownership

**Question:** The waste-sorting knowledge base supports fuzzy-match search ranked by configurable weights. Who is responsible for managing these weights — Operations Admins, Department Managers, or a dedicated role? What does the configuration interface look like, and are weight changes applied immediately or staged?

**My Understanding:** Weight configuration is a system-level concern that affects search quality for all users. It should be restricted to Operations Admins or Department Managers who understand the domain well enough to tune relevance. Changes should not take effect mid-session without warning since inspectors relying on search results could see unexpected shifts.

**Solution:** Restrict fuzzy-match weight configuration to the Operations Admin role via the Admin Console. Expose a settings panel where admins can adjust weights for exact match, alias match, and misspelling tolerance on a 0.0–1.0 scale. Changes are saved as a new weight profile version and take effect on the next user session refresh, not retroactively. Each weight change is recorded in the audit log with the previous and new values.

---

## 2. Knowledge Base Versioning — Rule Changes Mid-Inspection Cycle

**Question:** When waste-sorting rules change (new region rules, updated disposal requirements), what happens to inspections that are already in progress or scheduled under the previous rule version? Are they evaluated against the old rules or the new ones?

**My Understanding:** Inspections already submitted should be evaluated against the rule version that was active at the time of submission. Inspections that are in-progress but not yet submitted present an ambiguity — the inspector may have started work under one set of rules and would be unfairly judged under new ones.

**Solution:** Each inspection submission captures a snapshot of the applicable rule version ID at the time the task is started (not submitted). Rule changes publish a new version but do not retroactively alter in-flight inspections. The review workspace displays which rule version was active for the inspection. If an inspector has not yet started a scheduled task when rules change, the task picks up the latest version. A banner notification alerts inspectors when rules have changed for items relevant to their upcoming tasks.

---

## 3. Task Template Make-Up Rules — Cascading Missed Windows

**Question:** The make-up rule gives inspectors 48 hours to complete a missed task before it becomes overdue. What happens if the 48-hour make-up window is also missed? Does a second make-up window open, does the task immediately escalate, or is it simply marked overdue with no further recovery?

**My Understanding:** Allowing cascading make-up windows would let tasks slide indefinitely. A single make-up opportunity is likely the intended design, after which the task should be marked overdue and escalated.

**Solution:** Each task instance gets at most one make-up window of 48 hours. If the make-up window expires without completion, the task is marked `overdue` with no further make-up opportunity. An overdue notification is sent to the assigned inspector and their Reviewer. The Operations Admin can manually reassign the overdue task or create a new ad-hoc task if the work still needs to be performed. The overdue instance counts against the inspector's fault tolerance allowance.

---

## 4. Fault Tolerance Reset Period — Rolling Window vs. Calendar Month

**Question:** The fault tolerance allows 1 missed task per 30 days. Does "30 days" mean a rolling 30-day window calculated from each missed task, or does it reset on a fixed calendar-month boundary?

**My Understanding:** A rolling window is fairer and more precise, but a calendar-month boundary is simpler to explain and administer. The choice affects edge cases — for example, missing a task on the 29th and another on the 2nd of the next month would be two misses in a rolling window but split across calendar months.

**Solution:** Use a rolling 30-day window. When evaluating whether a new missed task exceeds the tolerance, count the number of missed tasks for that inspector in the 30 calendar days preceding the current miss. If the count (including the current miss) exceeds 1, flag the inspector as exceeding fault tolerance and notify the Reviewer and Operations Admin. The rolling window is recalculated on each new miss, not cached, to ensure accuracy.

---

## 5. Inspection Submission — Post-Submission Editing

**Question:** Once a Field Inspector submits an inspection, can they edit or retract it before a Reviewer picks it up? What about after review has started?

**My Understanding:** Allowing edits after submission undermines the integrity of the review process. However, a brief grace period for correcting obvious mistakes (wrong photo attached, typo in notes) before review begins could be practical without compromising trust.

**Solution:** Allow the submitting inspector to retract and edit an inspection only while its status is `submitted` and no Reviewer has claimed it. Once a Reviewer claims the inspection (status moves to `in_review`), the submission is locked. If the inspector needs to correct something after review has begun, they must request a revision through the dispute workflow, which the Reviewer can approve or deny. All retractions and edits during the grace window are logged in the audit trail with timestamps and diffs.

---

## 6. Blind Review — Small Team Anonymity Challenges

**Question:** Blind review is supported to reduce bias, but in small sanitation teams (e.g., 3–4 inspectors per district), Reviewers may easily identify submitters from writing style, assigned areas, or task context. How should the system handle cases where true anonymity is impractical?

**My Understanding:** The system cannot guarantee anonymity through UI masking alone when teams are small. The goal should be reducing casual bias rather than achieving absolute anonymity. Additional procedural controls may be needed.

**Solution:** When blind review is enabled, the system strips inspector name, ID, and assigned district from the review workspace. However, if the team size for a given task group falls below a configurable threshold (default: 5 inspectors), the system displays a warning to the Operations Admin that blind review effectiveness is limited. In such cases, the system recommends cross-district review assignment — routing inspections to Reviewers outside the submitting inspector's district. The Admin Console includes a toggle to enforce cross-district assignment as a requirement rather than a recommendation. The blind review audit record notes whether the small-team warning was active.

---

## 7. Scorecard Consistency Checks — Defining "Contradictory" Ratings

**Question:** The review workspace flags contradictory ratings on scorecards. What specific conditions constitute a contradiction? Is it purely numerical (e.g., high score on cleanliness but low on contamination), or does it also consider comment sentiment versus rating values?

**My Understanding:** Automated contradiction detection should focus on objective, rule-based checks rather than natural language sentiment analysis, which would be complex and error-prone in an offline system. The system should flag structurally contradictory rating combinations based on preconfigured dimension relationships.

**Solution:** Define contradiction rules as configurable dimension pairs with expected correlation direction. For example, if "Bin Cleanliness" is rated 5/5 but "Contamination Level" is rated 4/5 (high contamination), flag a contradiction because these dimensions are inversely correlated. Contradiction rules are managed by the Operations Admin in the scorecard configuration screen. When a Reviewer submits ratings that trigger a contradiction rule, the system displays an inline warning requiring the Reviewer to either adjust the ratings or provide a written justification in the required comment field before the scorecard can be finalized. No sentiment analysis is performed on comments.

---

## 8. Conflict-of-Interest Recusal — Depth of Previous Involvement

**Question:** Reviewers must recuse themselves based on department or previous involvement. How far back does "previous involvement" extend? Does it include any prior interaction with the inspector, only recent reviews, or involvement with the specific inspection location or task?

**My Understanding:** An unbounded lookback (any prior interaction ever) would make assignments nearly impossible in small organizations. A practical scope should be defined that balances integrity with operational feasibility.

**Solution:** Define "previous involvement" as any of the following within the past 90 days (configurable): the Reviewer reviewed another inspection by the same inspector, the Reviewer and inspector share the same department, or the Reviewer was involved in a dispute resolution for the same inspector. The automatic assignment engine checks these conditions and excludes conflicted Reviewers. If no unconflicted Reviewer is available, the system escalates to the Operations Admin for manual assignment with an explicit conflict-of-interest override that is recorded in the audit log. The 90-day window is configurable per organization in the Admin Console.

---

## 9. Disputed Classifications — Initiation and Workflow

**Question:** The system supports disputed waste-sorting classifications, but the requirements do not specify who can initiate a dispute, what the escalation path looks like, or how disputes are resolved. Can inspectors dispute Reviewer decisions? Can residents or external parties submit disputes?

**My Understanding:** Since CivicSort is an internal operations platform for city sanitation teams, disputes should be limited to internal roles. Field Inspectors should be able to dispute a Reviewer's scoring or classification decision, and the resolution should involve a separate Reviewer or a Department Manager to avoid the original Reviewer adjudicating their own decision.

**Solution:** Only Field Inspectors and Reviewers can initiate disputes. An inspector may dispute a review outcome within 5 business days of receiving the review result. The dispute must include a written rationale and optional supporting evidence (photos, rule references). Disputes are routed to a different Reviewer than the original — if blind review is active, the dispute is also blinded. If no alternative Reviewer is available, the dispute escalates to the Department Manager. The dispute workflow has three outcomes: `upheld` (original stands), `overturned` (score revised), or `partial` (specific dimensions revised). All dispute actions, outcomes, and rationale are recorded in the audit log and the inspector is notified of the outcome through the in-app inbox.

---

## 10. Campaign/Promo Expiration — Behavior After End Date

**Question:** The Admin Console supports education campaigns and promos with defined date ranges (e.g., "Spring Cleanup Week 04/15/2026–04/22/2026"). What happens when a campaign expires? Are associated tasks, banners, and knowledge base highlights removed immediately, archived, or left visible in a read-only state?

**My Understanding:** Abrupt removal could confuse inspectors who were mid-task during the campaign. However, leaving expired campaigns fully active indefinitely clutters the interface and may cause inspectors to follow outdated guidance.

**Solution:** When a campaign reaches its end date, the system automatically transitions it to an `expired` status. Expired campaigns are removed from active dashboards, banner rotations, and inspector task queues within 24 hours of expiration (processed by a scheduled background job). Any in-progress inspection tasks linked to the campaign are allowed to complete under the campaign's rules but no new tasks are created. Expired campaigns remain visible in the Admin Console under an "Archived Campaigns" section for reporting purposes. Campaign-specific knowledge base highlights are reverted to their default (non-campaign) state. The Operations Admin can manually extend or reactivate an expired campaign if needed.

---

## 11. Notification Queued Payloads — Retention and Cleanup of Unsent Messages

**Question:** The messaging system generates queued SMS/email/push payload files for manual transfer in the offline environment. How long are unsent payload files retained? What happens if payloads are never transferred — do they accumulate indefinitely, age out, or require manual purging?

**My Understanding:** In an offline system, queued payloads may sit for extended periods if the manual transfer process is delayed or overlooked. Indefinite retention risks filling local storage, but aggressive purging risks losing messages that were merely delayed.

**Solution:** Queued payload files are retained for 30 days (configurable) from their creation timestamp. The system displays a dashboard widget in the Admin Console showing the count and age of pending payloads, with a warning threshold at 7 days unsent. After the retention period expires, payloads are moved to an `expired_payloads` archive directory and excluded from the active transfer queue. Expired payloads are retained in the archive for an additional 60 days before automatic deletion. The Operations Admin can manually purge or re-queue expired payloads from the archive. Each payload status transition (queued, transferred, expired, purged) is written to the audit log.

---

## 12. Device-Account Binding — Shared Device Scenarios

**Question:** The risk control module includes device-account binding. In many sanitation field operations, devices (tablets, phones) are shared across inspectors on different shifts. How does device binding work when a single device is legitimately used by multiple inspectors?

**My Understanding:** Strict one-device-to-one-account binding would break shared-device workflows that are common in municipal field operations. The binding should be flexible enough to support shared devices while still detecting truly anomalous device usage.

**Solution:** Implement a configurable binding mode with two options: `strict` (one device per account) and `pool` (device shared across a defined group). In pool mode, the Operations Admin registers a device as a shared pool device and assigns it to a team or district. Any inspector in that team can authenticate from the pool device without triggering an anomaly alert. In strict mode, a new device triggers step-up verification (re-enter password). The system tracks a maximum of 3 bound devices per account in strict mode (configurable). If a device not in the user's binding list or pool attempts login, the system requires step-up verification and notifies the Operations Admin. All device binding changes are recorded in the audit log.

---

## 13. Anomalous Login Detection — Definition in an Offline System

**Question:** The risk control module mentions anomalous login detection. In a fully offline system with no internet connectivity, traditional signals like IP geolocation and VPN detection are unavailable. What constitutes an "anomalous" login in this context?

**My Understanding:** Without network-layer signals, anomaly detection must rely on local behavioral patterns — device identity, login timing, failed attempt history, and account usage patterns. The definition of "anomalous" needs to be scoped to what is observable locally.

**Solution:** Define anomalous login conditions based on locally observable signals: login from an unrecognized device (not in the user's device binding list or pool), login outside the user's historical time-of-day pattern (e.g., a user who always logs in between 7–9 AM suddenly logging in at 11 PM), login immediately following a lockout expiration (potential brute-force continuation), and concurrent active sessions from different devices for the same account. Each condition generates a risk score. If the cumulative risk score exceeds a configurable threshold, the system enforces step-up verification and creates an alert visible to the Operations Admin. Detection thresholds and enabled signals are configurable in the Admin Console.

---

## 14. Step-Up Verification Scope — Exhaustive List of Protected Actions

**Question:** The requirements mention step-up verification (re-enter password) for critical actions, listing exports, rule rollbacks, and result publication as examples. Is this an exhaustive list, or are there additional actions that require step-up verification? How is the list maintained?

**My Understanding:** The listed actions are examples, not an exhaustive enumeration. The full scope should cover any action where unauthorized execution would cause significant operational, data-integrity, or compliance harm. The list should be configurable rather than hardcoded.

**Solution:** Define a default set of step-up-protected actions: audit log export, knowledge base rule rollback, inspection result publication, user account creation or role change, campaign activation or extension, scorecard configuration changes, device binding overrides, bulk data operations (import/export), and password resets for other users. Store this list as a configurable policy in the Admin Console, editable by the Operations Admin. Each protected action checks whether the user's last password entry was within the step-up validity window (default: 5 minutes). If expired, the user must re-enter their password before proceeding. The step-up verification event (success or failure) is recorded in the audit log. Department Managers can request additions to the protected actions list through the Operations Admin.

---

## 15. Audit Log Export — Access Control and Meta-Auditing

**Question:** The audit log is exportable to CSV/PDF offline. Who has permission to export the audit log, and is the export action itself recorded in the audit log? Could a malicious admin export and then tamper with the export to hide their actions?

**My Understanding:** Audit log export is a sensitive operation because it produces an offline copy of compliance-critical data outside the system's control. Access should be tightly restricted, the export itself must be audited, and the exported file should include integrity verification to detect tampering.

**Solution:** Restrict audit log export to the Operations Admin and Department Manager roles. Every export action is itself recorded as an immutable entry in the audit log, capturing the exporting user, timestamp, date range of exported records, export format (CSV/PDF), and record count. This creates a meta-audit trail — even if someone exports the log, the fact that they did so is permanently recorded. Each exported file includes a SHA-256 checksum in the file footer (for PDF) or as a companion `.sha256` file (for CSV), computed from the file contents at generation time. The checksum and the export record in the audit log can be cross-referenced to verify that an export has not been modified after generation. Export requires step-up verification as defined in the protected actions policy.
