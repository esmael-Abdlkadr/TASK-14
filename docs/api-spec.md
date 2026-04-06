# OmniStock Warehouse & Catalog Management System — API Specification

**Version:** 1.0.0
**Last Updated:** 2026-04-06
**Architecture:** On-premise, fully offline. React frontend consuming REST API from Express.js backend with PostgreSQL database.

---

## Table of Contents

1. [General Conventions](#1-general-conventions)
2. [Authentication & Sessions](#2-authentication--sessions)
3. [Users & Roles (Admin)](#3-users--roles-admin)
4. [Warehouses, Zones & Bins](#4-warehouses-zones--bins)
5. [Inventory & Stock](#5-inventory--stock)
6. [Items & Barcode](#6-items--barcode)
7. [Global Search & Saved Views](#7-global-search--saved-views)
8. [Bulk Import/Export](#8-bulk-importexport)
9. [Product Content — Reviews](#9-product-content--reviews)
10. [Product Content — Q&A](#10-product-content--qa)
11. [Favorites & Browsing History](#11-favorites--browsing-history)
12. [Moderation & Abuse Reports](#12-moderation--abuse-reports)
13. [Notifications (In-App Inbox)](#13-notifications-in-app-inbox)
14. [Integration Clients (Admin)](#14-integration-clients-admin)
15. [Webhooks](#15-webhooks)
16. [Audit Log](#16-audit-log)
17. [Batch Jobs (Admin)](#17-batch-jobs-admin)
18. [Dashboard & Metrics](#18-dashboard--metrics)

---

## 1. General Conventions

### Base URL

```
https://<host>/api
```

### Content Type

All requests and responses use `application/json` unless otherwise noted (file uploads use `multipart/form-data`).

### Authentication

All endpoints except `POST /api/auth/login` and `POST /api/auth/captcha` require a valid session cookie (`omnisession`). The cookie is HttpOnly, Secure, SameSite=Strict.

### User Roles

| Role              | Description                                                        |
|-------------------|--------------------------------------------------------------------|
| Warehouse Clerk   | Receive, move, pick, adjust inventory. View items and stock.       |
| Catalog Editor    | Manage item catalog, product content, reviews, Q&A.                |
| Moderator         | Moderate reviews, Q&A, handle abuse reports.                       |
| Manager           | All Clerk and Editor permissions plus reports, dashboards, exports. |
| Administrator     | Full system access including user/role management and integrations. |

### Soft Delete

All entities use a `deleted_at` timestamp. `DELETE` operations set `deleted_at` rather than physically removing rows. Soft-deleted records are excluded from standard queries unless `?include_deleted=true` is passed (Administrator only).

### Pagination

List endpoints support cursor-based pagination by default.

| Parameter  | Type    | Default | Description                                      |
|------------|---------|---------|--------------------------------------------------|
| `limit`    | integer | 25      | Number of records per page. Max 100.             |
| `cursor`   | string  | —       | Opaque cursor from a previous response.          |
| `sort`     | string  | varies  | Sort field. Prefix with `-` for descending.      |

Paginated responses use this envelope:

```json
{
  "data": [],
  "pagination": {
    "next_cursor": "abc123",
    "has_more": true,
    "total_count": 584
  }
}
```

### Standard Error Format

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Human-readable description.",
    "details": [
      { "field": "email", "reason": "Must be a valid email address." }
    ]
  }
}
```

### Common HTTP Status Codes

| Code | Meaning                                              |
|------|------------------------------------------------------|
| 200  | Success                                              |
| 201  | Created                                              |
| 204  | No Content (successful delete / no body)             |
| 400  | Bad Request / Validation Error                       |
| 401  | Unauthenticated — session missing or expired         |
| 403  | Forbidden — insufficient permissions                 |
| 404  | Not Found                                            |
| 409  | Conflict (duplicate barcode, concurrent edit, etc.)  |
| 422  | Unprocessable Entity (business rule violation)       |
| 429  | Rate Limited                                         |
| 500  | Internal Server Error                                |

### Rate Limiting

- **Interactive users:** No hard rate limit (protected by session and CAPTCHA).
- **Integration clients:** 120 requests/minute per client. Exceeding returns `429` with `Retry-After` header (seconds).

### Inventory Availability Formula

```
available = on_hand - reserved - allocated
```

---

## 2. Authentication & Sessions

### POST /api/auth/login

Authenticate a user and create a session.

**Required Role:** None (public).

**Request Body:**

| Field      | Type   | Required | Description                                        |
|------------|--------|----------|----------------------------------------------------|
| `username` | string | Yes      | The user's login name.                             |
| `password` | string | Yes      | The user's password.                               |
| `captcha`  | string | Cond.    | Required after 3 consecutive failed login attempts.|

**Response — 200 OK:**

```json
{
  "data": {
    "user_id": "uuid",
    "username": "jdoe",
    "display_name": "Jane Doe",
    "roles": ["warehouse_clerk"],
    "permissions": ["inventory.receive", "inventory.move"],
    "session_expires_at": "2026-04-06T14:30:00Z"
  }
}
```

Sets `omnisession` HttpOnly cookie.

**Error Codes:**

| Code | Scenario                                                                 |
|------|--------------------------------------------------------------------------|
| 401  | Invalid credentials.                                                     |
| 403  | Account locked. Response includes `locked_until` ISO timestamp.          |
| 422  | CAPTCHA required but not provided, or CAPTCHA invalid.                   |

**Notes:**
- After 7 consecutive failed attempts the account is locked for 15 minutes.
- After 3 consecutive failed attempts the server requires a CAPTCHA token (see `POST /api/auth/captcha`).
- Failed attempt counter resets on successful login.

---

### POST /api/auth/logout

Invalidate the current session.

**Required Role:** Any authenticated user.

**Request Body:** None.

**Response — 204 No Content.**

Clears the `omnisession` cookie.

---

### POST /api/auth/captcha

Generate an SVG CAPTCHA challenge (via `svg-captcha`).

**Required Role:** None (public).

**Request Body:** None.

**Response — 200 OK:**

```json
{
  "data": {
    "captcha_id": "uuid",
    "svg": "<svg>...</svg>"
  }
}
```

**Notes:**
- The `captcha_id` must be submitted alongside the text solution in the login request.
- Each CAPTCHA is single-use and expires after 5 minutes.

---

### POST /api/auth/change-password

Change the password for the currently authenticated user.

**Required Role:** Any authenticated user.

**Request Body:**

| Field              | Type   | Required | Description                        |
|--------------------|--------|----------|------------------------------------|
| `current_password` | string | Yes      | The user's current password.       |
| `new_password`     | string | Yes      | The desired new password.          |

**Response — 200 OK:**

```json
{
  "data": {
    "message": "Password changed successfully."
  }
}
```

**Error Codes:**

| Code | Scenario                                                     |
|------|--------------------------------------------------------------|
| 400  | New password does not meet complexity requirements.          |
| 401  | Current password is incorrect.                               |
| 422  | New password matches one of the last 5 passwords.            |

**Notes:**
- Password history stores the last 5 Argon2id hashes. Reuse of any is rejected.
- Minimum 12 characters, at least one uppercase, one lowercase, one digit, one special character.
- Changing the password invalidates all other active sessions for the user.

---

### GET /api/auth/session

Return the current session details and user profile.

**Required Role:** Any authenticated user.

**Response — 200 OK:**

```json
{
  "data": {
    "user_id": "uuid",
    "username": "jdoe",
    "display_name": "Jane Doe",
    "roles": ["warehouse_clerk"],
    "permissions": ["inventory.receive", "inventory.move"],
    "session_expires_at": "2026-04-06T14:30:00Z"
  }
}
```

**Error Codes:**

| Code | Scenario                    |
|------|-----------------------------|
| 401  | No valid session.           |

---

## 3. Users & Roles (Admin)

### POST /api/users

Create a new user.

**Required Role:** Administrator.

**Request Body:**

| Field          | Type     | Required | Description                              |
|----------------|----------|----------|------------------------------------------|
| `username`     | string   | Yes      | Unique login name. 3-64 characters.      |
| `email`        | string   | Yes      | Unique email address.                    |
| `display_name` | string   | Yes      | Full display name.                       |
| `password`     | string   | Yes      | Initial password (must meet complexity). |
| `role_ids`     | string[] | Yes      | Array of role UUIDs to assign.           |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "username": "jdoe",
    "email": "jdoe@example.local",
    "display_name": "Jane Doe",
    "roles": [{ "id": "uuid", "name": "warehouse_clerk" }],
    "is_locked": false,
    "created_at": "2026-04-06T10:00:00Z",
    "updated_at": "2026-04-06T10:00:00Z",
    "deleted_at": null
  }
}
```

**Error Codes:**

| Code | Scenario                              |
|------|---------------------------------------|
| 400  | Validation error.                     |
| 409  | Username or email already exists.     |

---

### GET /api/users

List all users with filtering and pagination.

**Required Role:** Administrator.

**Query Parameters:**

| Parameter        | Type    | Default | Description                                  |
|------------------|---------|---------|----------------------------------------------|
| `q`              | string  | —       | Search by username, display name, or email.  |
| `role_id`        | string  | —       | Filter by role UUID.                         |
| `is_locked`      | boolean | —       | Filter locked/unlocked accounts.             |
| `include_deleted`| boolean | false   | Include soft-deleted users.                  |
| `limit`          | integer | 25      | Page size.                                   |
| `cursor`         | string  | —       | Pagination cursor.                           |
| `sort`           | string  | `username` | Sort field (`username`, `created_at`, `display_name`). |

**Response — 200 OK:** Paginated list of user objects.

---

### GET /api/users/:id

Get a single user by ID.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | User UUID.      |

**Response — 200 OK:** Single user object.

**Error Codes:**

| Code | Scenario            |
|------|---------------------|
| 404  | User not found.     |

---

### PUT /api/users/:id

Update user details.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | User UUID.      |

**Request Body:**

| Field          | Type     | Required | Description                  |
|----------------|----------|----------|------------------------------|
| `email`        | string   | No       | Updated email.               |
| `display_name` | string   | No       | Updated display name.        |
| `role_ids`     | string[] | No       | Replacement set of role UUIDs.|

**Response — 200 OK:** Updated user object.

**Error Codes:**

| Code | Scenario                          |
|------|-----------------------------------|
| 404  | User not found.                   |
| 409  | Email already exists.             |

---

### DELETE /api/users/:id

Soft-delete a user.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | User UUID.      |

**Response — 204 No Content.**

**Notes:**
- Soft-deletes the user (sets `deleted_at`).
- Invalidates all active sessions for the user.
- The Administrator cannot delete their own account.

---

### GET /api/users/:id/audit-log

Retrieve the audit trail for a specific user.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | User UUID.      |

**Query Parameters:**

| Parameter    | Type   | Default | Description                                   |
|--------------|--------|---------|-----------------------------------------------|
| `action`     | string | —       | Filter by action type (e.g., `login`, `password_change`). |
| `from`       | string | —       | ISO 8601 start timestamp.                     |
| `to`         | string | —       | ISO 8601 end timestamp.                       |
| `limit`      | integer| 25      | Page size.                                    |
| `cursor`     | string | —       | Pagination cursor.                            |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "id": "uuid",
      "user_id": "uuid",
      "action": "login",
      "ip_address": "192.168.1.42",
      "details": {},
      "created_at": "2026-04-06T09:15:00Z"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 120 }
}
```

---

### POST /api/users/:id/lock

Manually lock a user account.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | User UUID.      |

**Request Body:**

| Field    | Type   | Required | Description                             |
|----------|--------|----------|-----------------------------------------|
| `reason` | string | No       | Human-readable reason for the lock.     |

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "is_locked": true,
    "locked_until": null,
    "lock_reason": "Policy violation."
  }
}
```

**Notes:**
- Manual locks have no `locked_until` — they persist until explicitly unlocked.
- Invalidates all active sessions for the user.

---

### POST /api/users/:id/unlock

Unlock a user account.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | User UUID.      |

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "is_locked": false,
    "locked_until": null,
    "lock_reason": null
  }
}
```

**Notes:**
- Resets the failed login attempt counter to 0.

---

### POST /api/roles

Create a new role.

**Required Role:** Administrator.

**Request Body:**

| Field          | Type     | Required | Description                         |
|----------------|----------|----------|-------------------------------------|
| `name`         | string   | Yes      | Unique role name.                   |
| `description`  | string   | No       | Human-readable description.         |
| `permissions`  | string[] | No       | Initial set of permission slugs.    |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "name": "warehouse_clerk",
    "description": "Can receive, move, pick, and adjust inventory.",
    "permissions": ["inventory.receive", "inventory.move", "inventory.pick", "inventory.adjust"],
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

---

### GET /api/roles

List all roles.

**Required Role:** Administrator.

**Query Parameters:**

| Parameter | Type    | Default | Description   |
|-----------|---------|---------|---------------|
| `limit`   | integer | 25      | Page size.    |
| `cursor`  | string  | —       | Cursor.       |

**Response — 200 OK:** Paginated list of role objects.

---

### GET /api/roles/:id

Get a single role.

**Required Role:** Administrator.

**Response — 200 OK:** Single role object with permissions array.

---

### PUT /api/roles/:id

Update role name and description.

**Required Role:** Administrator.

**Request Body:**

| Field         | Type   | Required | Description                 |
|---------------|--------|----------|-----------------------------|
| `name`        | string | No       | Updated role name.          |
| `description` | string | No       | Updated description.        |

**Response — 200 OK:** Updated role object.

---

### DELETE /api/roles/:id

Soft-delete a role.

**Required Role:** Administrator.

**Response — 204 No Content.**

**Notes:**
- Cannot delete a role that is currently assigned to users. Returns `422` with a list of affected user IDs.

---

### PUT /api/roles/:id/permissions

Replace the full permission set for a role.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | Role UUID.      |

**Request Body:**

| Field         | Type     | Required | Description                                |
|---------------|----------|----------|--------------------------------------------|
| `permissions` | string[] | Yes      | Complete list of permission slugs to assign.|

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "name": "warehouse_clerk",
    "permissions": ["inventory.receive", "inventory.move"]
  }
}
```

**Notes:**
- This is a full replacement, not a merge. Any permission not in the array is revoked.
- Changes take effect on the user's next request (permission check is per-request).

---

## 4. Warehouses, Zones & Bins

### POST /api/warehouses

Create a warehouse.

**Required Role:** Administrator, Manager.

**Request Body:**

| Field       | Type   | Required | Description                      |
|-------------|--------|----------|----------------------------------|
| `name`      | string | Yes      | Unique warehouse name.           |
| `code`      | string | Yes      | Short unique code (e.g., `WH-A`).|
| `address`   | object | No       | Address object (street, city, etc.).|

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "name": "Main Warehouse",
    "code": "WH-A",
    "address": { "street": "123 Industrial Dr", "city": "Springfield", "state": "IL", "zip": "62704" },
    "created_at": "2026-04-06T10:00:00Z",
    "updated_at": "2026-04-06T10:00:00Z",
    "deleted_at": null
  }
}
```

---

### GET /api/warehouses

List all warehouses.

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter | Type    | Default | Description                     |
|-----------|---------|---------|---------------------------------|
| `q`       | string  | —       | Search by name or code.         |
| `limit`   | integer | 25      | Page size.                      |
| `cursor`  | string  | —       | Pagination cursor.              |
| `sort`    | string  | `name`  | Sort field (`name`, `code`, `created_at`). |

**Response — 200 OK:** Paginated list of warehouse objects.

---

### GET /api/warehouses/:id

Get a single warehouse.

**Required Role:** Any authenticated user.

**Path Parameters:**

| Parameter | Type   | Description      |
|-----------|--------|------------------|
| `id`      | string | Warehouse UUID.  |

**Response — 200 OK:** Single warehouse object.

---

### PUT /api/warehouses/:id

Update a warehouse.

**Required Role:** Administrator, Manager.

**Request Body:**

| Field     | Type   | Required | Description               |
|-----------|--------|----------|---------------------------|
| `name`    | string | No       | Updated name.             |
| `code`    | string | No       | Updated code.             |
| `address` | object | No       | Updated address.          |

**Response — 200 OK:** Updated warehouse object.

---

### DELETE /api/warehouses/:id

Soft-delete a warehouse.

**Required Role:** Administrator.

**Response — 204 No Content.**

**Notes:**
- Cannot delete a warehouse that has non-zero inventory. Returns `422`.
- All zones and bins underneath are cascade soft-deleted.

---

### POST /api/warehouses/:id/zones

Create a zone within a warehouse.

**Required Role:** Administrator, Manager.

**Path Parameters:**

| Parameter | Type   | Description      |
|-----------|--------|------------------|
| `id`      | string | Warehouse UUID.  |

**Request Body:**

| Field       | Type   | Required | Description                                            |
|-------------|--------|----------|--------------------------------------------------------|
| `name`      | string | Yes      | Zone name (unique within warehouse).                   |
| `code`      | string | Yes      | Short zone code (e.g., `Z-01`).                        |
| `type`      | string | No       | Zone type: `picking`, `bulk`, `cold`, `hazmat`, `staging`. |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "warehouse_id": "uuid",
    "name": "Zone A - Picking",
    "code": "Z-01",
    "type": "picking",
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

---

### GET /api/warehouses/:id/zones

List zones for a warehouse.

**Required Role:** Any authenticated user.

**Path Parameters:**

| Parameter | Type   | Description      |
|-----------|--------|------------------|
| `id`      | string | Warehouse UUID.  |

**Query Parameters:**

| Parameter | Type    | Default | Description     |
|-----------|---------|---------|-----------------|
| `type`    | string  | —       | Filter by zone type. |
| `limit`   | integer | 25      | Page size.      |
| `cursor`  | string  | —       | Cursor.         |

**Response — 200 OK:** Paginated list of zone objects.

---

### GET /api/warehouses/:id/zones/:zoneId

Get a single zone.

**Required Role:** Any authenticated user.

**Response — 200 OK:** Single zone object.

---

### PUT /api/warehouses/:id/zones/:zoneId

Update a zone.

**Required Role:** Administrator, Manager.

**Request Body:**

| Field  | Type   | Required | Description       |
|--------|--------|----------|-------------------|
| `name` | string | No       | Updated name.     |
| `code` | string | No       | Updated code.     |
| `type` | string | No       | Updated type.     |

**Response — 200 OK:** Updated zone object.

---

### DELETE /api/warehouses/:id/zones/:zoneId

Soft-delete a zone.

**Required Role:** Administrator.

**Response — 204 No Content.**

**Notes:**
- Cannot delete a zone that has non-empty bins. Returns `422`.

---

### POST /api/warehouses/:warehouseId/zones/:zoneId/bins

Create a bin within a zone.

**Required Role:** Administrator, Manager, Warehouse Clerk.

**Path Parameters:**

| Parameter     | Type   | Description      |
|---------------|--------|------------------|
| `warehouseId` | string | Warehouse UUID.  |
| `zoneId`      | string | Zone UUID.       |

**Request Body:**

| Field      | Type   | Required | Description                                    |
|------------|--------|----------|------------------------------------------------|
| `code`     | string | Yes      | Unique bin code (e.g., `A-01-03-02`).          |
| `type`     | string | No       | Bin type: `shelf`, `pallet`, `floor`, `drawer`. |
| `capacity` | object | No       | `{ "max_weight_kg": 500, "max_volume_m3": 2 }`.|

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "zone_id": "uuid",
    "warehouse_id": "uuid",
    "code": "A-01-03-02",
    "type": "shelf",
    "capacity": { "max_weight_kg": 500, "max_volume_m3": 2 },
    "is_enabled": true,
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

---

### GET /api/warehouses/:warehouseId/zones/:zoneId/bins

List bins in a zone.

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter    | Type    | Default | Description                     |
|--------------|---------|---------|---------------------------------|
| `type`       | string  | —       | Filter by bin type.             |
| `is_enabled` | boolean | —       | Filter enabled/disabled bins.   |
| `limit`      | integer | 25      | Page size.                      |
| `cursor`     | string  | —       | Cursor.                         |

**Response — 200 OK:** Paginated list of bin objects.

---

### GET /api/warehouses/:warehouseId/zones/:zoneId/bins/:binId

Get a single bin.

**Required Role:** Any authenticated user.

**Response — 200 OK:** Single bin object including current inventory summary.

---

### PUT /api/warehouses/:warehouseId/zones/:zoneId/bins/:binId

Update a bin.

**Required Role:** Administrator, Manager.

**Request Body:**

| Field      | Type   | Required | Description            |
|------------|--------|----------|------------------------|
| `code`     | string | No       | Updated bin code.      |
| `type`     | string | No       | Updated bin type.      |
| `capacity` | object | No       | Updated capacity.      |

**Response — 200 OK:** Updated bin object.

---

### DELETE /api/warehouses/:warehouseId/zones/:zoneId/bins/:binId

Soft-delete a bin.

**Required Role:** Administrator.

**Response — 204 No Content.**

**Notes:**
- Cannot delete a bin that contains inventory. Returns `422`.

---

### PATCH /api/bins/:id/enable

Re-enable a previously disabled bin.

**Required Role:** Administrator, Manager.

**Path Parameters:**

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `id`      | string | Bin UUID.   |

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "is_enabled": true
  }
}
```

---

### PATCH /api/bins/:id/disable

Disable a bin. Flags all inventory for relocation and blocks new put-away into this bin.

**Required Role:** Administrator, Manager.

**Path Parameters:**

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `id`      | string | Bin UUID.   |

**Request Body:**

| Field    | Type   | Required | Description                              |
|----------|--------|----------|------------------------------------------|
| `reason` | string | No       | Reason for disabling (e.g., maintenance).|

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "is_enabled": false,
    "disabled_reason": "Shelf damage — maintenance required.",
    "inventory_flagged_for_relocation": 12
  }
}
```

**Notes:**
- Disabling a bin creates a notification for all Warehouse Clerks assigned to that warehouse.
- Existing inventory is flagged for relocation but not automatically moved.
- Any `POST /api/inventory/receive` or `POST /api/inventory/move` targeting a disabled bin returns `422`.

---

### GET /api/bins/:id/timeline

Get the full activity timeline for a bin.

**Required Role:** Any authenticated user.

**Path Parameters:**

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `id`      | string | Bin UUID.   |

**Query Parameters:**

| Parameter | Type    | Default | Description                |
|-----------|---------|---------|----------------------------|
| `from`    | string  | —       | ISO 8601 start timestamp.  |
| `to`      | string  | —       | ISO 8601 end timestamp.    |
| `limit`   | integer | 50      | Page size.                 |
| `cursor`  | string  | —       | Cursor.                    |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "id": "uuid",
      "event_type": "receive",
      "item_id": "uuid",
      "quantity": 50,
      "performed_by": "uuid",
      "created_at": "2026-04-06T09:30:00Z",
      "details": {}
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 340 }
}
```

---

## 5. Inventory & Stock

### GET /api/inventory

Search and list inventory across all warehouses.

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter      | Type    | Default | Description                                            |
|----------------|---------|---------|--------------------------------------------------------|
| `q`            | string  | —       | Full-text search across item name, SKU, barcode.       |
| `warehouse_id` | string  | —       | Filter by warehouse UUID.                              |
| `zone_id`      | string  | —       | Filter by zone UUID.                                   |
| `bin_id`       | string  | —       | Filter by bin UUID.                                    |
| `item_id`      | string  | —       | Filter by item UUID.                                   |
| `lot_number`   | string  | —       | Filter by lot number.                                  |
| `below_reorder`| boolean | —       | Only show items below reorder point.                   |
| `limit`        | integer | 25      | Page size.                                             |
| `cursor`       | string  | —       | Cursor.                                                |
| `sort`         | string  | `item_name` | Sort field (`item_name`, `on_hand`, `available`, `updated_at`). |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "id": "uuid",
      "item_id": "uuid",
      "item_name": "Widget A",
      "sku": "WGT-A-001",
      "barcode": "0012345678905",
      "warehouse_id": "uuid",
      "warehouse_name": "Main Warehouse",
      "zone_id": "uuid",
      "bin_id": "uuid",
      "bin_code": "A-01-03-02",
      "lot_number": "LOT-2026-04",
      "on_hand": 100,
      "reserved": 10,
      "allocated": 5,
      "available": 85,
      "unit": "ea",
      "updated_at": "2026-04-06T10:00:00Z"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 1250 }
}
```

---

### POST /api/inventory/receive

Receive stock into a bin (put-away).

**Required Role:** Warehouse Clerk, Manager, Administrator.

**Request Body:**

| Field         | Type    | Required | Description                                     |
|---------------|---------|----------|-------------------------------------------------|
| `item_id`     | string  | Yes      | Item UUID.                                      |
| `bin_id`      | string  | Yes      | Destination bin UUID.                            |
| `quantity`    | integer | Yes      | Quantity received. Must be > 0.                 |
| `lot_number`  | string  | No       | Lot/batch number.                               |
| `expiry_date` | string  | No       | ISO 8601 date for perishable items.             |
| `reference`   | string  | No       | External reference (PO number, ASN, etc.).      |
| `notes`       | string  | No       | Free-text notes.                                |

**Response — 201 Created:**

```json
{
  "data": {
    "transaction_id": "uuid",
    "type": "receive",
    "item_id": "uuid",
    "bin_id": "uuid",
    "quantity": 50,
    "new_on_hand": 150,
    "created_at": "2026-04-06T10:05:00Z"
  }
}
```

**Error Codes:**

| Code | Scenario                                                  |
|------|-----------------------------------------------------------|
| 422  | Bin is disabled — new put-away blocked.                   |
| 422  | Bin capacity would be exceeded.                           |
| 404  | Item or bin not found.                                    |

**Notes:**
- Creates an entry in the stock ledger.
- Generates an audit log entry.

---

### POST /api/inventory/move

Move stock between bins.

**Required Role:** Warehouse Clerk, Manager, Administrator.

**Request Body:**

| Field           | Type    | Required | Description                            |
|-----------------|---------|----------|----------------------------------------|
| `item_id`       | string  | Yes      | Item UUID.                             |
| `from_bin_id`   | string  | Yes      | Source bin UUID.                        |
| `to_bin_id`     | string  | Yes      | Destination bin UUID.                   |
| `quantity`      | integer | Yes      | Quantity to move. Must be > 0.         |
| `lot_number`    | string  | No       | Lot number (must match source stock).  |
| `reason`        | string  | No       | Reason for the move.                   |

**Response — 200 OK:**

```json
{
  "data": {
    "transaction_id": "uuid",
    "type": "move",
    "item_id": "uuid",
    "from_bin_id": "uuid",
    "to_bin_id": "uuid",
    "quantity": 20,
    "from_bin_new_on_hand": 80,
    "to_bin_new_on_hand": 120,
    "created_at": "2026-04-06T10:10:00Z"
  }
}
```

**Error Codes:**

| Code | Scenario                                                       |
|------|----------------------------------------------------------------|
| 422  | Insufficient stock in source bin.                              |
| 422  | Destination bin is disabled.                                   |
| 422  | Moving reserved or allocated stock not permitted without release.|

---

### POST /api/inventory/pick

Pick stock from a bin for order fulfillment.

**Required Role:** Warehouse Clerk, Manager, Administrator.

**Request Body:**

| Field        | Type    | Required | Description                                |
|--------------|---------|----------|--------------------------------------------|
| `item_id`    | string  | Yes      | Item UUID.                                 |
| `bin_id`     | string  | Yes      | Bin UUID to pick from.                     |
| `quantity`   | integer | Yes      | Quantity to pick. Must be > 0.             |
| `lot_number` | string  | No       | Lot number.                                |
| `order_ref`  | string  | No       | Associated order reference.                |

**Response — 200 OK:**

```json
{
  "data": {
    "transaction_id": "uuid",
    "type": "pick",
    "item_id": "uuid",
    "bin_id": "uuid",
    "quantity": 5,
    "new_on_hand": 95,
    "created_at": "2026-04-06T10:15:00Z"
  }
}
```

**Error Codes:**

| Code | Scenario                                           |
|------|----------------------------------------------------|
| 422  | Insufficient available stock (on_hand - reserved - allocated < quantity). |

---

### POST /api/inventory/adjust

Perform an inventory adjustment (e.g., cycle count correction, damage write-off).

**Required Role:** Manager, Administrator.

**Request Body:**

| Field           | Type    | Required | Description                                                   |
|-----------------|---------|----------|---------------------------------------------------------------|
| `item_id`       | string  | Yes      | Item UUID.                                                    |
| `bin_id`        | string  | Yes      | Bin UUID.                                                     |
| `adjustment`    | integer | Yes      | Signed quantity (positive = add, negative = subtract).        |
| `reason_code`   | string  | Yes      | One of: `cycle_count`, `damage`, `theft`, `expiry`, `other`.  |
| `notes`         | string  | No       | Explanation notes.                                            |

**Response — 200 OK:**

```json
{
  "data": {
    "transaction_id": "uuid",
    "type": "adjust",
    "item_id": "uuid",
    "bin_id": "uuid",
    "adjustment": -3,
    "new_on_hand": 97,
    "reason_code": "damage",
    "created_at": "2026-04-06T10:20:00Z"
  }
}
```

**Error Codes:**

| Code | Scenario                                             |
|------|------------------------------------------------------|
| 422  | Resulting on_hand would be negative.                 |

**Notes:**
- Adjustments always generate an audit log entry with the reason code.
- Negative adjustments exceeding a configurable threshold trigger a Manager notification.

---

### GET /api/inventory/stock-ledger

Query the immutable stock ledger (all transactions).

**Required Role:** Manager, Administrator.

**Query Parameters:**

| Parameter      | Type    | Default | Description                                              |
|----------------|---------|---------|----------------------------------------------------------|
| `item_id`      | string  | —       | Filter by item.                                          |
| `bin_id`       | string  | —       | Filter by bin.                                           |
| `warehouse_id` | string  | —       | Filter by warehouse.                                     |
| `type`         | string  | —       | Filter by transaction type: `receive`, `move`, `pick`, `adjust`. |
| `from`         | string  | —       | ISO 8601 start timestamp.                                |
| `to`           | string  | —       | ISO 8601 end timestamp.                                  |
| `limit`        | integer | 50      | Page size.                                               |
| `cursor`       | string  | —       | Cursor.                                                  |
| `sort`         | string  | `-created_at` | Sort field.                                        |

**Response — 200 OK:** Paginated list of ledger entry objects with full transaction details.

---

### POST /api/inventory/reserve

Create a stock reservation (hold inventory for a pending order).

**Required Role:** Warehouse Clerk, Manager, Administrator.

**Request Body:**

| Field        | Type    | Required | Description                         |
|--------------|---------|----------|-------------------------------------|
| `item_id`    | string  | Yes      | Item UUID.                          |
| `bin_id`     | string  | Yes      | Bin UUID.                           |
| `quantity`   | integer | Yes      | Quantity to reserve. Must be > 0.   |
| `order_ref`  | string  | Yes      | Associated order reference.         |
| `expires_at` | string  | No       | ISO 8601 reservation expiry. Default: 24 hours. |

**Response — 201 Created:**

```json
{
  "data": {
    "reservation_id": "uuid",
    "item_id": "uuid",
    "bin_id": "uuid",
    "quantity": 10,
    "order_ref": "ORD-20260406-001",
    "expires_at": "2026-04-07T10:25:00Z",
    "new_available": 75,
    "created_at": "2026-04-06T10:25:00Z"
  }
}
```

**Error Codes:**

| Code | Scenario                                             |
|------|------------------------------------------------------|
| 422  | Insufficient available stock for the reservation.    |

**Notes:**
- `available = on_hand - reserved - allocated`. The reservation quantity is checked against `available`.
- Expired reservations are automatically released by a background batch job.

---

### DELETE /api/inventory/reserve/:id

Release (cancel) a stock reservation.

**Required Role:** Warehouse Clerk, Manager, Administrator.

**Path Parameters:**

| Parameter | Type   | Description        |
|-----------|--------|--------------------|
| `id`      | string | Reservation UUID.  |

**Response — 204 No Content.**

**Notes:**
- The reserved quantity is added back to available.
- Creates a stock ledger entry.

---

## 6. Items & Barcode

### POST /api/items

Create a new catalog item.

**Required Role:** Catalog Editor, Manager, Administrator.

**Request Body:**

| Field           | Type     | Required | Description                                    |
|-----------------|----------|----------|------------------------------------------------|
| `name`          | string   | Yes      | Item name. Max 255 characters.                 |
| `sku`           | string   | Yes      | Unique SKU.                                    |
| `barcode`       | string   | Yes      | Globally unique barcode.                        |
| `description`   | string   | No       | Rich-text description.                         |
| `category_id`   | string   | No       | Category UUID.                                 |
| `unit`          | string   | Yes      | Unit of measure (e.g., `ea`, `kg`, `m`).       |
| `weight_kg`     | number   | No       | Item weight in kilograms.                      |
| `dimensions`    | object   | No       | `{ "length_cm": 10, "width_cm": 5, "height_cm": 3 }`. |
| `reorder_point` | integer  | No       | Minimum stock level before reorder alert.      |
| `tags`          | string[] | No       | Freeform tags for search/filter.               |
| `attributes`    | object   | No       | Arbitrary key-value pairs for item properties.  |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "name": "Widget A",
    "sku": "WGT-A-001",
    "barcode": "0012345678905",
    "description": "A high-quality widget.",
    "category_id": "uuid",
    "unit": "ea",
    "weight_kg": 0.25,
    "dimensions": { "length_cm": 10, "width_cm": 5, "height_cm": 3 },
    "reorder_point": 50,
    "tags": ["widget", "industrial"],
    "attributes": { "color": "blue", "material": "steel" },
    "created_at": "2026-04-06T10:00:00Z",
    "updated_at": "2026-04-06T10:00:00Z",
    "deleted_at": null
  }
}
```

**Error Codes:**

| Code | Scenario                                |
|------|-----------------------------------------|
| 409  | SKU or barcode already exists.          |

**Notes:**
- Barcodes are globally unique across all warehouses.

---

### GET /api/items

List and search catalog items.

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter      | Type    | Default    | Description                                  |
|----------------|---------|------------|----------------------------------------------|
| `q`            | string  | —          | Full-text search on name, SKU, barcode, tags.|
| `category_id`  | string  | —          | Filter by category.                          |
| `tags`         | string  | —          | Comma-separated tag filter.                  |
| `has_inventory`| boolean | —          | Only items with current stock.               |
| `limit`        | integer | 25         | Page size.                                   |
| `cursor`       | string  | —          | Cursor.                                      |
| `sort`         | string  | `name`     | Sort field (`name`, `sku`, `created_at`).    |

**Response — 200 OK:** Paginated list of item objects.

---

### GET /api/items/:id

Get a single item with full details.

**Required Role:** Any authenticated user.

**Response — 200 OK:** Single item object.

---

### PUT /api/items/:id

Update an item.

**Required Role:** Catalog Editor, Manager, Administrator.

**Request Body:** Same fields as POST (all optional). `barcode` changes are restricted to Administrator only.

**Response — 200 OK:** Updated item object.

**Error Codes:**

| Code | Scenario                                         |
|------|--------------------------------------------------|
| 409  | SKU or barcode conflict.                         |
| 403  | Non-Administrator attempting to change barcode.  |

---

### DELETE /api/items/:id

Soft-delete an item.

**Required Role:** Administrator.

**Response — 204 No Content.**

**Notes:**
- Cannot delete an item with non-zero inventory across any warehouse. Returns `422`.

---

### GET /api/items/barcode/:code

Look up an item by scanning its barcode.

**Required Role:** Any authenticated user.

**Path Parameters:**

| Parameter | Type   | Description                         |
|-----------|--------|-------------------------------------|
| `code`    | string | The barcode string to look up.      |

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "name": "Widget A",
    "sku": "WGT-A-001",
    "barcode": "0012345678905",
    "inventory_summary": {
      "total_on_hand": 350,
      "total_available": 300,
      "warehouse_breakdown": [
        { "warehouse_id": "uuid", "warehouse_name": "Main Warehouse", "on_hand": 200, "available": 170 },
        { "warehouse_id": "uuid", "warehouse_name": "Overflow", "on_hand": 150, "available": 130 }
      ]
    }
  }
}
```

**Error Codes:**

| Code | Scenario                                    |
|------|---------------------------------------------|
| 404  | No item found for the given barcode.        |

**Notes:**
- Optimized for handheld scanner use — returns item details with a cross-warehouse inventory summary in a single call.
- Barcodes are globally unique so the lookup is unambiguous.

---

### GET /api/items/:id/lots

List lot/batch numbers for an item across all warehouses.

**Required Role:** Any authenticated user.

**Path Parameters:**

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `id`      | string | Item UUID.  |

**Query Parameters:**

| Parameter      | Type    | Default | Description                     |
|----------------|---------|---------|---------------------------------|
| `warehouse_id` | string  | —       | Filter by warehouse.            |
| `expired`      | boolean | —       | Filter expired/non-expired lots.|
| `limit`        | integer | 25      | Page size.                      |
| `cursor`       | string  | —       | Cursor.                         |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "lot_number": "LOT-2026-04",
      "item_id": "uuid",
      "warehouse_id": "uuid",
      "bin_id": "uuid",
      "on_hand": 50,
      "expiry_date": "2027-04-06",
      "received_at": "2026-04-06T10:00:00Z"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": false, "total_count": 3 }
}
```

---

## 7. Global Search & Saved Views

### GET /api/search

Unified search across items, inventory, warehouses, bins, and users.

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter  | Type    | Default | Description                                                               |
|------------|---------|---------|---------------------------------------------------------------------------|
| `q`        | string  | Yes     | Search query string.                                                      |
| `scope`    | string  | `all`   | Comma-separated scopes: `items`, `inventory`, `warehouses`, `bins`, `users`. |
| `limit`    | integer | 10      | Results per scope. Max 25.                                                |

**Response — 200 OK:**

```json
{
  "data": {
    "items": { "results": [], "total_count": 42 },
    "inventory": { "results": [], "total_count": 15 },
    "warehouses": { "results": [], "total_count": 2 },
    "bins": { "results": [], "total_count": 8 },
    "users": { "results": [], "total_count": 0 }
  }
}
```

**Notes:**
- Users scope is only returned if the requester has Administrator role.
- Results are ranked by relevance.

---

### POST /api/saved-views

Create a saved view (private to the current user).

**Required Role:** Any authenticated user.

**Request Body:**

| Field       | Type   | Required | Description                                          |
|-------------|--------|----------|------------------------------------------------------|
| `name`      | string | Yes      | Display name for the saved view.                     |
| `entity`    | string | Yes      | Entity type: `inventory`, `items`, `bins`, `orders`. |
| `filters`   | object | Yes      | Filter configuration (stored as JSON).               |
| `sort`      | string | No       | Sort configuration.                                  |
| `columns`   | string[]| No      | Visible column list.                                 |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "user_id": "uuid",
    "name": "Low Stock - Main WH",
    "entity": "inventory",
    "filters": { "warehouse_id": "uuid", "below_reorder": true },
    "sort": "-available",
    "columns": ["item_name", "sku", "on_hand", "available", "bin_code"],
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

---

### GET /api/saved-views

List saved views for the current user.

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter | Type   | Default | Description                 |
|-----------|--------|---------|-----------------------------|
| `entity`  | string | —       | Filter by entity type.      |

**Response — 200 OK:** List of saved view objects belonging to the current user.

**Notes:**
- Saved views are private per user. Users cannot see or access other users' saved views.

---

### GET /api/saved-views/:id

Get a single saved view.

**Required Role:** Any authenticated user (owner only).

**Response — 200 OK:** Single saved view object.

**Error Codes:**

| Code | Scenario                                              |
|------|-------------------------------------------------------|
| 403  | Attempting to access another user's saved view.       |
| 404  | Saved view not found.                                 |

---

### PUT /api/saved-views/:id

Update a saved view.

**Required Role:** Any authenticated user (owner only).

**Request Body:** Same fields as POST (all optional).

**Response — 200 OK:** Updated saved view object.

---

### DELETE /api/saved-views/:id

Delete a saved view.

**Required Role:** Any authenticated user (owner only).

**Response — 204 No Content.**

---

## 8. Bulk Import/Export

### GET /api/bulk/templates/:type

Download a CSV/XLSX template for bulk import.

**Required Role:** Manager, Administrator.

**Path Parameters:**

| Parameter | Type   | Description                                                  |
|-----------|--------|--------------------------------------------------------------|
| `type`    | string | Template type: `items`, `inventory`, `warehouses`, `bins`.   |

**Query Parameters:**

| Parameter | Type   | Default | Description                      |
|-----------|--------|---------|----------------------------------|
| `format`  | string | `csv`   | File format: `csv` or `xlsx`.    |

**Response — 200 OK:**

Returns the file with appropriate `Content-Type` and `Content-Disposition` headers.

---

### POST /api/bulk/import/validate

Pre-check an import file for errors before committing.

**Required Role:** Manager, Administrator.

**Request Body:** `multipart/form-data`

| Field  | Type   | Required | Description                                                  |
|--------|--------|----------|--------------------------------------------------------------|
| `type` | string | Yes      | Import type: `items`, `inventory`, `warehouses`, `bins`.     |
| `file` | file   | Yes      | CSV or XLSX file.                                            |

**Response — 200 OK:**

```json
{
  "data": {
    "total_rows": 500,
    "valid_rows": 487,
    "error_rows": 13,
    "errors": [
      { "row": 12, "field": "barcode", "message": "Barcode already exists." },
      { "row": 45, "field": "sku", "message": "SKU is required." }
    ],
    "warnings": [
      { "row": 100, "field": "reorder_point", "message": "Reorder point is unusually high (>10000)." }
    ]
  }
}
```

**Notes:**
- Validation is non-destructive — no data is written.
- All barcode uniqueness checks are performed.

---

### POST /api/bulk/import/execute

Execute a bulk import. Creates a batch job for tracking.

**Required Role:** Manager, Administrator.

**Request Body:** `multipart/form-data`

| Field           | Type    | Required | Description                                              |
|-----------------|---------|----------|----------------------------------------------------------|
| `type`          | string  | Yes      | Import type: `items`, `inventory`, `warehouses`, `bins`. |
| `file`          | file    | Yes      | CSV or XLSX file.                                        |
| `skip_errors`   | boolean | No       | If `true`, skip invalid rows and import the rest. Default: `false`. |

**Response — 202 Accepted:**

```json
{
  "data": {
    "job_id": "uuid",
    "status": "pending",
    "type": "items",
    "total_rows": 500,
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

**Notes:**
- The import runs asynchronously as a batch job. Poll `GET /api/bulk/import/:jobId/results` for status.
- A record is created in the `batch_job_runs` table.

---

### GET /api/bulk/import/:jobId/results

Get the results of a bulk import job.

**Required Role:** Manager, Administrator.

**Path Parameters:**

| Parameter | Type   | Description       |
|-----------|--------|-------------------|
| `jobId`   | string | Batch job UUID.   |

**Response — 200 OK:**

```json
{
  "data": {
    "job_id": "uuid",
    "status": "completed",
    "type": "items",
    "total_rows": 500,
    "imported_rows": 487,
    "skipped_rows": 13,
    "errors": [
      { "row": 12, "field": "barcode", "message": "Barcode already exists." }
    ],
    "started_at": "2026-04-06T10:00:05Z",
    "completed_at": "2026-04-06T10:01:30Z"
  }
}
```

**Notes:**
- `status` is one of: `pending`, `running`, `completed`, `failed`.

---

### POST /api/bulk/export

Export data to a downloadable file. Creates a batch job for large datasets.

**Required Role:** Manager, Administrator.

**Request Body:**

| Field     | Type   | Required | Description                                                 |
|-----------|--------|----------|-------------------------------------------------------------|
| `type`    | string | Yes      | Export type: `items`, `inventory`, `stock_ledger`, `users`. |
| `format`  | string | No       | File format: `csv` or `xlsx`. Default: `csv`.               |
| `filters` | object | No       | Same filter parameters as the corresponding list endpoint.  |

**Response — 202 Accepted:**

```json
{
  "data": {
    "job_id": "uuid",
    "status": "pending",
    "download_url": null,
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

**Notes:**
- When `status` becomes `completed`, the `download_url` field contains the path to retrieve the file.
- Export files are retained for 24 hours before automatic cleanup.

---

## 9. Product Content — Reviews

### POST /api/items/:id/reviews

Submit a review for an item.

**Required Role:** Catalog Editor, Manager, Administrator.

**Path Parameters:**

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `id`      | string | Item UUID.  |

**Request Body:**

| Field    | Type    | Required | Description                          |
|----------|---------|----------|--------------------------------------|
| `rating` | integer | Yes      | Rating from 1 to 5.                 |
| `title`  | string  | Yes      | Review title. Max 200 characters.    |
| `body`   | string  | Yes      | Review text. Max 5000 characters.    |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "item_id": "uuid",
    "author_id": "uuid",
    "author_name": "Jane Doe",
    "rating": 4,
    "title": "Solid product",
    "body": "Works well for our use case...",
    "status": "published",
    "images": [],
    "follow_ups": [],
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

---

### GET /api/items/:id/reviews

List reviews for an item.

**Required Role:** Any authenticated user.

**Path Parameters:**

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `id`      | string | Item UUID.  |

**Query Parameters:**

| Parameter | Type    | Default    | Description                                  |
|-----------|---------|------------|----------------------------------------------|
| `rating`  | integer | —          | Filter by exact rating (1-5).                |
| `status`  | string  | `published`| Filter by status: `published`, `hidden`, `removed`. |
| `limit`   | integer | 25         | Page size.                                   |
| `cursor`  | string  | —          | Cursor.                                      |
| `sort`    | string  | `-created_at` | Sort field (`created_at`, `rating`).     |

**Response — 200 OK:** Paginated list of review objects with image URLs and follow-ups.

---

### POST /api/reviews/:id/follow-up

Add a follow-up comment to an existing review.

**Required Role:** Catalog Editor, Moderator, Manager, Administrator.

**Path Parameters:**

| Parameter | Type   | Description  |
|-----------|--------|--------------|
| `id`      | string | Review UUID. |

**Request Body:**

| Field  | Type   | Required | Description                          |
|--------|--------|----------|--------------------------------------|
| `body` | string | Yes      | Follow-up text. Max 2000 characters. |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "review_id": "uuid",
    "author_id": "uuid",
    "author_name": "Support Agent",
    "body": "Thank you for the feedback...",
    "created_at": "2026-04-06T11:00:00Z"
  }
}
```

---

### POST /api/reviews/:id/images

Upload images to a review.

**Required Role:** Catalog Editor, Manager, Administrator (must be review author or Moderator+).

**Path Parameters:**

| Parameter | Type   | Description  |
|-----------|--------|--------------|
| `id`      | string | Review UUID. |

**Request Body:** `multipart/form-data`

| Field    | Type   | Required | Description                              |
|----------|--------|----------|------------------------------------------|
| `images` | file[] | Yes      | One or more image files.                 |

**Response — 201 Created:**

```json
{
  "data": {
    "review_id": "uuid",
    "images": [
      { "id": "uuid", "url": "/uploads/reviews/uuid/img1.jpg", "size_bytes": 245000, "mime_type": "image/jpeg" }
    ],
    "total_images": 3
  }
}
```

**Error Codes:**

| Code | Scenario                                                        |
|------|-----------------------------------------------------------------|
| 400  | Invalid file type. Allowed: JPEG, PNG, WebP.                   |
| 400  | File exceeds 5 MB size limit.                                  |
| 422  | Upload would exceed maximum of 5 images per review.            |

**Notes:**
- Accepted formats: JPEG, PNG, WebP.
- Maximum file size: 5 MB per image.
- Maximum 5 images per review.

---

### DELETE /api/reviews/:id

Soft-delete a review.

**Required Role:** Moderator, Manager, Administrator (or review author).

**Path Parameters:**

| Parameter | Type   | Description  |
|-----------|--------|--------------|
| `id`      | string | Review UUID. |

**Response — 204 No Content.**

---

## 10. Product Content — Q&A

### POST /api/items/:id/questions

Submit a question about an item.

**Required Role:** Any authenticated user.

**Path Parameters:**

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `id`      | string | Item UUID.  |

**Request Body:**

| Field  | Type   | Required | Description                          |
|--------|--------|----------|--------------------------------------|
| `body` | string | Yes      | Question text. Max 2000 characters.  |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "item_id": "uuid",
    "author_id": "uuid",
    "author_name": "Jane Doe",
    "body": "Is this compatible with model XYZ?",
    "answers": [],
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

---

### GET /api/items/:id/questions

List questions for an item.

**Required Role:** Any authenticated user.

**Path Parameters:**

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `id`      | string | Item UUID.  |

**Query Parameters:**

| Parameter    | Type    | Default        | Description                            |
|--------------|---------|----------------|----------------------------------------|
| `unanswered` | boolean | —              | If `true`, only show questions with no answers. |
| `limit`      | integer | 25             | Page size.                             |
| `cursor`     | string  | —              | Cursor.                                |
| `sort`       | string  | `-created_at`  | Sort field.                            |

**Response — 200 OK:** Paginated list of question objects with nested answers.

---

### POST /api/questions/:id/answers

Submit an answer to a question.

**Required Role:** Catalog Editor, Moderator, Manager, Administrator.

**Path Parameters:**

| Parameter | Type   | Description    |
|-----------|--------|----------------|
| `id`      | string | Question UUID. |

**Request Body:**

| Field  | Type   | Required | Description                        |
|--------|--------|----------|------------------------------------|
| `body` | string | Yes      | Answer text. Max 5000 characters.  |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "question_id": "uuid",
    "author_id": "uuid",
    "author_name": "Catalog Team",
    "body": "Yes, it is fully compatible with model XYZ.",
    "created_at": "2026-04-06T11:00:00Z"
  }
}
```

---

## 11. Favorites & Browsing History

### GET /api/favorites

List the current user's favorited items.

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter | Type    | Default | Description    |
|-----------|---------|---------|----------------|
| `limit`   | integer | 25      | Page size.     |
| `cursor`  | string  | —       | Cursor.        |

**Response — 200 OK:** Paginated list of favorited item objects with `favorited_at` timestamp.

---

### POST /api/favorites

Add an item to the current user's favorites.

**Required Role:** Any authenticated user.

**Request Body:**

| Field     | Type   | Required | Description |
|-----------|--------|----------|-------------|
| `item_id` | string | Yes      | Item UUID.  |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "user_id": "uuid",
    "item_id": "uuid",
    "favorited_at": "2026-04-06T10:00:00Z"
  }
}
```

**Error Codes:**

| Code | Scenario                               |
|------|----------------------------------------|
| 409  | Item is already in favorites.          |

---

### DELETE /api/favorites

Remove an item from the current user's favorites.

**Required Role:** Any authenticated user.

**Request Body:**

| Field     | Type   | Required | Description |
|-----------|--------|----------|-------------|
| `item_id` | string | Yes      | Item UUID.  |

**Response — 204 No Content.**

---

### GET /api/browsing-history

Get the current user's recent browsing history (item detail views).

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter | Type    | Default | Description                    |
|-----------|---------|---------|--------------------------------|
| `limit`   | integer | 50      | Number of entries. Max 200.    |
| `cursor`  | string  | —       | Cursor.                        |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "item_id": "uuid",
      "item_name": "Widget A",
      "sku": "WGT-A-001",
      "viewed_at": "2026-04-06T09:45:00Z"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 150 }
}
```

**Notes:**
- Browsing history is recorded automatically when a user calls `GET /api/items/:id`.
- Duplicate views of the same item update the `viewed_at` timestamp rather than creating a new entry.

---

## 12. Moderation & Abuse Reports

### GET /api/moderation/queue

Get the moderation queue — content pending review.

**Required Role:** Moderator, Manager, Administrator.

**Query Parameters:**

| Parameter      | Type    | Default    | Description                                        |
|----------------|---------|------------|----------------------------------------------------|
| `content_type` | string  | —          | Filter: `review`, `question`, `answer`.            |
| `status`       | string  | `pending`  | Filter: `pending`, `escalated`.                    |
| `limit`        | integer | 25         | Page size.                                         |
| `cursor`       | string  | —          | Cursor.                                            |
| `sort`         | string  | `created_at` | Sort field. Oldest first by default.             |

**Response — 200 OK:** Paginated list of moderation queue items with content preview and report count.

---

### POST /api/abuse-reports

Submit an abuse report against a piece of content.

**Required Role:** Any authenticated user.

**Request Body:**

| Field          | Type   | Required | Description                                              |
|----------------|--------|----------|----------------------------------------------------------|
| `content_type` | string | Yes      | Type of content: `review`, `question`, `answer`.         |
| `content_id`   | string | Yes      | UUID of the reported content.                            |
| `reason`       | string | Yes      | One of: `spam`, `offensive`, `misleading`, `off_topic`, `other`. |
| `details`      | string | No       | Additional context. Max 1000 characters.                 |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "reporter_id": "uuid",
    "content_type": "review",
    "content_id": "uuid",
    "reason": "offensive",
    "details": "Contains inappropriate language.",
    "status": "open",
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

**Notes:**
- Duplicate reports from the same user for the same content return `409`.
- Reports that remain in `open` status for more than 48 hours are automatically escalated. The system sets `status` to `escalated` and creates notifications for Managers and Administrators.

---

### PATCH /api/abuse-reports/:id

Update the status of an abuse report.

**Required Role:** Moderator, Manager, Administrator.

**Path Parameters:**

| Parameter | Type   | Description         |
|-----------|--------|---------------------|
| `id`      | string | Abuse report UUID.  |

**Request Body:**

| Field            | Type   | Required | Description                                                  |
|------------------|--------|----------|--------------------------------------------------------------|
| `status`         | string | Yes      | New status: `dismissed`, `hidden`, `removed`, `warned`, `escalated`. |
| `moderator_note` | string | No       | Internal note explaining the decision.                       |

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "status": "removed",
    "moderator_id": "uuid",
    "moderator_note": "Content violates policy section 3.2.",
    "resolved_at": "2026-04-06T12:00:00Z"
  }
}
```

**Notes:**
- `hidden`: Content is hidden from public view but not deleted.
- `removed`: Content is soft-deleted.
- `warned`: Content author receives a warning notification.
- `escalated`: Report is escalated to Manager/Administrator level.
- Each status transition is recorded in the audit log.

---

### GET /api/abuse-reports/:id

Get full details of an abuse report.

**Required Role:** Moderator, Manager, Administrator.

**Path Parameters:**

| Parameter | Type   | Description         |
|-----------|--------|---------------------|
| `id`      | string | Abuse report UUID.  |

**Response — 200 OK:** Full abuse report object including content snapshot, reporter info, and moderation history.

---

## 13. Notifications (In-App Inbox)

### GET /api/notifications

List notifications for the current user.

**Required Role:** Any authenticated user.

**Query Parameters:**

| Parameter | Type    | Default | Description                                    |
|-----------|---------|---------|------------------------------------------------|
| `is_read` | boolean | —       | Filter by read/unread status.                  |
| `type`    | string  | —       | Filter by type: `info`, `warning`, `action`.   |
| `limit`   | integer | 25      | Page size.                                     |
| `cursor`  | string  | —       | Cursor.                                        |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "id": "uuid",
      "type": "warning",
      "title": "Bin A-01-03-02 disabled",
      "message": "Bin A-01-03-02 in Main Warehouse has been disabled. 12 items flagged for relocation.",
      "is_read": false,
      "action_url": "/warehouses/uuid/bins/uuid",
      "created_at": "2026-04-06T10:00:00Z"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 42 },
  "meta": { "unread_count": 7 }
}
```

