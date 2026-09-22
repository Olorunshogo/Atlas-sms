# Context / Decisions Log

Terse record of settled decisions, so future sessions don't re-litigate them.
Not a tutorial - see `docs/jottings/TechPRD.md` (product) and
`docs/repo/ARCHITECTURE.md` (technical) for the full reasoning.

---

## Stayar / Debridgers independence

Klassivox no longer contains source code from either prior reference
project (Stayar - a rental platform; Debridgers - an agri marketplace).
Both were used only to study patterns while scaffolding Klassivox's own
stack. Raw dumps of their docs/config (`READMEStayar.md`,
`StayarMonorepoTooling.md`, `StayarFrontendReference.md`,
`StayarBackendReference.md`, `backend_task.md`, `TAILWIND_THEME.md`) have
been deleted - Klassivox has its own equivalents for everything they
covered.

If a future need arises to reference Stayar/Debridgers patterns again, go
through:

- `docs/API/atlas-api.md` - the `@atlasapi` typed client pattern
- `docs/repo/ARCHITECTURE.md` - everything else (app conventions, hooks,
  dialog engine, etc.)

Do not re-add raw dumps of either project's source or config.

---

## `@atlasui-core` is the component source of truth

`packages/atlas-ui/core` (`@atlasui-core`) holds every
cross-platform Vue component - buttons, inputs, form primitives, cards,
etc. - consumed by every frontend app: `landing-page`, `atlas-web`,
`atlas-mobile`, and future `atlas-admin`.

`@atlasui-web` and `@atlasui-app` are thin re-export/chrome layers,
not parallel component libraries. They exist only for genuinely
platform-specific concerns:

- `ui-web` - desktop-only chrome
- `ui-app` - Ionic-native navigation/gestures/transitions (tab bar, action
  sheet, page transitions)

Ionic components (`IonButton`, etc.) are **not** used by default anywhere.
`ui-core` components are used everywhere unless a screen specifically needs
native gesture/transition behavior that only Ionic provides.

### Reference example: `SubmitButton`

`packages/atlas-ui/core/src/lib/submit-button/submit-button.vue` is the
pattern to copy for new components. `color: 'brand' | 'neutral' | 'danger'`
and `variant: 'solid' | 'outline'` are both **required** props with no
defaults, so every usage explicitly locks in its look rather than silently
inheriting a default. Modeled after Stayar's `DashSubmitButton`/
`DashOutlineButton` pair, generalized into one component with a 3-color x
2-variant matrix instead of one component per color/variant combination.

---

## Docs split: PRD vs Architecture

`docs/jottings/TechPRD.md` is the product PRD (roles, modules, phased scope,
business rules, non-functional requirements) - what Klassivox does.

`docs/repo/ARCHITECTURE.md` is the technical decision log (tech stack,
app/package boundaries, frontend conventions, API design, bootstrap order) -
how Klassivox gets built.

Don't mix the two again. Product content goes in the PRD; engineering
decisions go in ARCHITECTURE.md.

---

## Root `package.json` script/dependency cleanup: partial, follow-up flagged

Removed `atlas-backend:generate`/`:seed`/`:drop`/`:migrate`/`:studio`/
`:run-seed` - these called `nx run atlas-backend:db --args=...`, but
`atlas-backend`'s `project.json` has no `db` target and never has (it's
Rust/Axum/sqlx, not the Drizzle/NestJS stack these commands assume - `sqlx`
migrations use `sqlx migrate add/run`, not `generate`/`studio`). These
scripts were dead on arrival. Real sqlx-based migration scripts should be
added once `apps/atlas-backend` actually has a `migrations/` folder.

Fixed `build:ui-web`/`watch:ui-web`/`storybook:ui-web` pointing at scripts
that moved to `@atlasui-core` when `ui-web` became a thin layer -
renamed to `build:ui-core`/`watch:ui-core`/`storybook:ui-core` and gave each
of `ui-core`/`ui-web`/`ui-app` their own `build`/`watch` scripts.

