# Atlas-sms - Locked Architecture Decisions

Settled for this pass of scaffolding. Do not re-litigate without a real
reason to. The tooling is an Nx monorepo, Rust/Axum/sqlx/Postgres, Nuxt SSR,
and the TenantTx/RLS mechanism. The domain model is Atlas's own: tenants
and stores, not a school hierarchy. See `docs/atlas/README.md` for the
product-level reasoning this schema is built from.

## 1. Flat tenant -> stores, no ancestor hierarchy

Atlas's stated scale is 1-20 stores per tenant. `atlas.stores.tenant_id` is a
direct FK, no chain-walking. Cross-store access for a role grant that isn't
full tenant-wide is `atlas.store_group_grants`, not a parent/child tree.
A real franchise/holding-company hierarchy (`tenants.parent_tenant_id`) is
tracked as possible-later in `docs/v2.md`, not built pre-emptively.

## 2. Dynamic roles: `roles` + `role_grants`, not a fixed enum ladder

`atlas.roles` holds seeded system roles (`tenant_id NULL`, `is_system`)
and tenant-defined custom roles. `atlas.role_grants` is one row per
(user, role, tenant, store) - `store_id NULL` on a tenant-scope grant means
tenant-wide. A user can hold different roles at different stores. Granting
a role never lets the granter exceed their own permission set
(`assert_permission` in `roles::service` is the single generic check every
handler calls - no per-action bespoke SQL).

## 3. Own Postgres server, own schema

Atlas runs on its own Postgres server (`docker-compose.atlas.yml`).
Tables live in an `atlas` Postgres schema rather than `public`, and the
app connects as `atlas_api` (never the superuser, because a superuser
bypasses RLS).

## 4. RLS is fail-closed

`TenantTx` (Rust extractor) sets `app.current_tenant_id` /
`app.current_store_id` via `set_config` immediately after `BEGIN`; every
tenant table is `FORCE ROW LEVEL SECURITY`; an unscoped connection matches
no rows. `AdminTx` is the explicit, narrow bypass
(`app.bypass_tenant = 'on'`) for pre-tenant operations (signup) and
platform-role work that intentionally crosses tenants.

## 5. UUIDv7, app-generated

Sequential IDs would let a tenant enumerate
record counts off a public store slug. `Uuid::now_v7()` in Rust, never a
DB default.

## 6. Web app naming: `atlas-web` does both marketing and dashboard

Atlas's one Nuxt app, `apps/atlas-web`, carries both the public pages and the
authenticated owner/manager dashboard (Nitro proxy, cookie/Bearer auth,
401-retry) under one name. Deliberate choice, made explicitly, not a
default.

## 7. POS is a separate client, not a web page

Per the PRD, the point-of-sale terminal is a native/tablet app, so it is
`apps/atlas-pos` (Ionic + Capacitor), not a route inside `atlas-web`.
Login and a home screen exist: tenant switch, store selection, platform
invite, and logout, using `X-Client-Type: app`. Checkout and offline sync
are not on those screens yet. `@atlas/ui-app` is the Ionic package it can
depend on.

## 8. No CI workflow yet

No `.github/workflows/` exists in this repo yet.
CI gets wired when that work happens, not before.

## What is in the tree now

- `apps/atlas-backend` - Rust/Axum/sqlx. Auth, tenants, stores, roles,
  products, stock, sales, purchasing, customers, and payments have handlers.
  Migrations live under `apps/atlas-backend/migrations/`, including
  `20261020000024_platform_invites.sql`. The app connects as `atlas_api`.
- `docker-compose.atlas.yml` + `docker/atlas-postgres-init.sql` - Postgres
  on host port 5434, login role `atlas_api`.
- `apps/atlas-web` - public pages, auth screens, and the dashboard,
  behind a full-width header and footer. The Nitro proxy holds the
  session cookies. See `docs/atlas/auth-flow.md`.
- `apps/atlas-pos` - login and a session home screen. Checkout is not built.
- `packages/atlas-ui/{core,web,app}` - Vue UI packages. `web` and `app`
  re-export `core`.
- `docs/v2.md` - scope deliberately deferred past v1.

## What's still open

- The POS checkout screen and the offline sale sync client. The backend
  for FR-15 to FR-19 is built: catalog search, barcode lookup, quote and
  offline snapshot under `/api/v1/stores/{id}/catalog`, server-priced
  checkout (`POST /api/v1/sales`, `POST /api/v1/sync/sales`), payments,
  receipts with per-store gap-free numbering, returns and customers. No
  screen calls them yet. The cart (`addItem`, `removeItem`,
  `updateQuantity`, `applyDiscount`, `clearCart`, a running total several
  components read, payment status pending/success/failed, and persistence
  across a reload or backgrounded app via `pinia-plugin-persistedstate`) is
  the natural place to use it. Pinia is only a dependency of `atlas-web`
  today; `atlas-pos` needs it added. The till must not offer store credit
  while offline: `POST /api/v1/sync/sales` refuses it, since an offline
  till can't check the balance. Session/auth state stays a plain composable;
  it is not what Pinia is for here.