---

### PATCH /api/notifications/:id/read

Mark a single notification as read.

**Required Role:** Any authenticated user (owner only).

**Path Parameters:**

| Parameter | Type   | Description         |
|-----------|--------|---------------------|
| `id`      | string | Notification UUID.  |

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "is_read": true
  }
}
```

---

### PATCH /api/notifications/read-all

Mark all notifications as read for the current user.

**Required Role:** Any authenticated user.

**Response — 200 OK:**

```json
{
  "data": {
    "marked_count": 7
  }
}
```

---

## 14. Integration Clients (Admin)

### POST /api/integrations/clients

Register a new integration client (API consumer).

**Required Role:** Administrator.

**Request Body:**

| Field         | Type     | Required | Description                                              |
|---------------|----------|----------|----------------------------------------------------------|
| `name`        | string   | Yes      | Client name.                                             |
| `description` | string   | No       | Description of the integration.                          |
| `permissions` | string[] | Yes      | Permission slugs granted to this client.                 |
| `ip_whitelist`| string[] | No       | Allowed source IP addresses. Empty = allow all.          |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "name": "ERP Connector",
    "description": "Syncs inventory levels to corporate ERP.",
    "permissions": ["inventory.read", "items.read"],
    "ip_whitelist": ["192.168.1.0/24"],
    "primary_key": "omni_sk_live_abc123...",
    "secondary_key": null,
    "rate_limit": "120/min",
    "is_active": true,
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

**Notes:**
- The `primary_key` is an HMAC signing key shown only once at creation. Store it securely.
- Rate limit: 120 requests/minute per integration client.

---

### GET /api/integrations/clients

List all integration clients.

**Required Role:** Administrator.

**Query Parameters:**

| Parameter   | Type    | Default | Description              |
|-------------|---------|---------|--------------------------|
| `is_active` | boolean | —       | Filter active/inactive.  |
| `limit`     | integer | 25      | Page size.               |
| `cursor`    | string  | —       | Cursor.                  |

**Response — 200 OK:** Paginated list of client objects (keys are masked).

---

### GET /api/integrations/clients/:id

Get a single integration client.

**Required Role:** Administrator.

**Response — 200 OK:** Client object with masked keys.

---

### PUT /api/integrations/clients/:id

Update an integration client.

**Required Role:** Administrator.

**Request Body:**

| Field          | Type     | Required | Description                    |
|----------------|----------|----------|--------------------------------|
| `name`         | string   | No       | Updated name.                  |
| `description`  | string   | No       | Updated description.           |
| `permissions`  | string[] | No       | Updated permissions.           |
| `ip_whitelist` | string[] | No       | Updated IP whitelist.          |
| `is_active`    | boolean  | No       | Enable/disable the client.     |

**Response — 200 OK:** Updated client object.

---

### DELETE /api/integrations/clients/:id

Soft-delete an integration client.

**Required Role:** Administrator.

**Response — 204 No Content.**

---

### POST /api/integrations/clients/:id/rotate-key

Rotate HMAC signing keys with primary/secondary support for zero-downtime rotation.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description    |
|-----------|--------|----------------|
| `id`      | string | Client UUID.   |

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "primary_key": "omni_sk_live_new456...",
    "secondary_key": "omni_sk_live_abc123...",
    "key_rotated_at": "2026-04-06T10:00:00Z",
    "secondary_key_expires_at": "2026-04-07T10:00:00Z"
  }
}
```

