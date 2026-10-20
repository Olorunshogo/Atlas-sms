# Atlas Auth Flow

What's built, what isn't yet, and the design for the rest, so the next
pass implements against a plan rather than improvising per-endpoint.
JWT plus a rotating refresh token, on Atlas's flat tenant model.
`atlas-pos` is a second client beside `atlas-marketing`. The role model is
`roles`/`role_grants` (dynamic, permission-array based).

---

## Built (`apps/atlas-backend/src/auth/`)

### 1. Signup - `POST /api/v1/auth/signup`

Creates the user and a brand-new tenant in one transaction (`AdminTx`, since
no tenant exists yet to scope a `TenantTx` to), grants the caller `owner` on
it (`role_grants.status = 'pending'` until verified), and emails a 6-digit
OTP via `EmailProvider` (`LogEmailProvider` in dev).

There is no "join an existing tenant" signup path - that's the invite flow
below. Every signup creates its own tenant.

### 2. Verify email - `POST /api/v1/auth/verify-email`

Consumes the OTP, sets `users.is_email_verified = true`, and activates every
`pending` role grant the user holds (in practice, exactly the one signup
created).

### 3. Login - `POST /api/v1/auth/login`

Verifies the password, resolves which tenant to authenticate into (the sole
one if the user has exactly one active grant, otherwise the caller must name
`tenant_id` - `409 TENANT_REQUIRED`), issues a 15-minute access JWT
(`sub`, `tenant_id`, `exp`) and a 7-day opaque refresh token (stored
SHA-256-hashed, see `crypto::hash_lookup_token`'s doc comment for why that's
not Argon2).

### 4. Refresh - `POST /api/v1/auth/refresh`

Looks the refresh token up by its lookup hash, issues a new access token for
the same tenant, and **rotates** the refresh token (revokes the old row,
inserts a new one) - a refresh token presented twice (once by an attacker,
once by the real client) is detectable because the second presentation
finds a revoked row.

### Enforcement: `TenantTx`

Every tenant-scoped handler takes a `TenantTx`, which requires a verified
`Authorization: Bearer <token>` JWT (never a raw header) and exposes
`user_id`/`tenant_id` from its claims, plus an optional `x-store-id` header
for store selection. `AdminTx` is the narrow, explicit bypass for pre-tenant
operations.

---

## Per-role provisioning path

