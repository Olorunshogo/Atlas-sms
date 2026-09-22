# Atlas-sms - Locked Architecture Decisions

Settled for this pass of scaffolding. Do not re-litigate without a real
reason to. Where a decision differs from Klassivox's own `CONTEXT.md`, the
difference is deliberate and explained - Atlas reuses Klassivox's *tooling*
(Nx monorepo, Rust/Axum/sqlx/Postgres, Nuxt SSR, the TenantTx/RLS
mechanism), not its domain model. See `docs/Atlas/README.md` for the
product-level reasoning this schema is built from.

## 1. Flat tenant -> stores, no ancestor hierarchy

Unlike Klassivox's self-referencing `organizations.parent_organization_id`
(justified by diocese/state-ministry chains at national scale), Atlas's
stated scale is 1-20 stores per tenant. `atlas.stores.tenant_id` is a
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

Atlas runs on its own Postgres server (`docker-compose.atlas.yml`),
entirely separate from Klassivox's. Tables additionally live in an `atlas`
Postgres schema rather than `public`, and the app connects as `atlas_api`
(never the superuser - RLS is bypassed entirely for a superuser
connection). Both layers of separation exist so a bug or incident in one
product has no path to the other's data, even if the two ever end up on
one instance.

## 4. RLS is the same fail-closed mechanism, renamed

`TenantTx` (Rust extractor) sets `app.current_tenant_id` /
`app.current_store_id` via `set_config` immediately after `BEGIN`; every
tenant table is `FORCE ROW LEVEL SECURITY`; an unscoped connection matches
no rows. `AdminTx` is the explicit, narrow bypass
(`app.bypass_tenant = 'on'`) for pre-tenant operations (signup) and
platform-role work that intentionally crosses tenants.

## 5. UUIDv7, app-generated

Same reasoning as Klassivox: sequential IDs would let a tenant enumerate
record counts off a public store slug. `Uuid::now_v7()` in Rust, never a
DB default.

## 6. Web app naming: `atlas-marketing` does both marketing and dashboard

Unlike Klassivox's split (`landing-page` vs `atlas-web`), Atlas's one
Nuxt app, `apps/atlas-marketing`, carries both the public pages and the
authenticated owner/manager dashboard (Nitro proxy, cookie/Bearer auth,
401-retry) under one name. Deliberate choice, made explicitly, not a
default.

## 7. POS is a separate client, not a web page

Per the PRD, the point-of-sale terminal is a native/tablet app. The closer
existing analogue is `atlas-mobile` (Ionic + Capacitor) than the web
dashboard. `apps/atlas-pos` itself is not yet scaffolded - no screens exist
yet, and an empty Capacitor shell with no actual POS flow is speculative
scaffolding rather than useful structure. Build it when the offline-sync
API contract (`docs/atlas-sms-schema-notes.md`) has a first real endpoint
to call. `@atlas/ui-app` (the package it will depend on) is already in
place, since the UI kit's three-package split is infrastructure, not a
speculative feature.

## 8. No CI workflow yet

Klassivox's own `ARCHITECTURE.md` documents a 4-tier CI pipeline as intent,
but no `.github/workflows/` exists in this repo yet for either product.
Atlas doesn't invent one unilaterally; it gets wired in alongside
Klassivox's, when that work happens, not before.

## What shipped in this scaffolding pass

- `apps/atlas-backend` - Rust/Axum/sqlx crate, compiles and its health
  check test passes. `tenants`, `stores`, `roles` modules with real
  (not stubbed) `TenantTx`-scoped handlers, backed by six migrations
  (schema, users, tenants, stores, roles/grants, RLS).
- `docker-compose.atlas.yml` + `docker/atlas-postgres-init.sql` - Atlas's
  own Postgres server, port 5434.
- `.env.example` - `ATLAS_*` variables appended.
- `apps/atlas-marketing` - Nuxt app structure (pages/layouts/types,
  Nitro proxy) mirroring atlas-web's conventions. Not yet
  `pnpm install`-ed or run.
- `packages/atlas-ui/{core,web,app}` - full three-package split mirroring
  atlas-ui's build tooling (tsup, vite, vitest, Storybook on port 6007
  so it can run alongside atlas-ui's on 6006). No components ported
  from Klassivox - `core`'s `lib/` and `components/ui/` are empty stubs
  with the directory shape in place; `web`/`app` just re-export `core`.
  Registered in `pnpm-workspace.yaml`.
- `docs/v2.md` - V2-deferred scope tracker.

## What's still open

- Auth (signup/login/JWT) - `atlas-backend` has no `auth` module yet;
  the extractors currently trust raw headers, matching the same
  documented pre-auth caveat Klassivox's `TenantTx` started with.
- `apps/atlas-pos` (POS client) - see item 7.
- Product/inventory/sales/billing schema from
  `docs/atlas-sms-schema-notes.md` - not yet migrated.
- `pnpm install` across the new packages, and a first `pnpm dlx nuxi`
  verification that atlas-marketing actually boots.
