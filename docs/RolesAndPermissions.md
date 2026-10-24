# Roles and Permissions

How access control actually works in Atlas today. Sources: the migration
that defines the schema
(`apps/atlas-backend/migrations/20260922000005_create_roles_and_grants.sql`),
the seed data (`...0009_seed_system_roles.sql`,
`...0020_add_product_permission.sql`, `...0021_grant_product_permission_to_roles.sql`,
`...0022_seed_platform_admin_role.sql`,
`...20261022000025_add_sales_permissions.sql`,
`...20261022000026_grant_sales_permissions.sql`), and the Rust source under
`apps/atlas-backend/src/roles/` and `src/extractors.rs`.

---

## 1. The core idea

There is no fixed role enum and no single "role" column on a user. Instead:

- **`atlas.roles`** - a named bundle of permissions. Either a seeded
  system role (`tenant_id NULL`, `is_system = true`, never editable) or a
  role a tenant builds itself (`tenant_id` set, `is_system = false`).
- **`atlas.role_grants`** - one row per (user, role, tenant, optional
  store). A user can hold several grants at once - different roles at
  different stores fall out of this being multiple rows, not a special
  case.

A permission is never just "can void sale." It's always "can void sale,
in store 3" (or tenant-wide, or platform-wide). The scope lives on the
grant, not on the permission itself.

---

## 2. The permission list

`atlas.permission` is a Postgres enum, not a free-text column - a typo'd
permission string is rejected at write time (a `422`, effectively),
instead of silently evaluating to "not granted" at request time.

```
sale.create      sale.void        sale.refund
stock.view       stock.adjust     stock.transfer
purchase.create  supplier.manage
staff.invite     staff.remove
store.create     store.update
role.create      role.assign
report.view
tenant.manage
product.manage   (added later - covers products, product types,
                   units, and price rules in one permission rather
                   than one per entity)

Added with the sales, receipts, returns and customer work:
sale.view            search and read sales beyond your own open shift
sale.discount        discount above the store's cashier allowance
sale.price_override  charge a price other than the resolved one
sale.return          start a return (refunding it may still need approval)
receipt.reprint      print a receipt again after the original
customer.view        look customers up (contact details masked)
customer.manage      register, edit and erase customers
customer.history     full contact details and purchase history
payment.manage       confirm, fail or reverse a payment

Added with purchasing, expenses and stock control:
purchase.approve     approve a submitted purchase order, and receive more
                     than was ordered when an over-receipt reason is given
expense.manage       record, approve and pay store expenses
stock.approve        adjust or write off more than the store's threshold
audit.view           search the audit trail and see operational issues

Added with the online storefront:
storefront.manage    set up a store's online shop and shipping zones
promotion.manage     create and change promotions
order.manage         process, ship, deliver, cancel and collect online orders
```

Small and coarse-grained on purpose. `product.manage` bundling four
different resources under one permission is a deliberate choice - split
it further only when a real case needs "can edit prices but not add
product types" as a distinct grant, not pre-emptively. The later
additions each mark a line a real role draws differently: a cashier
looks customers up and starts returns, but doesn't see purchase history,
override prices, reprint receipts, or approve their own refunds.

Some checks only change behaviour instead of refusing the request, via
`roles::service::has_permission`: a quote reports a price override
without `sale.price_override` as a violation instead of failing, and a
return started without `sale.refund` waits for approval. Approving a
return also needs a different person from whoever started it.

---

## 3. Scope: platform, tenant, or store

`atlas.role_scope` is `'platform' | 'tenant' | 'store'`, enforced by three
check constraints on `role_grants` so an invalid combination can't be
written at all:

| Scope      | Meaning                                          | Constraint                 |
| ---------- | ------------------------------------------------ | -------------------------- |
| `platform` | Atlas's own internal staff, crosses every tenant | `tenant_id` must be `NULL` |
| `tenant`   | Every store the tenant has, now and later        | `store_id` must be `NULL`  |
| `store`    | One specific store only                          | `store_id` is required     |

A `tenant`-scope grant is what lets an owner open store #2 tomorrow
without a new grant - "every store" is expressed as "no store filter,"
not as a row per store.

**Grant status** (`atlas.role_grant_status`): `pending` (signup created
it, email not verified yet), `active` (usable), `revoked`. Only `active`
grants are considered by any permission check.