**Notes:**
- On rotation, the current `primary_key` becomes the `secondary_key`, and a new `primary_key` is generated.
- Both keys are valid for signing during the rotation window.
- The `secondary_key` expires after 24 hours, after which only the `primary_key` is accepted.
- The new `primary_key` is displayed only once in this response.

---

### GET /api/integrations/clients/:id/logs

Get request logs for an integration client.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description    |
|-----------|--------|----------------|
| `id`      | string | Client UUID.   |

**Query Parameters:**

| Parameter | Type    | Default       | Description                           |
|-----------|---------|---------------|---------------------------------------|
| `from`    | string  | —             | ISO 8601 start timestamp.             |
| `to`      | string  | —             | ISO 8601 end timestamp.               |
| `status`  | integer | —             | Filter by HTTP status code.           |
| `limit`   | integer | 50            | Page size.                            |
| `cursor`  | string  | —             | Cursor.                               |
| `sort`    | string  | `-created_at` | Sort field.                           |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "id": "uuid",
      "client_id": "uuid",
      "method": "GET",
      "path": "/api/inventory",
      "status_code": 200,
      "response_time_ms": 45,
      "ip_address": "192.168.1.50",
      "created_at": "2026-04-06T09:00:00Z"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 5200 }
}
```

---

## 15. Webhooks

### POST /api/webhooks

Register a webhook endpoint.

**Required Role:** Administrator.

**Request Body:**

| Field    | Type     | Required | Description                                                              |
|----------|----------|----------|--------------------------------------------------------------------------|
| `url`    | string   | Yes      | Destination URL for webhook delivery.                                    |
| `events` | string[] | Yes      | Events to subscribe to (e.g., `inventory.received`, `item.created`, `review.published`). |
| `secret` | string   | No       | Shared secret for HMAC-SHA256 signature verification. Auto-generated if omitted. |

**Response — 201 Created:**

```json
{
  "data": {
    "id": "uuid",
    "url": "https://erp.local/webhooks/omnistock",
    "events": ["inventory.received", "inventory.adjusted"],
    "secret": "whsec_...",
    "is_active": true,
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

**Notes:**
- Webhook payloads are signed with HMAC-SHA256. The signature is sent in the `X-OmniStock-Signature` header.
- The `secret` is shown only once at creation.

---

### GET /api/webhooks

List all registered webhooks.

**Required Role:** Administrator.

**Query Parameters:**

| Parameter   | Type    | Default | Description               |
|-------------|---------|---------|---------------------------|
| `is_active` | boolean | —       | Filter active/inactive.   |
| `event`     | string  | —       | Filter by event type.     |
| `limit`     | integer | 25      | Page size.                |
| `cursor`    | string  | —       | Cursor.                   |

**Response — 200 OK:** Paginated list of webhook objects (secrets masked).

---

### GET /api/webhooks/:id

Get a single webhook.

**Required Role:** Administrator.

**Response — 200 OK:** Webhook object with delivery statistics summary.

---

### PUT /api/webhooks/:id

Update a webhook.

**Required Role:** Administrator.

**Request Body:**

| Field       | Type     | Required | Description                |
|-------------|----------|----------|----------------------------|
| `url`       | string   | No       | Updated URL.               |
| `events`    | string[] | No       | Updated event list.        |
| `is_active` | boolean  | No       | Enable/disable.            |

**Response — 200 OK:** Updated webhook object.

---

### DELETE /api/webhooks/:id

Soft-delete a webhook.

**Required Role:** Administrator.

**Response — 204 No Content.**

---

### POST /api/webhooks/:id/retry

Manually retry the last failed delivery for a webhook.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description   |
|-----------|--------|---------------|
| `id`      | string | Webhook UUID. |

**Request Body:**

| Field         | Type   | Required | Description                                |
|---------------|--------|----------|--------------------------------------------|
| `delivery_id` | string | No       | Specific delivery UUID to retry. If omitted, retries the most recent failed delivery. |

**Response — 202 Accepted:**

```json
{
  "data": {
    "delivery_id": "uuid",
    "status": "pending",
    "retry_count": 1,
    "next_retry_at": "2026-04-06T10:01:00Z"
  }
}
```

**Notes:**
- Automatic retries use exponential backoff: 1 minute, 5 minutes, 15 minutes (3 attempts total).
- After 3 failed automatic retries, the delivery is marked `failed` and the webhook is deactivated if 10 consecutive deliveries fail.
- Manual retry resets the retry counter for that specific delivery.

---

### GET /api/webhooks/:id/deliveries

List delivery attempts for a webhook.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description   |
|-----------|--------|---------------|
| `id`      | string | Webhook UUID. |

**Query Parameters:**

| Parameter | Type    | Default       | Description                                       |
|-----------|---------|---------------|---------------------------------------------------|
| `status`  | string  | —             | Filter: `pending`, `delivered`, `failed`.         |
| `event`   | string  | —             | Filter by event type.                             |
| `from`    | string  | —             | ISO 8601 start timestamp.                         |
| `to`      | string  | —             | ISO 8601 end timestamp.                           |
| `limit`   | integer | 25            | Page size.                                        |
| `cursor`  | string  | —             | Cursor.                                           |
| `sort`    | string  | `-created_at` | Sort field.                                       |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "id": "uuid",
      "webhook_id": "uuid",
      "event": "inventory.received",
      "status": "delivered",
      "request_body": { "...": "..." },
      "response_status": 200,
      "response_time_ms": 120,
      "retry_count": 0,
      "created_at": "2026-04-06T09:30:00Z",
      "delivered_at": "2026-04-06T09:30:01Z"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 230 }
}
```

---

## 16. Audit Log

### GET /api/audit-log

Query the system-wide audit log.

**Required Role:** Administrator.

**Query Parameters:**

| Parameter | Type    | Default       | Description                                                        |
|-----------|---------|---------------|--------------------------------------------------------------------|
| `user_id` | string  | —             | Filter by user UUID.                                               |
| `action`  | string  | —             | Filter by action type (e.g., `login`, `inventory.receive`, `user.create`). |
| `entity`  | string  | —             | Filter by entity type (e.g., `user`, `item`, `inventory`, `bin`). |
| `entity_id`| string | —             | Filter by specific entity UUID.                                    |
| `from`    | string  | —             | ISO 8601 start timestamp.                                          |
| `to`      | string  | —             | ISO 8601 end timestamp.                                            |
| `limit`   | integer | 50            | Page size.                                                         |
| `cursor`  | string  | —             | Cursor.                                                            |
| `sort`    | string  | `-created_at` | Sort field.                                                        |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "id": "uuid",
      "user_id": "uuid",
      "username": "jdoe",
      "action": "inventory.receive",
      "entity": "inventory",
      "entity_id": "uuid",
      "ip_address": "192.168.1.42",
      "changes": {
        "before": {},
        "after": { "on_hand": 150 }
      },
      "created_at": "2026-04-06T10:05:00Z"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 15000 }
}
```