**Follow-up now done:** removed root `devDependencies` `@nestjs/schematics`,
`@nestjs/testing`, `@nx/nest`, `@nx/webpack`, `@nx/jest`, `drizzle-kit`,
`jest`, `jest-environment-node`, `jest-util`, `ts-jest`, `@types/jest`,
`@swc/jest`, `webpack`, `webpack-cli`, `tsconfig-paths-webpack-plugin`,
`@types/bcrypt`, `@types/express`, `@types/pg`, `@types/nodemailer`,
`@types/ms`, `chokidar-cli`, `concurrently`, `fast-check`, `ts-node`, `tsx`

- all confirmed zero source references anywhere outside `package.json`/the
  lockfile via repo-wide grep before removal. Removed the matching
  `@nx/webpack/plugin` and `@nx/jest/plugin` blocks from `nx.json` and deleted
  the now-pointless root `jest.config.ts`/`jest.preset.js` (Nx auto-discovery
  stubs with zero actual per-project jest configs anywhere - vitest, via
  `@nx/vite`/`@nx/vitest`, is the only real test runner in this repo). Also
  deleted `libs/shared/ui` (the empty `export {};` stub superseded by
  `@atlasui-core`) and its dangling `@atlasshared/ui` path alias in
  `tsconfig.base.json`, and added the missing `@atlasui-core`/
  `@atlasvalidation`/`@atlascomposables` aliases that were never
  added when those packages were created.

Also added `fix:lockfile` (`pnpm clean --lockfile && pnpm install`) as a
root script - the correct remediation when `pnpm install` rejects the
lockfile on `minimumReleaseAge` grounds (see ARCHITECTURE.md's dependency
management section). Don't relax the policy; re-resolve instead.

---

## Comment convention: repeat mistake to stop making

