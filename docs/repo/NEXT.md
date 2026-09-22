# What's Next - Klassivox

The single state-and-sequencing doc. Absorbs the old `PLAN.md` (weekly
shipping order) and `TASKS.md` (backlog), both deleted on 2026-07-31
because three docs disagreeing about what was done was costing more than it
was worth.

Division of labour across the surviving docs:

- **[CONTEXT.md](CONTEXT.md)** - settled decisions. Why things are the way
  they are. Do not re-litigate.
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - technical design. How the system
  is built.
- **[TechPRD.md](../jottings/TechPRD.md)** - product. What Klassivox does, for whom.
- **this file** - what is actually built right now, and what to do next.
- **[KPI.md](KPI.md)** - targets and the condensed quarterly scorecard.

---

## 1. Actual current state

Verified against the repo, not inferred from docs.

### Built and working

- **Database schema.** Six migrations under
  `apps/atlas-backend/migrations/`: `users`, `organizations`,
  `schools` (plus a `states` lookup seeded with all 36 states and the
  FCT), `user_roles`, `identifier_formats`/`identifier_sequences`, and
  RLS. **Applied to a live Postgres 16 on 2026-07-31 and verified**, which
  closes the long-standing "never run against a real database" caveat.

  What was verified, not just written:

  - All six apply cleanly in order.
  - RLS **fails closed**: a connection with no tenant context set sees
    zero schools. Scoped to each of two organizations, it sees exactly
    that organization's school and no other.
  - Two schools named "Unity College" coexist in Lagos and Kano via
    state-prefixed slugs.
  - Two organizations both generate `unique_id` `2026/00001` and both
    insert, while a duplicate within a single organization is rejected.
    This was a real bug under the old global `UNIQUE`.
  - The recursive ancestor walk reaches a diocese three levels above a
    school, and the self-parent CHECK rejects a trivial cycle.

- **Local infrastructure.** `docker-compose.yml` runs Postgres (host port
  **5433**, so it coexists with a developer's own Postgres on 5432) and
  Redis, both with healthchecks. `docker/postgres-init.sql` creates the
  non-superuser `klassivox_api` login role that RLS depends on.

- **Nx targets.** `nx docker atlas-backend` (containerized) and
  `nx dev atlas-backend` (local Postgres), plus `migrate` and
  `migrate:revert`. Both start with `nx reset` by request.

- **Module boundaries.** `@nx/enforce-module-boundaries` now has real
  `depConstraints` (it previously had a `*` to `*` rule that enforced
  nothing) and every project is tagged. An app importing another app, or
  defining what belongs in `ui-core`, is a lint failure.

- **The Nitro proxy.** `apps/atlas-web/server/api/[...path].ts` reads
  the auth cookies, injects `Authorization: Bearer`, forwards with
  `x-school-slug`, and retries once through `/auth/refresh` on a 401. Real,
  working, and waiting on a backend to talk to.

- **Shared packages.** `@atlasui-core`, `@atlascomposables`,
  `@atlasvalidation`, `@atlastheme`. Real, with no consumers
  beyond placeholder pages.

### Not built

- **`atlas-backend` is still a `GET /health` stub.** `src/lib.rs` is
  one route. No auth, no handlers, no `TenantTx`, no provider traits. The
  schema underneath it is real; nothing reads it yet.
- **Both Nuxt apps are placeholders.** `atlas-web` has `index.vue` and
  a contentless `dashboard.vue`. No layouts, no RBAC middleware, no forms.
- **`libs/api` (`@atlasapi`) does not exist as code**, despite
  [atlas-api.md](../API/atlas-api.md) being a complete spec.
- **`@atlasutils` does not exist**, though decision 10 requires it as
  the home for locale-aware formatters.
- **No CI.** Only local Husky hooks, which are opt-out by design.
- **`atlas-mobile`** builds and has no real screens.

---

## 2. The next concrete task: Auth (Module 1), backend-first

Unchanged in ordering, but the prerequisites list is much shorter than it
was, because the schema and its verification are now done.

In order:

1. **`TenantTx` and the provider traits, before any handler.** This is the
   one thing that must come first. RLS policies read transaction-local
   settings, so every tenant-touching request opens a transaction and sets
   `app.current_organization_id`. Handlers must receive a `TenantTx` and
   must not be able to obtain a raw pool connection, because that
   impossibility is the enforcement mechanism. Writing handlers first and
   retrofitting the wrapper means auditing every query by hand.
2. **`POST /auth/signup`** - the `registration_type: organization | school`
   fork, per [auth.md](../backend/auth.md).
3. **`POST /auth/verify-email`, `POST /auth/resend-verification`** - OTP.
4. **`POST /auth/login`** - scoped by `organization_id`/`school_id`, with
   `409 ROLE_NOT_FOUND`. Note that identifier login now **requires** tenant
   context, since `unique_id` is unique per organization rather than
   globally.
5. **`POST /auth/refresh`** - the proxy already calls this on a 401 and
   currently gets a 404. First real end-to-end correctness check: hit the
   proxy with an expired cookie and confirm the retry path works.
6. **`POST /auth/add-role`, `forgot-password`, `reset-password`.**

Both 4 and 5 must implement client-declared token delivery
(`X-Client-Type`) from the start, not as a later mobile retrofit.

**Still missing, needed before step 2:**

- `sqlx-cli` is not installed in the current environment. The migrations
  were verified through `psql` directly. Install it before relying on the
  `migrate` targets.
- `jsonwebtoken` and `argon2` are not yet in `Cargo.toml`.
- No seed pipeline yet. Decision 12 settles the shape (SQL fixtures driven
  by an environment-gated `cargo run --bin seed`); it is not written.

Once auth endpoints exist, the first real frontend work unblocks: a login
page in `atlas-web` replacing the placeholder, consuming the endpoints
through `@atlasapi`.

---

## 3. Explicitly not next

- **`atlas-admin`.** No user until `platform_admin` review queues have
  something to review.
- **`atlas-mobile` screens.** Revisit once web Auth plus one full role
  path works.
- **Attendance, Report Cards, Billing, Timetable.** Gated behind Auth,
  Role, Student, Academic.
- **Sharding, partitioning, read replicas, OpenTelemetry, offline sync,
  i18n translation.** All decided in principle, none built. See
  ARCHITECTURE.md's promotion path. Building any of them now is the
  over-engineering risk, not prudence.
- **CI.** Worth landing in the same window as Auth, but it is parallel
  infrastructure rather than a blocker.

---

## 4. Known deferred problems

1. **`landing-page`'s `useArticles.ts` is orphaned.** Real, working code
   fetching `/api/medium` with a local fallback, called by no page. Pending
   a content decision. Do not build more around it until that decision
   exists.
2. **`@atlasapi` and `@atlasutils` are specced with zero code.**
   Both become blockers the moment the frontend needs to call auth.
   Scaffold alongside the Auth backend work rather than after, and do not
   let a raw `$fetch` stopgap become permanent.
3. **`.husky/pre-push` references a Debridgers package.** A commented-out
   e2e step pointing at `@debridgers/debridgers-backend-e2e`. Delete the
   reference rather than renaming it to a Klassivox equivalent that also
   does nothing.
4. **`oversight_grants` is designed but not built.** State ministries need
   read-only supervision that ancestor organizations cannot express,
   because an ancestor confers unconditional write access. Not needed until
   a ministry is actually onboarded.
5. **Cycle prevention in the org hierarchy is application-side.** The
   CHECK constraint blocks only the trivial self-parent case. A longer
   cycle must be rejected in code when a parent is assigned, since Postgres
   cannot express that declaratively.