**Notes:**
- Audit log entries are immutable and cannot be deleted.
- The `changes` object captures before/after snapshots where applicable.

---

### POST /api/audit-log/export

Export audit log entries to a file.

**Required Role:** Administrator.

**Request Body:**

| Field    | Type   | Required | Description                                    |
|----------|--------|----------|------------------------------------------------|
| `format` | string | Yes      | Export format: `csv` or `pdf`.                 |
| `from`   | string | No       | ISO 8601 start timestamp.                      |
| `to`     | string | No       | ISO 8601 end timestamp.                        |
| `user_id`| string | No       | Filter by user UUID.                           |
| `action` | string | No       | Filter by action type.                         |
| `entity` | string | No       | Filter by entity type.                         |

**Response — 202 Accepted:**

```json
{
  "data": {
    "job_id": "uuid",
    "status": "pending",
    "format": "csv",
    "download_url": null,
    "created_at": "2026-04-06T10:00:00Z"
  }
}
```

**Notes:**
- The export runs as a batch job. When `status` is `completed`, the `download_url` field will contain the file path.
- Export files are retained for 24 hours.

---

## 17. Batch Jobs (Admin)

### GET /api/batch-jobs

List all batch jobs.

**Required Role:** Administrator.

**Query Parameters:**

| Parameter | Type    | Default       | Description                                                    |
|-----------|---------|---------------|----------------------------------------------------------------|
| `status`  | string  | —             | Filter: `pending`, `running`, `completed`, `failed`.           |
| `type`    | string  | —             | Filter by job type: `import`, `export`, `cleanup`, `reindex`. |
| `from`    | string  | —             | ISO 8601 start timestamp.                                      |
| `to`      | string  | —             | ISO 8601 end timestamp.                                        |
| `limit`   | integer | 25            | Page size.                                                     |
| `cursor`  | string  | —             | Cursor.                                                        |
| `sort`    | string  | `-created_at` | Sort field.                                                    |