The mechanism above (signup / invite / platform provisioning) is generic;
this is what actually happens for each seeded role. "Store-scoped" means
the grant's `store_id` is set (visible/actionable at that store only);
"tenant-scoped" means `store_id IS NULL` (every store the tenant has, now
and later - opening store #2 needs no new grant for a tenant-scoped role).

| Role             | Scope           | How they're provisioned                                                                          |
| ---------------- | --------------- | -------------------------------------------------------------------------------------------------- |
| `owner`          | tenant          | **Signup** (`POST /auth/signup`). Exactly one per tenant at creation; a tenant can have more than one `owner` grant later only via the invite flow (#5), an owner inviting a co-owner. |
| `admin`          | tenant          | **Invite** (#5), by an `owner` or an existing `admin` - `admin`'s own permission set is a subset of `owner`'s, so an admin inviting another admin passes the "can't grant more than you hold" check. |
| `store_manager`  | store           | **Invite** (#5), by `owner`/`admin` (tenant-wide) or by a `store_manager` already granted at that specific store (a manager can build out their own store's team, not another store's). |
| `cashier`        | store           | **Invite** (#5), by anyone holding `staff.invite` at that store - in practice `owner`/`admin`/`store_manager`. |
| `stock_clerk`    | store           | Same as `cashier`.                                                                                  |
| `accountant`     | tenant or store | **Invite** (#5). Tenant-scoped when the owner wants one bookkeeper across every branch; store-scoped when a store manager wants their own. Both are the same role definition - only the grant's `store_id` differs. |
| `platform_admin` | platform        | **Direct provisioning only** (migration `20260922000022_seed_platform_admin_role.sql` seeds the role; no endpoint grants it). Atlas's own staff, never a tenant-assignable role. An internal admin tool to invite platform staff is out of scope until Atlas has more than a couple of internal operators. |

Two invariants that hold across every row above, already enforced by
`roles::service::assert_permission`'s "granter can't exceed their own
permissions" rule and the `role_grants` schema itself, not by a special
case per role:

- **Nobody can grant `platform_admin`** through the tenant invite flow -
  it's a different `scope`, and the invite design (#5) only ever writes
  `scope = 'tenant'`/`'store'` grants.
- **A custom role a tenant builds** (`roles.tenant_id` set,
  `is_system = false` - see the original scaffolding plan's "owner wants
  to build a custom role" example) goes through the exact same invite path
  as a system role. There is no separate mechanism for custom roles.

---

## Not built yet

### 5. Invite a role - `POST /api/v1/tenants/roles/invite` (planned)

**The gap that matters most right now**: there is no way today for an owner
to add an admin, or for anyone to add a store-scoped `store_manager`/
`cashier`/`stock_clerk`/`accountant`. Every role grant in the schema besides
the signup-time `owner` one is currently only reachable by hand-inserting a
row.

Design, following `roles::service::assert_permission`'s existing rule
("a granter can never hand out more than they hold"):

```
POST /api/v1/tenants/roles/invite
Auth: Bearer <token> (TenantTx)
Body: { email, role_id, store_id: Option<Uuid> }
```

1. `assert_permission(caller, tenant_id, None, "staff.invite")`.
2. Look up the target `role_id`'s `permissions`; reject if it is not a
   subset of the caller's own effective permissions for that scope (the
   same containment check `assert_permission` already does, exposed as a
   standalone `assert_grantable` helper so it's reusable here).
3. If a user with that email exists: insert a `role_grants` row directly,
   `status = 'pending'`, and email an "you've been added to \<tenant\>"
   notice (no new OTP needed - they already have a verified account).
4. If no user exists: insert an `invites` table row (new migration -
   `email`, `role_id`, `tenant_id`, `store_id`, `token_hash`, `expires_at`,
   `accepted_at`) and email an invite link. Accepting it
   (`POST /api/v1/auth/invites/:token/accept`, body: `full_name`, `phone`,
   `password`) creates the user pre-verified (they proved control of the
   email by receiving the link, so no separate OTP step) and the
   `role_grants` row in one transaction.

This is the mechanism behind "an admin can assign further roles" and "an
owner branches out and staffs a new store" from the original design
conversation - nothing else needs to change in the role/permission schema,
this is the one missing endpoint set.

### 6. Forgot / reset password (planned)

`POST /api/v1/auth/forgot-password` always
returns the same message regardless of whether the email exists (anti-
enumeration); consumes an OTP via `POST /api/v1/auth/reset-password`.
Reuses the existing `email_otps` table with a new `purpose =
'password_reset'` value - no schema change needed, `otp::generate_code`/
`hash_code`/`verify_code` are already generic.

### 7. Mobile token delivery for `atlas-pos`

`atlas-marketing`'s Nitro proxy is the only client today, and it expects
`Set-Cookie` (cookies read server-side, never handed to browser JS).
`atlas-pos` is a Capacitor app with no server of its own.
`X-Client-Type: app` returns tokens in the response body instead of a cookie.

Plan: `/auth/login` and `/auth/refresh` branch on `X-Client-Type`
(`web` -> cookies only, no tokens in the body; `app` -> tokens in the body,
no `Set-Cookie`). A `capacitor://` origin can't reliably receive an HttpOnly cookie
cross-site. `X-Client-Type` is a response-shape hint only, never a
permission boundary - neither mode may grant more than the other.
`atlas-pos` keeps the refresh token in Capacitor secure storage and the
access token in memory only, never `localStorage` - load-bearing given a
POS device is more likely to be shared/lost than a phone.

### 8. Tenant/store switching mid-session

Currently a user with grants in more than one tenant must re-login,
naming `tenant_id`, to switch. Whether that's acceptable for v1 or needs a
`POST /api/v1/auth/switch-tenant` (reissue a token for a different active
grant without a full password re-entry) is an open question - low priority
given Atlas's SOM is 1-20 stores under one tenant, so multi-tenant users are
the exception, not the common case. Revisit if it becomes one.

### 9. Rate limiting

Not implemented. `/auth/login` and future `/auth/forgot-password` are the
two endpoints that need it most (credential stuffing, enumeration). Tracked
as a gap, not designed yet - likely an `axum` middleware layer keyed by IP
+ email, or pushed to the edge (Cloudflare) once `atlas-marketing` is
actually deployed there.

---

## Build order for the rest

1. Invite flow (#5) - unblocks the entire "admin adds staff" story, which
   nothing else in the roles/permissions design depends on but which is
   completely unusable without it today.
2. Forgot/reset password (#6) - small, reuses existing tables.
3. `X-Client-Type` mobile branch (#7) - needed before `atlas-pos` can
   authenticate at all, so it's a prerequisite for wiring real screens
   there, not before.
4. Tenant switching (#8) and rate limiting (#9) - defer until a real need
   shows up; tracked here so they aren't forgotten, not because they're
   next.