`CLAUDE.md`'s "Comment Style" section already spells this out, but it was
violated multiple times across a single session even after being corrected
once, so it's repeated here as a hard rule, locked in, not up for
re-litigating: **one line stays `//`. The moment a comment needs a second
line, stop writing more `//` lines and switch to a block comment instead**,
with each continuation line prefixed by `*` and aligned under the opening
line. This applies in every language this repo touches - TypeScript/
JavaScript/Vue/Rust all use the JSDoc-style `/** ... */` shape. SQL
(`kyc-schema-api.md`'s migrations) uses plain `/* ... */` instead, since SQL
has no doc-comment convention to match. JSONC config files (`tsconfig*.json`
etc., which tolerate the same comment syntax as JS) follow the JS rule too -
file extension and language are never an exception, only SQL's lack of a
doc-comment syntax changes the block style used.

```ts
// WRONG - two or more // lines strung together as one explanation
// Some multi-sentence explanation that
// keeps going on a second line.

/**
 * RIGHT - switch to a doc-comment block the moment it's genuinely
 * multi-line.
 */
```

```rust
// WRONG - same mistake, Rust syntax
// refresh_token is NOT a JWT - an opaque random token stored (hashed) in a
// `refresh_tokens` table with a 7-day expiry, so it can be revoked.

/**
 * RIGHT - Rust follows the same JSDoc-style block as TS/JS, not plain
 * slash-star, once a comment is genuinely multi-line.
 */
```

A single, genuinely one-line `//` comment (including a `// === Heading`
divider) is always fine as-is - this rule only kicks in once a comment
needs a second line. Before finishing any task that touches comments,
grep for 2+ consecutive `//`/`--` lines across the changed files and fix
any hit, rather than waiting to be told again.

---

## `docs/repo/NEXT.md`: verified state doc, not a `PLAN.md` replacement

`PLAN.md`/`TASKS.md`/`KPI.md`/`ARCHITECTURE.md` were written before the
`ui-core`/`composables`/`validation`/`atlas-mobile`-scaffold infra
session landed, so a follow-up session cross-checked their claims against
the actual repo and wrote `docs/repo/NEXT.md` to record the result: the
backend is still a health-check stub with no `migrations/` folder, both
Nuxt apps have only placeholder pages, `libs/api` doesn't exist as code
despite `docs/API/atlas-api.md` being a complete spec, and
`atlas-web`'s `server/api/[...path].ts` Nitro proxy is real and already
does cookie-read/Bearer-inject/401-retry - ready to be called the moment
Auth exists on the backend. None of this contradicted `PLAN.md`/`TASKS.md`;
it confirmed them and filled in specifics they didn't have yet. The
concrete next task is still Auth (Module 1), backend-first, per `PLAN.md`
Week 3-4 - `NEXT.md` just spells out the exact steps and prerequisites
(first `sqlx` migration, JWT/password-hashing crates not yet in
`Cargo.toml`, local Postgres via Docker Compose which also doesn't exist
yet).

`NEXT.md` also flags four deferred problems from that infra session worth
not forgetting: `landing-page`'s `useArticles.ts` composable is real but
orphaned (no page calls it) pending a content decision; `.husky/pre-push`'s
e2e step is commented-out dead code referencing a Debridgers package name,
not a Klassivox one; there's no seed-data pipeline yet. See `NEXT.md`
itself for the full detail - this entry exists so future sessions know the
doc exists and why it's separate from `PLAN.md` rather than merged into it
(kept separate so `PLAN.md`'s shipping order isn't touched by a
state-verification pass; fold the two together once Auth actually starts
shipping, per `NEXT.md`'s own closing note).

---

## State typing: always annotate, even when inferable (reversed decision)

The global convention (`~/.claude/CLAUDE.md`, applies to every project) used
to say "let inference work - don't annotate `ref(false)`/`useState(false)`
when TypeScript can already infer it." That's been reversed globally as of
2026-07-15: every piece of state (`ref`, `reactive`, `useState`, Pinia/
Zustand/Redux state fields) now gets an explicit type annotation at its
definition, even when inference would produce the same result -
`ref<boolean>(false)`, not `ref(false)`. Reason: scannability at a glance
across a large codebase was judged more valuable than avoiding the extra
characters. This applies here and in every other project, not just
Klassivox - see the global CLAUDE.md for the full rule and framework-by-
framework examples (React/Vue/Pinia/Zustand/Redux).

All `ref()` calls across this repo were swept and corrected to this rule
as of the same date.

---

## Page-scoped types: `app/types/types.ts`, organized by page

Each Nuxt app (`atlas-web`, `landing-page`) has an `app/types/types.ts`
file, organized with one `// === PageName` heading per page (matching the
page's file under `app/pages/`), with that page's local types/interfaces
exported directly beneath its heading. A type used by more than one page
moves to a `// === Shared` section in the same file instead of being
duplicated across headings. A type genuinely shared across apps
(atlas-web, landing-page, atlas-mobile) doesn't belong here at all;
it goes in a `libs/shared/*` package instead, per `ARCHITECTURE.md`'s
"Shared packages/libs" table.

Both files currently exist as scaffolds only - neither app's pages have
local types yet, so each heading is a placeholder to fill in as real
per-page state/props get added. Follow this same pattern for any future
app (`atlas-admin` when it's scaffolded).

---

## Locked architecture decisions (2026-07-31)

Settled in one session before any product code existed, which is why every
one of them was cheap. All are closed. Do not reopen without a concrete
reason that did not exist on this date.

Context for the whole set: Klassivox targets every school in Nigeria,
organized state by state, as a single source of truth. Expansion outside
Nigeria is anticipated but explicitly **not** being built for. The
operative rule is a seam, not a second implementation: make the provider,
locale, and currency swappable, then ship exactly one of each.

### 1. UUIDv7 primary keys, generated in application code

Every table uses `id UUID PRIMARY KEY` with **no column default**.
Postgres 16 has no native `uuidv7()` (that arrived in 18) and
`gen_random_uuid()` would silently issue v4, losing the time-ordering that
keeps UUID primary keys from fragmenting their indexes. Ids come from
`Uuid::now_v7()` in Rust, so a missing id is an insert-time error rather
than a quietly unordered key.

Chosen over `BIGSERIAL` because ids appear in URLs on public per-school
subdomains, and sequential ids let any tenant enumerate `/students/1`,
`/students/2` and infer national record counts. Rejected the hybrid
(internal `BIGSERIAL` plus a public UUID column) because two identity
columns per table means one slip exposes the enumerable one.

### 2. Organizations are a hierarchy, not a flat level

`organizations.parent_organization_id` is nullable and self-referencing.
Authorization walks ancestors with a recursive CTE instead of assuming one
hop from a school to its owning organization.

Reason: a diocese, a multi-state chain, and a state ministry all break a
single-hop check, and all three exist domestically. Writing the walk now
is roughly ten lines; retrofitting it after `PermissionService` ships is a
rewrite of the whole authorization layer.

State ministries will get a separate `oversight_grants` table rather than
being modeled as ancestor organizations, because an ancestor org confers
unconditional write access into every school beneath it and a regulator
should read without writing. Not built yet.

### 3. School slugs are state-prefixed

`lagos-unity-college.grandek.com`, not `unity-college.grandek.com`.
`schools.state_id` references a `states` lookup table seeded with all 36
states plus the FCT.

Reason: `schools.slug` is globally unique, so nationally the first school
named "Government Secondary School" would take that name from every other
one. Prefixing removes the land grab and encodes geography for free, which
suits the state-by-state model. Geography is a table rather than an enum
so a state can be added or renamed without `ALTER TYPE`, and the
`country_code` column exists purely so a non-NG row is possible later. No
code branches on it.

### 4. `user_roles.unique_id` is unique per organization, not globally

`UNIQUE (organization_id, unique_id)`, replacing a global `UNIQUE`.

The global constraint was a live bug, not a scale concern.
`identifier_formats` and `identifier_sequences` are both per-organization,
so two organizations on the same default template both generate
`2026/00001` and the second organization's first-ever signup fails.
Verified against Postgres: both rows now insert, and a duplicate within a
single organization is still rejected.

Consequence: login cannot resolve a `unique_id` without knowing the
tenant, so the subdomain (already sent by the proxy as `x-school-slug`) is
now required for identifier-based login rather than optional.

### 5. RLS is adopted, and handlers cannot opt out

Row Level Security is enabled and **forced** on every tenant table.
Policies read `app.current_organization_id` and `app.current_school_id`,
set per request with `set_config(..., true)` inside a transaction.

Service-layer scoping stays the primary mechanism. RLS is the seatbelt
underneath, so a query that forgets its tenant predicate returns nothing
instead of another school's rows. At national scale a single missed
`WHERE` is a countrywide breach, which is what justifies the cost.

The cost is real and worth stating plainly: **every request touching
tenant data must open a transaction**, because pooled connections are
reused and the setting has to be transaction-local.

Two non-negotiable details:

- `FORCE ROW LEVEL SECURITY`, not just `ENABLE`. Without it the table
  owner is exempt from its own policies, and the owner is who runs
  migrations, so protection would be absent wherever the app connects as
  the owner.
- The app connects as `klassivox_api`, a non-superuser login role granted
  the privilege-holding `klassivox_app` role. A superuser bypasses RLS
  entirely and would make every policy decorative.

Enforcement is structural rather than cultural: handlers receive a
`TenantTx`, never a raw pool connection. A handler that cannot obtain a
raw connection cannot write an unscoped query.

Verified against a live Postgres on 2026-07-31: with no tenant context set
the connection sees zero schools (fails closed), and scoped to each of two
organizations it sees exactly that organization's school.

`platform_admin` bypasses via `app.bypass_tenant = 'on'`, set only after a
platform-role check has already passed in application code.

`users` and `states` carry no policy on purpose. `users` is not
tenant-scoped, since one human holds role grants across many
organizations, and is reachable only through the policed `user_roles`.
`states` is public reference data.

### 6. Host-only cookies, one session per school

Session cookies are host-only, scoped to a single school subdomain, not
shared across `.grandek.com`.

Isolation by default is the safer failure mode for children's data. A
parent with several children **in one school** sees all of them from one
login, via a child list in the parent panel sorted by class or level.
A parent with children in **different** schools logs in at each school's
subdomain separately. That friction is accepted: cross-school parents are
a minority, and the alternatives (a signed cross-subdomain handoff, or a
shared `parents.grandek.com` hub) each add a cross-tenant authentication
path, which is the highest-risk surface in the entire product.

### 7. Client-declared token delivery

`/auth/login` and `/auth/refresh` shape their response from an
`X-Client-Type` header. `web` (set by the Nitro proxy) gets `Set-Cookie`
and **no tokens in the body**. `app` (set by `@atlasapi` in mobile
mode) gets tokens in the body and **no cookies**.

Why not the alternatives:

- Cookie only: web is perfect, mobile is impossible. A Capacitor webview
  runs on a `capacitor://` or `localhost` origin talking cross-site to the
  API, where HttpOnly cookie handling is unreliable at best.
- Body only: mobile works, web has to hold the token in JavaScript, which
  discards the HttpOnly guarantee and turns any XSS into account takeover.
- Both unconditionally: works, but puts tokens in the response body on web
  too, where SSR payload serialization, logging, or an error reporter can
  capture them. That quietly undoes the benefit of the cookie path.

Two constraints that are part of the decision:

- `X-Client-Type` is a response-shape hint and **never** a permission
  boundary. Neither mode may ever grant more than the other. Worst case a
  client requests the wrong shape and receives a token it cannot use.
- Mobile stores the refresh token in Capacitor secure storage and keeps
  the access token in memory only. Never `localStorage`.

A fourth client later (`atlas-admin`, a partner API, a state ministry
integration) is one new variant, not a second auth system.

### 8. Per-school branding is runtime CSS variables (depth level 2)

Schools differ by uniform, colour, hue, and logo, so the public page must
look different per school. Four depths were considered:

1. Data only (logo and photos differ, look identical). Documented, not
   built.
2. **Token theming. Chosen.** The school supplies an accent hue and a
   logo; SSR reads the tenant row and emits CSS variable overrides on the
   shared tokens. Same components, same layout, genuinely different feel.
   Backed by `schools.brand_color`.
3. Template choice, a school picking among N public-page layouts.
   Documented and schema-reserved via `schools.public_template` defaulting
   to `'default'`. Not built.
4. Fully custom markup or a per-school CMS. **Rejected permanently. Do not
   raise it again.**

The rule that follows and is not optional: **no component ever hardcodes a
Tailwind colour.** Every colour resolves through a token. Level 2 is
nearly free today only because `THEME.md` already defines everything as
CSS variables; it becomes a rewrite the moment a compile-time colour ships
in a shared component.

### 9. Monorepo stays, and boundaries are enforced by lint

Number of schools is a runtime and database property. The repository has
no idea how many tenants exist, so 20,000 schools is not a monorepo
argument. Monorepo cost scales with packages and developers, and four apps
plus five libs is far inside what Nx handles comfortably.

Splitting into per-app repos would actively defeat the goal of one shared
input component, because each app would pin its own published version of
`@atlasui-core` and drift. Reconsider at roughly 30+ engineers or a
contractor who must not see backend code, not at a tenant count.

`@nx/enforce-module-boundaries` is switched on with tags, so an app
defining its own input component or importing another app's internals is a
**lint failure**, not a convention. Turned on before the first real
component, because retrofitting tags across a grown tree is the painful
version.

Placement rule: **if a second app could plausibly need it, it does not
live in an app.**

| Lives in                 | Holds                                                             |
| ------------------------ | ----------------------------------------------------------------- |
| `@atlasui-core`     | every visual component: inputs, buttons, form primitives, cards   |
| `@atlasui-web`      | desktop-only chrome                                               |
| `@atlasui-app`      | Ionic navigation chrome                                           |
| `@atlasutils`       | formatters: date, currency, phone, address. Pure TS, no framework |
| `@atlasvalidation`  | zod schemas                                                       |
| `@atlascomposables` | Vue composables                                                   |
| `@atlasapi`         | the typed client                                                  |
| `@atlastheme`       | design tokens                                                     |
| the app itself           | pages, routes, layouts, app-specific glue, nothing else           |

### 10. Providers are adapters, never core

Backend defines `PaymentProvider`, `NotificationProvider`,
`StorageProvider`, and `EmailProvider` traits. Paystack, Termii, and R2
are implementations selected at startup from config.

Hard rule: **no handler ever imports a provider directly.** Handlers
depend on the trait. Swapping Paystack for Flutterwave becomes one new
module plus a config value.

Same principle on the frontend. Every formatter takes its context
explicitly rather than assuming Nigeria:

```ts
formatCurrency(amount, { currency: 'NGN', locale: 'en-NG' });
formatDate(value, { locale: 'en-NG', timeZone: 'Africa/Lagos' });
```

The Nigerian default lives in one config constant, never baked into a
function body. `schools` carries `timezone`, `locale`, and `currency`
columns for the same reason, defaulted to `Africa/Lagos`, `en-NG`, `NGN`.
Nothing branches on them yet, and nothing should until a real second case
exists.

Money is stored as `amount_minor` plus an explicit `currency` column.
`amount_kobo` hardcoded a currency into a column name and is retired.

### 11. Tenant resolution is explicit, custom domains are reserved

`host.split('.')[0]` in the Nitro proxy is replaced by a `resolveTenant`
helper with a reserved-subdomain list (`www`, `api`, `admin`, `app`,
`static`, `assets`, `mail`). Today that expression sends `"grandek"` as a
school slug on the apex and `"localhost:3000"` in development. An unknown
slug returns a real 404 school-not-found page rather than falling through
to marketing. Development uses `<slug>.localhost`.

Custom domains are out of scope. `schools.custom_domain` exists as a
`UNIQUE` nullable column so availability is checkable at the schema level
and two schools can never claim the same host, but no resolution code
reads it. Most Nigerian schools have no domain, and the subdomain is part
of what Klassivox is selling them.

The public page is gated by `schools.is_published`, defaulting to false.
Without it a pending signup could publish a crawlable page under a real
school's name.

### 12. Development and operations

- **`pnpm nx docker atlas-backend`**: `nx reset`, then Postgres and
  Redis containers, then the backend container. Container Postgres is on
  host port **5433**, deliberately not 5432, so it coexists with the
  developer's own local Postgres rather than conflicting with it.
- **`pnpm nx dev atlas-backend`**: `nx reset`, then `sqlx migrate
run`, then `cargo run` against the developer's local Postgres on 5432.
- Both start with `nx reset` by explicit request. Worth knowing: this
  clears the Nx cache and stops the daemon, so each start pays a cold
  start and the next command loses caching.
- **Seeds**: SQL fixtures under `apps/atlas-backend/seeds/`, driven by
  an environment-gated `cargo run --bin seed`. A Rust binary rather than
  raw SQL because seeds need argon2 hashing and generated `unique_id`s
  that must go through the real identifier logic. Reusing the real code
  path is what keeps seed data structurally identical to production data.
- **CI/CD**: GitHub Actions, four tiers. (1) PR checks: `nx affected` lint,
  typecheck, vitest; `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test`; migrations against an ephemeral Postgres plus `cargo sqlx
prepare --check`. (2) A guard that fails any PR editing an
  already-applied migration, since migrations are append-only. (3) Deploy:
  main to staging automatically, production behind manual approval. (4)
  Post-deploy smoke test, `cargo audit`, `pnpm audit`.

CI matters here specifically because sqlx checks queries at compile time
against a real schema, so a migration that breaks a query is only caught
with a live database; because a change to `ui-core` can break three apps
at once; and because at national scale a bad deploy is a simultaneous
outage for every school.