**Response — 200 OK:**

```json
{
  "data": [
    {
      "id": "uuid",
      "type": "import",
      "status": "completed",
      "description": "Bulk item import - 500 rows",
      "progress_percent": 100,
      "started_at": "2026-04-06T10:00:05Z",
      "completed_at": "2026-04-06T10:01:30Z",
      "created_at": "2026-04-06T10:00:00Z",
      "created_by": "uuid"
    }
  ],
  "pagination": { "next_cursor": "...", "has_more": true, "total_count": 85 }
}
```

**Notes:**
- All batch jobs are tracked in the `batch_job_runs` table.

---

### GET /api/batch-jobs/:id

Get details of a specific batch job.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | Batch job UUID. |

**Response — 200 OK:**

```json
{
  "data": {
    "id": "uuid",
    "type": "import",
    "status": "completed",
    "description": "Bulk item import - 500 rows",
    "progress_percent": 100,
    "result_summary": {
      "total_rows": 500,
      "processed": 487,
      "skipped": 13,
      "errors": []
    },
    "download_url": "/api/bulk/import/uuid/results",
    "started_at": "2026-04-06T10:00:05Z",
    "completed_at": "2026-04-06T10:01:30Z",
    "created_at": "2026-04-06T10:00:00Z",
    "created_by": "uuid"
  }
}
```