---

## 4. The one check every handler calls

`roles::service::assert_permission` (`apps/atlas-backend/src/roles/service.rs`)
is the single generic gate. No handler writes its own bespoke role
`if`/`match` - every permission-gated endpoint calls this same function
with a permission string and a scope:

```rust
roles::service::assert_permission(
    &mut tx,
    user_id,
    tenant_id,
    Some(body.store_id),   // None for a tenant-wide action
    "sale.create",
)
.await?;
```

It's satisfied when the caller has an `active` grant that is any of:

- `platform` scope (Atlas staff can do anything), **or**
- `tenant` scope matching this `tenant_id`, **or**
- `store` scope matching this exact `store_id`, **or**
- a `store_group_grants` row covering this `store_id` (see section 7 -
  wired into the check already, even though nothing populates that table
  yet).

...and the role behind that grant actually has the requested permission
in its `permissions` array. One SQL query, one code path, used
identically by `sales`, `stock`, `roles`, and every other module.

Two companions share that query: `has_permission` returns the answer as
a bool, and `stores_with_permission` returns every store a permission
reaches (`StoreScope::All` or a list) for list endpoints that span
stores, such as sales search and customer purchase history.

Customers belong to the tenant, not a store, so their endpoints check
against the caller's current store from `x-store-id`. Without that
header a store-scoped grant can't match, and the request is a 403.

---

## 5. Enforcement is structural, not a convention

Every tenant-scoped handler takes a `TenantTx` (`src/extractors.rs`)
instead of a raw database connection - there is no other way to obtain a
connection inside one of these handlers. `TenantTx`:

1. Decodes and verifies the `Authorization: Bearer` JWT (never trusts a
   raw header for identity).
2. Opens a transaction and immediately runs
   `set_config('app.current_tenant_id', ...)` from the token's `tenant_id`
   claim.
3. Optionally sets `app.current_store_id` from an `x-store-id` header -
   this one _is_ a plain header, because it's the caller picking which
   store's data to look at, not an identity claim. A role grant scoped to
   a different store still gets rejected, either by `assert_permission`
   or by row-level security itself.

Because `set_config` runs inside Postgres before any query executes, and
every tenant table is `FORCE ROW LEVEL SECURITY`, a handler that forgets
an `assert_permission` call still can't read another tenant's rows - RLS
is the backstop, `assert_permission` is the finer-grained permission
check on top of it. Two independent layers, not one mechanism doing both
jobs.

`AdminTx` is the deliberately narrow escape hatch for the handful of
operations that legitimately have no tenant yet to scope to - signup,
login, refresh - or that intentionally cross tenants (platform-role
work). It sets `app.bypass_tenant = 'on'` instead of a tenant ID. A
handler either takes `TenantTx` or `AdminTx` at the type level; there is
no runtime toggle that flips a `TenantTx` handler into bypass mode.

---

## 6. Granting a role: the invite flow

**Built and wired** - `POST /api/v1/roles/invite` →
`roles::handlers::invite`:

1. `assert_permission(caller, tenant_id, store_id, "staff.invite")` - the
   caller must be allowed to invite at this scope at all.
2. `assert_grantable(caller, tenant_id, store_id, target_role_id)` - see
   section 7, the containment check.
3. If the email belongs to an existing, verified user: insert an
   `active` `role_grants` row directly and email a notice. No new OTP -
   they've already proven control of that email account.
4. If not: insert an `atlas.invites` row (email, role, tenant/store,
   hashed token, 7-day expiry) and email an invite link. Accepting it
   (`POST /api/v1/auth/invites/:token/accept`) creates the user
   pre-verified and the `role_grants` row in one transaction - receiving
   the link is the proof of email ownership, so no separate OTP step
   either.

The `owner` role is the one exception: it's granted only at signup
(`POST /auth/signup` creates the user, a brand-new tenant, and a
`pending` `owner` grant together, activated on email verification).
There's no "join an existing tenant" signup path - every signup creates
its own tenant. A second `owner` on a tenant only ever happens via an
existing owner/admin inviting one through the same flow above.

---

## 7. "A granter can never hand out more than they hold"

