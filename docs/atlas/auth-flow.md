# Atlas Auth Flow

What's built, what isn't yet, and the design for the rest, so the next
pass implements against a plan rather than improvising per-endpoint.
JWT plus a rotating refresh token, on Atlas's flat tenant model.
`atlas-pos` is a second client beside `atlas-web`. The role model is
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

Verifies the password, then picks a tenant. Store-scoped grants count:
a cashier's `role_grants.tenant_id` is set even though `scope` is `store`.
One tenant logs the user in. More than one returns `409 TENANT_REQUIRED`
with `{ data: { tenants: [{ id, name }] } }` until the body names
`tenant_id`. A platform admin with no tenant grant gets
`409 PLATFORM_TENANT_REQUIRED` and the full tenant list, and may name any
existing tenant. The session is a 15-minute access JWT (`sub`, `tenant_id`,
`exp`) and a 7-day opaque refresh token (stored SHA-256-hashed, see
`crypto::hash_lookup_token`'s doc comment for why that's not Argon2).

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

| Role             | Scope           | How they're provisioned                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ---------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `owner`          | tenant          | **Signup** (`POST /auth/signup`). Exactly one per tenant at creation; a tenant can have more than one `owner` grant later only via the invite flow (#5), an owner inviting a co-owner.                                                                                                                                                                                                                                                                          |
| `admin`          | tenant          | **Invite** (#5), by an `owner` or an existing `admin` - `admin`'s own permission set is a subset of `owner`'s, so an admin inviting another admin passes the "can't grant more than you hold" check.                                                                                                                                                                                                                                                            |
| `store_manager`  | store           | **Invite** (#5), by `owner`/`admin` (tenant-wide) or by a `store_manager` already granted at that specific store (a manager can build out their own store's team, not another store's).                                                                                                                                                                                                                                                                         |
| `cashier`        | store           | **Invite** (#5), by anyone holding `staff.invite` at that store - in practice `owner`/`admin`/`store_manager`.                                                                                                                                                                                                                                                                                                                                                  |
| `stock_clerk`    | store           | Same as `cashier`.                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `accountant`     | tenant or store | **Invite** (#5). Tenant-scoped when the owner wants one bookkeeper across every branch; store-scoped when a store manager wants their own. Both are the same role definition - only the grant's `store_id` differs.                                                                                                                                                                                                                                             |
| `platform_admin` | platform        | Seeded by `20260922000022_seed_platform_admin_role.sql`. Further staff are invited only by someone who already holds a platform grant: `POST /api/v1/auth/platform/invite` with `{ email }`. An existing account gets the grant immediately. A new email gets an `atlas.invites` row with `tenant_id` null (`20261020000024_platform_invites.sql`) and accepts it at `POST /api/v1/auth/invites/{token}/accept`. Never assigned through the tenant invite flow. |

Two invariants that hold across every row above, already enforced by
`roles::service::assert_permission`'s "granter can't exceed their own
permissions" rule and the `role_grants` schema itself, not by a special
case per role:

- **Nobody can grant `platform_admin`** through `POST /api/v1/roles/invite`.
  That path only writes `scope = 'tenant'` or `'store'`. Platform invites
  are `POST /api/v1/auth/platform/invite`, and the caller must already hold
  a platform grant.
- **A custom role a tenant builds** (`roles.tenant_id` set,
  `is_system = false` - see the original scaffolding plan's "owner wants
  to build a custom role" example) goes through the exact same invite path
  as a system role. There is no separate mechanism for custom roles.

---

## Also built

### 5. Invite a role - `POST /api/v1/roles/invite`

`TenantTx`, then `staff.invite`, then `assert_grantable` (a granter cannot
hand out a permission they do not hold).

```
POST /api/v1/roles/invite
Body: { email, role_id, store_id: Option<Uuid> }
```

1. Existing account: insert an active `role_grants` row and email a notice.
2. New email: insert `atlas.invites` (hashed token, 7-day expiry) and email
   the token.
3. `POST /api/v1/auth/invites/{token}/accept` with `full_name`, `phone`, and
   `password` creates the user pre-verified and the grant. It does not log
   them in.

### 6. Forgot / reset password

`POST /api/v1/auth/forgot-password` always returns the same message.
`POST /api/v1/auth/reset-password` consumes an OTP with purpose
`password_reset` from `email_otps`, sets the new password, and revokes
every refresh token. It does not log the user in.

### 7. Client token shape

`X-Client-Type` defaults to `web`. Web gets `Set-Cookie` only (`access_token`,
`refresh_token`, HttpOnly). `app` gets the tokens in the JSON body and no
cookie. The header is a response-shape hint, not a permission boundary.

`atlas-web` never calls the API origin from the browser. Both apps
use `@atlas/api`, an Axios client whose interceptors attach `x-client-type`
and always settle on `{ status, body }` where `body` is
`{ success, message, data?, code? }`. A missing body or a network failure
becomes `{ success: false }` with status 502, so screens do not each parse
Axios errors. Marketing's base URL is `/api`, so the call still hits Nitro
(`server/api/[...path].ts`). That proxy injects the Bearer token from the
cookie, copies `Set-Cookie` back, retries once on 401 via `/auth/refresh`,
and on plain HTTP strips `Secure` so a local browser will store the cookie.
Store selection is a separate HttpOnly cookie at `/session/store`, forwarded
as `x-store-id`. The marketing client does not refresh on its own.

`atlas-pos` sends `X-Client-Type: app` and talks to the API directly. The
access token and the refresh token stay in memory. A 401 on an authenticated
call refreshes once, then retries. There is no Capacitor secure-storage
plugin in the app yet, so a reload asks the cashier to log in again. Do not
put either token in `localStorage`.

### 8. Session, tenant switch, store switch

- `GET /api/v1/auth/me` returns the user, `tenant_id`, `is_platform_admin`,
  and the tenants that user holds a grant in.
- `POST /api/v1/auth/switch-tenant` with `{ tenant_id }` re-issues a session
  for a grant the caller holds. A platform admin may name any existing tenant.
- `GET /api/v1/auth/platform/tenants` lists every tenant. Platform staff only.
- `POST /api/v1/auth/logout` revokes the presented refresh token. Web reads
  it from the cookie (`RefreshRequest.refresh_token` may be empty). App sends
  it in the body.
- The dashboard and the POS home screen both expose the tenant list and the
  store list. The web store id is the `/session/store` cookie. The POS store
  id is memory and is sent as `x-store-id`.

### 9. Rate limiting

The whole `/api/v1/auth` router sits behind an in-memory per-IP limiter
(`apps/atlas-backend/src/rate_limit.rs`). OTP verify and reset also count
attempts on the code itself.

### Web and POS screens

`atlas-web` pages: `/` is the overview. Auth pages live under
`/auth`: `/auth/signup`, `/auth/verify-email`, `/auth/login`,
`/auth/forgot-password`, `/auth/reset-password`, `/auth/invites/[token]`.
The signed-in home is `/dashboard`. The layout is a full-width header and
footer. `/dashboard` uses the
`auth` route middleware. Login renders the tenant picker when the API
returns `TENANT_REQUIRED` or `PLATFORM_TENANT_REQUIRED`.

`atlas-pos` routes: `/auth/login` and `/` (requires the in-memory session).
Home can switch tenant, pick a store, invite platform staff, and log out.
Checkout and offline sale sync are not on these screens yet.

---

## Still open

- Capacitor secure storage for the POS refresh token. Memory-only is the
  current stand-in.
- A checkout screen that calls `POST /api/v1/sales` and
  `POST /api/v1/sync/sales`.
- A UI for building custom roles. The API path is the same invite endpoint.
  There is no screen that inserts a custom `atlas.roles` row.