---

### POST /api/batch-jobs/:id/rerun

Re-run a previously completed or failed batch job with the same parameters.

**Required Role:** Administrator.

**Path Parameters:**

| Parameter | Type   | Description     |
|-----------|--------|-----------------|
| `id`      | string | Batch job UUID. |

**Response — 202 Accepted:**

```json
{
  "data": {
    "new_job_id": "uuid",
    "original_job_id": "uuid",
    "status": "pending",
    "created_at": "2026-04-06T12:00:00Z"
  }
}
```

**Error Codes:**

| Code | Scenario                                                  |
|------|-----------------------------------------------------------|
| 422  | Original job is still running.                            |
| 404  | Job not found.                                            |

**Notes:**
- Creates a new job entry linked to the original. Does not modify the original record.
- Archived jobs (terminal status + older than 365 days) cannot be rerun. Returns `422`.

---

## 18. Dashboard & Metrics

### GET /api/dashboard/kpis

Get key performance indicators for the dashboard.

**Required Role:** Manager, Administrator.

**Query Parameters:**

| Parameter      | Type   | Default      | Description                                         |
|----------------|--------|--------------|-----------------------------------------------------|
| `warehouse_id` | string | —            | Filter by warehouse. Omit for system-wide KPIs.     |
| `period`       | string | `today`      | Time period: `today`, `7d`, `30d`, `90d`, `ytd`.   |