This is the rule that makes a dynamic, tenant-editable role system safe
without a rigid hierarchy table (no "admin > store_manager > cashier"
ranking hardcoded anywhere). `roles::service::assert_grantable`:

1. Looks up the target role's full `permissions` array.
2. Computes the granter's own **effective permissions** at that scope -
   the union of every permission across every `active` grant they hold
   that reaches this tenant/store (so an admin who also happens to be a
   store's `stock_clerk` still gets the admin-level union when inviting
   at that store, not just the narrower grant).
3. Rejects with a `403` naming the first permission the target role has
   that the granter doesn't.

One consequence worth naming explicitly: **nobody can grant
`platform_admin`** through `POST /api/v1/roles/invite`. That handler only
writes `tenant` or `store` scope. Further platform staff use
`POST /api/v1/auth/platform/invite`, and the caller must already hold a
platform grant. The role row itself is seeded by
`20260922000022_seed_platform_admin_role.sql`. A new email lands in
`atlas.invites` with `tenant_id` null (`20261020000024_platform_invites.sql`).

A second consequence: **a tenant's own custom role goes through the
exact same path** as a system role. There's no separate "custom role"
mechanism - `assert_grantable` only cares about the target role's
permission array, not whether `is_system` is true or false.

---

## 8. Seeded system roles

Six tenant-facing roles plus one platform role, seeded once with fixed
UUIDs (so code can reference `owner`'s ID as a constant, not a name
lookup):

| Role             | Scope in practice                                                                      | Permissions                                                                                                                                                                                                               |
| ---------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `owner`          | tenant-wide                                                                            | Everything: sales, stock, purchasing, staff, stores, roles, reports, customers, `tenant.manage`, `product.manage`                                                                                                         |
| `admin`          | tenant-wide                                                                            | Same as `owner` minus `tenant.manage`                                                                                                                                                                                     |
| `store_manager`  | usually store-scoped                                                                   | Sales (incl. view, discount, override, return, reprint), payments, customers, stock, purchasing, `staff.invite`, `report.view`, `product.manage` - no `staff.remove`, `role.create`/`role.assign`, or store create/update |
| `cashier`        | store-scoped                                                                           | `sale.create`, `sale.return`, `stock.view`, `customer.view`, `customer.manage`                                                                                                                                            |
| `stock_clerk`    | store-scoped                                                                           | `stock.view`, `stock.adjust`, `stock.transfer`                                                                                                                                                                            |
| `accountant`     | tenant **or** store-scoped (same role definition, only the grant's `store_id` differs) | `stock.view`, `report.view`, `sale.view`, `customer.history`                                                                                                                                                              |
| `platform_admin` | platform                                                                               | Everything `owner` has, plus it's the only role with `scope = 'platform'`                                                                                                                                                 |

`store_manager`, `cashier`, and `stock_clerk` are typically store-scoped
grants in practice (per `docs/atlas/auth-flow.md`'s provisioning table),
but nothing in the schema forces that - a tenant could grant
`store_manager` tenant-wide if that fit how they run the business. The
permission set is the same either way; only the grant's `store_id`
changes what it actually reaches.

A tenant can also define its own roles (`atlas.roles` with `tenant_id`
set, `is_system = false`) - e.g. a title between cashier and manager -
picking any subset of the same fixed permission enum. There's no UI for
this yet; it goes through the same `INSERT` shape as the seed data.

---

## 9. What this doesn't do yet

- **`store_group_grants`** (cross-store access for one grant without
  going full tenant-wide - "regional manager over 3 of my 5 stores") -
  the table exists and `assert_permission`/`assert_grantable` already
  join against it, but nothing writes to it yet. Deferred to v2 per
  `docs/v2.md`; the FK shape is settled now so it slots in later without
  a schema change.
- **No admin UI for building custom roles or editing permission sets.**
  Inviting an existing `role_id` is `POST /api/v1/roles/invite`. Nothing
  inserts a custom `atlas.roles` row from a screen.
- **Tenant and store switching are built.** `POST /api/v1/auth/switch-tenant`
  re-issues the session. Store selection is the `x-store-id` header.
- **Auth routes are rate limited** per IP (`rate_limit.rs`), including
  invite and accept-invite because they sit on the auth router. Role
  invite is under `/api/v1/roles`, so it is not covered by that limiter.