- Capacitor secure storage for the POS refresh token. It is memory-only today.
- Custom roles: no create, edit or delete endpoint and no screen. Only
  `GET /api/v1/roles` and inviting with a seeded role are built.
- `store_module_toggles` (recipes, table orders, layaway, repairs,
  delivery, loyalty) has no HTTP API, and nothing reads it yet. Price
  rules can be created and deleted but not edited in place; a change is a
  new rule, recorded in `GET /api/v1/products/{id}/price-history`.
- No payment gateway at the till: a `pending` card or transfer payment is
  confirmed by a person (`POST /api/v1/payments/{id}/status`), not a
  webhook.
- SRS v1.0 FR-07 to FR-30 have backend coverage, FR-25 and FR-27 to FR-30
  re-checked against the SRS text. Approvals (FR-09, FR-29) are approval
  policies plus held requests (`api-reference.md` section 16f); the
  cashier discount cap stays the store setting
  `max_cashier_discount_basis_points`, because a till discount is
  authorised on the spot by a `sale.discount` holder, not queued. Not
  built: email/SMS notification delivery beyond the email outbox (FR-26
  calls it optional).
- The `atlas-web` dashboard has one page. None of the backend modules
  (products, stock, transfers, purchasing, expenses, reports, approvals,
  audit, imports, notifications, settings) has a screen yet.
- No CI workflow. See item 8.

## The online storefront (approved and built)

FR-10 to FR-14 from `docs/SystemsRequirementDoc.md` (cart, wishlist,
checkout, payment gateway, promotions, invoices) was initially out of
scope per the dated SRS v1.0 (`docs/PDF/Store-Management-System-SRS-v1.0.pdf`,
which explicitly lists "online shopping storefront" as excluded from the
initial release). The owner has since separately approved it, and it is
built: the backend `shop` module (catalogue, carts, wishlists, checkout,
promotions, loyalty, shipping, staff order handling, payment and courier
webhooks) and the `apps/atlas-storefront` Nuxt app (catalogue, product,
cart, checkout, wishlist, orders, account). It is a separate app, not a
route tree inside `atlas-web`, because customer auth is a different surface
from staff auth.

Every outside provider (payment gateway, courier, social sign-in) sits
behind a trait in `integrations.rs` with a mock, the same pattern as
`EmailProvider`; a real Paystack/Flutterwave or courier adapter is
separate, later work. Rebuilt clean against `develop`, not copied from the
stale `origin/at` branch it originated on.

## The Debridgers reference-source port

`packages/{hooks,schemas,types,utils,data,api-client}` and
`libs/shared/composables/src/{payment,ratings}` are ported from
`~/Projects/React/Debridgers` (a separate, unrelated React/Remix monorepo -
a delivery/logistics marketplace with buyer/agent/admin/employee/careers
apps). None of it is Atlas's domain. Current state:

- Most of it had its content cleared to a placeholder comment but the
  files and directory shape were kept deliberately, as a structural
  head-start for Atlas's own hooks/schemas/types/utils/data/api-client
  when that work actually happens.
- `nigerian-states.ts` and `phone-dial-codes.ts` under `packages/data` were
  left intact - genuinely reusable reference data, not domain logic.
- `ratings` (both `packages/{hooks,schemas}/ratings` and
  `libs/shared/composables/src/ratings`) was left intact, untouched. It is
  not wired into Atlas yet but is expected to be built out and modified
  for real use later, unlike the rest of the port.
- `libs/shared/composables/src/payment` was not addressed in this pass -
  still an open prune-or-keep call, separate from ratings.
- The generic utility hooks that used to live in `packages/hooks`
  (debounce, media query, reduced motion) are superseded by `@vueuse/core`,
  wired in through `@atlas/ui-core` (`packages/atlas-ui/core/src/lib/responsive`)
  since that's the one package both `atlas-web` and `atlas-pos` depend on.
- The duplicate dialog engine that lived in `@atlas/ui-core`
  (`packages/atlas-ui/core/src/lib/dialog`, plus its tokens-only
  `lib/motion`) has been deleted. It was a second, unused Vue conversion of
  the same Debridgers dialog engine already live in `@atlas/ui`
  (`libs/shared/ui/src/dialog/DialogProvider.vue`, wired up in
  `apps/atlas-web/app/layouts/default.vue`) - nothing imported the
  `ui-core` copy.
- The table port is still incomplete. `libs/shared/ui/src/table/DataTable.vue`
  covers search/sort/pagination; it is missing a toolbar, loading
  skeleton, row-actions menu, mobile card view, and density toggle that
  exist in the raw source at `packages/lib/table/*.tsx`. That raw copy
  stays until those are ported, then gets cleared like the rest.