**Response — 200 OK:**

```json
{
  "data": {
    "period": "today",
    "warehouse_id": null,
    "kpis": {
      "total_items": 12450,
      "total_on_hand": 384200,
      "total_available": 351800,
      "total_reserved": 22400,
      "total_allocated": 10000,
      "items_below_reorder": 23,
      "disabled_bins": 4,
      "pending_abuse_reports": 7,
      "active_reservations": 156,
      "receives_today": 34,
      "picks_today": 128,
      "adjustments_today": 5
    }
  }
}
```

---

### GET /api/dashboard/metrics

Get detailed operational metrics.

**Required Role:** Manager, Administrator.

**Query Parameters:**

| Parameter      | Type   | Default  | Description                                             |
|----------------|--------|----------|---------------------------------------------------------|
| `warehouse_id` | string | —        | Filter by warehouse.                                    |
| `metric`       | string | —        | Specific metric: `put_away_time`, `pick_accuracy`, `review_resolution_sla`. Omit for all. |
| `period`       | string | `30d`    | Time period: `7d`, `30d`, `90d`, `ytd`.               |
| `granularity`  | string | `day`    | Data point granularity: `hour`, `day`, `week`, `month`.|

**Response — 200 OK:**

```json
{
  "data": {
    "period": "30d",
    "granularity": "day",
    "metrics": {
      "put_away_time": {
        "unit": "minutes",
        "average": 12.4,
        "p50": 10.0,
        "p95": 28.0,
        "trend": -0.8,
        "data_points": [
          { "date": "2026-03-07", "value": 13.2 },
          { "date": "2026-03-08", "value": 11.8 }
        ]
      },
      "pick_accuracy": {
        "unit": "percent",
        "average": 99.2,
        "trend": 0.1,
        "data_points": [
          { "date": "2026-03-07", "value": 99.0 },
          { "date": "2026-03-08", "value": 99.4 }
        ]
      },
      "review_resolution_sla": {
        "unit": "hours",
        "average": 18.5,
        "p50": 12.0,
        "p95": 44.0,
        "sla_target_hours": 48,
        "compliance_percent": 94.2,
        "trend": -2.1,
        "data_points": [
          { "date": "2026-03-07", "value": 20.0 },
          { "date": "2026-03-08", "value": 16.5 }
        ]
      }
    }
  }
}
```

**Notes:**
- `trend` indicates the change compared to the previous equivalent period (positive = increase, negative = decrease).
- `review_resolution_sla` tracks time from abuse report creation to resolution, with the 48-hour escalation target as the SLA benchmark.

---

## Appendix A: Permission Slugs

The following permission slugs are used across role assignments and integration client configurations:

| Slug                        | Description                                    |
|-----------------------------|------------------------------------------------|
| `users.read`                | View user accounts.                            |
| `users.write`               | Create/update/delete user accounts.            |
| `users.lock`                | Lock/unlock user accounts.                     |
| `roles.read`                | View roles and permissions.                    |
| `roles.write`               | Create/update/delete roles and permissions.    |
| `warehouses.read`           | View warehouses, zones, bins.                  |
| `warehouses.write`          | Create/update/delete warehouses, zones, bins.  |
| `bins.manage`               | Enable/disable bins.                           |
| `inventory.read`            | View inventory and stock ledger.               |
| `inventory.receive`         | Receive stock (put-away).                      |
| `inventory.move`            | Move stock between bins.                       |
| `inventory.pick`            | Pick stock.                                    |
| `inventory.adjust`          | Adjust inventory (cycle count, damage, etc.).  |
| `inventory.reserve`         | Create/release stock reservations.             |
| `items.read`                | View catalog items.                            |
| `items.write`               | Create/update catalog items.                   |
| `items.delete`              | Soft-delete catalog items.                     |
| `items.barcode_edit`        | Modify item barcodes (Administrator only).     |
| `reviews.write`             | Create reviews and follow-ups.                 |
| `reviews.delete`            | Soft-delete reviews.                           |
| `reviews.images`            | Upload review images.                          |
| `questions.write`           | Create questions.                              |
| `answers.write`             | Create answers.                                |
| `moderation.read`           | View moderation queue and abuse reports.       |
| `moderation.write`          | Update abuse report status.                    |
| `favorites.manage`          | Manage personal favorites.                     |
| `bulk.import`               | Execute bulk imports.                          |
| `bulk.export`               | Execute bulk exports.                          |
| `integrations.manage`       | Manage integration clients.                    |
| `webhooks.manage`           | Manage webhooks.                               |
| `audit.read`                | View and export audit logs.                    |
| `batch_jobs.read`           | View batch job status.                         |
| `batch_jobs.manage`         | Rerun batch jobs.                              |
| `dashboard.read`            | View dashboard KPIs and metrics.               |
| `notifications.read`        | View personal notifications.                   |
| `search.global`             | Use global search.                             |
| `saved_views.manage`        | Manage personal saved views.                   |

---

## Appendix B: Webhook Event Types

| Event                        | Trigger                                            |
|------------------------------|----------------------------------------------------|
| `inventory.received`         | Stock received into a bin.                         |
| `inventory.moved`            | Stock moved between bins.                          |
| `inventory.picked`           | Stock picked from a bin.                           |
| `inventory.adjusted`         | Inventory adjustment recorded.                     |
| `inventory.reserved`         | Stock reservation created.                         |
| `inventory.reservation_released` | Stock reservation released or expired.         |
| `item.created`               | New catalog item created.                          |
| `item.updated`               | Catalog item updated.                              |
| `item.deleted`               | Catalog item soft-deleted.                         |
| `review.published`           | New review published.                              |
| `review.deleted`             | Review soft-deleted.                               |
| `abuse_report.created`       | New abuse report submitted.                        |
| `abuse_report.escalated`     | Abuse report escalated (48-hour auto or manual).   |
| `abuse_report.resolved`      | Abuse report resolved.                             |
| `bin.disabled`               | Bin disabled, inventory flagged for relocation.    |
| `bin.enabled`                | Bin re-enabled.                                    |
| `batch_job.completed`        | Batch job finished successfully.                   |
| `batch_job.failed`           | Batch job failed.                                  |
| `user.locked`                | User account locked (auto or manual).              |
| `user.unlocked`              | User account unlocked.                             |

---

## Appendix C: Rate Limit Headers

All responses to integration client requests include the following headers:

| Header                  | Description                                         |
|-------------------------|-----------------------------------------------------|
| `X-RateLimit-Limit`     | Maximum requests allowed per window (120).          |
| `X-RateLimit-Remaining` | Requests remaining in the current window.           |
| `X-RateLimit-Reset`     | Unix timestamp when the window resets.              |
| `Retry-After`           | Seconds until the client can retry (only on `429`). |
