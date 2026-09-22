# Klassivox - Architecture & Technical Decision Log

This is the canonical technical decision log for Klassivox: tech stack choices,
application/package boundaries, frontend conventions, system architecture, API
design, and the bootstrap order for scaffolding new apps/packages. It answers
"how does Klassivox get built" - not "what does Klassivox do." For the
product side (roles, modules, phased scope, business rules), see
[TechPRD.md](../jottings/TechPRD.md).

This file was split out of `docs/jottings/TechPRD.md`, which originally mixed
product requirements and engineering decisions together. Content here is
carried over largely verbatim from that split, lightly edited to stand alone.

---

## 1) Tech Stack Decision Log

Frontend: Nuxt 4 (built on Vue 3 + Nitro). Supersedes the earlier
Remix/React Router v7 choice - re-evaluated once it became clear this was
greenfield scaffolding with no committed product code, and the team's deeper
fluency is in Vue/Nuxt. Nitro has a mature, first-class Cloudflare
Pages/Workers preset (`nitro.preset: 'cloudflare-pages'`), built edge-first
rather than retrofitted, which is a stronger match for this repo's deployment
target than React Router v7's younger `@react-router/cloudflare` adapter.
Nuxt gives the same SSR, nested layouts (`layouts/` + route middleware for
RBAC gating), co-located data fetching (`useFetch`/`useAsyncData`), and
progressive enhancement (critical for low-bandwidth Nigerian users - see
the PRD's non-functional requirements) that the Remix choice was chasing.
Frontend apps are fully TypeScript, no exceptions.

Backend: Rust (supersedes the NestJS/TypeScript recommendation in
`TypescriptRust.md` - that doc's caution no longer applies; this is now a
committed decision, not a threshold to hit later). Web framework: Axum
(tokio-based, async, the same ecosystem `RustProjectIdeas.md` already
pointed at). Database access: `sqlx` over an ORM like SeaORM - `sqlx` is the
closer philosophical match to the original Drizzle choice (compile-time
query verification against the real schema, no heavy runtime abstraction,
close to raw SQL). Migrations run via the `sqlx` CLI rather than Drizzle Kit.

API: REST (chosen over GraphQL). This domain is mostly predictable CRUD with
clear resource boundaries. GraphQL adds schema/resolver overhead, harder
RBAC auditing, and caching complexity for a small team. A narrow GraphQL
reporting endpoint can be added in Phase 3 if needed.

Multi-tenancy (implementation): row-level isolation (`school_id` on every
table + RLS). Operationally simplest for 1-1,000+ schools, avoids migration
hell, and can be promoted to schema-per-tenant later without downtime. (For
the product-level tenancy _model_ - organization/school hierarchy, roles -
see the PRD.)

Extra modules added: Timetable (conflict detection), events/calendar,
document/certificate generation, audit logs, bulk operations,
Nigeria-specific integrations (Paystack + Termii). These are non-negotiable
for a real school product.

Overall mindset: Modular, stateless, observable, and built so a dev team can
start committing code tomorrow. No over-engineering, but every decision
leaves growth paths open.

### Subdomain model

- Preferred school access: `<school-slug>.grandek.com`
- Optional org context: `<org-slug>.grandek.com` for multi-school command center
- `atlas-admin` (platform roles) sits on one fixed internal subdomain (`admin.grandek.com`), not a per-tenant slug
- This differs from a per-role-app subdomain model (each app gets one fixed URL): here, subdomains are per-tenant-customer (school/org), resolved dynamically after login, because every school/org needs independent SEO/crawlability, not just the marketing site

### Tech Stack Recommendation & Justification

| Layer          | Choice                                                                                                                                                                                                                                   | Trade-off Justification                                                                                                                                                                                                                         |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Frontend       | Nuxt 4 (Vue 3 + Nitro + TS + Tailwind)                                                                                                                                                                                                   | SSR for SEO/landing + nested layouts for dashboards. Progressive enhancement for Nigeria. Nitro's Cloudflare Pages/Workers preset is edge-first, more mature than React Router v7's adapter. Better team fluency than Next.js/Remix.            |
| Backend        | Rust (Axum)                                                                                                                                                                                                                              | Committed decision (supersedes `TypescriptRust.md`'s earlier "stay with NestJS" recommendation). Memory safety, predictable concurrency, low resource cost per Railway instance.                                                                |
| Database       | PostgreSQL + `sqlx`                                                                                                                                                                                                                      | Compile-time checked queries against the real schema, close to raw SQL, no heavy runtime ORM - same philosophy as the original Drizzle choice, ported to Rust.                                                                                  |
| Monorepo       | Nx + pnpm (frontend) + Cargo workspace (backend)                                                                                                                                                                                         | Nx/pnpm still orchestrates shared TS libs; Nuxt apps run their own `nuxt` CLI via `nx:run-commands` rather than an Nx-native Vue plugin. The Rust backend lives in its own Cargo workspace, wired into Nx the same way (see bootstrap section). |
| Proxy          | Nitro server routes (`server/api/[...path].ts` in `atlas-web`), deployed as Cloudflare Pages Functions                                                                                                                               | No separate `proxy` app needed - Nitro's Cloudflare Pages preset deploys the app's own `server/` directory as edge functions, so the BFF/cookie-handling layer lives inside the Nuxt app that already needs it.                                 |
| Deployment     | Cloudflare (web+proxy) + Railway (API+DB+Redis)                                                                                                                                                                                          | Cost-effective, Nigeria-friendly latency. Railway builds the Rust binary via the backend's Dockerfile.                                                                                                                                          |
| State          | Nuxt `useFetch`/`useAsyncData` + Pinia (client state) + VeeValidate (forms)                                                                                                                                                              | Co-located data fetching, composable-based controlled form handling, and optimistic UI where needed.                                                                                                                                            |
| Caching/Queues | Redis (Railway) + `apalis` (Rust job queue crate)                                                                                                                                                                                        | Single Redis service for cache + background jobs, replacing BullMQ now that the backend isn't Node.                                                                                                                                             |
| Animations     | Case 1: Tailwind CSS transitions/animations (default). Case 2: JS-driven animation (staggered lists, layout/exit animations, drag) via `motion-v` or `@vueuse/motion`, optionally GSAP - not yet decided, pick when a real case comes up | Avoid bloat; CSS first. No framework committed for the JS-driven case since none of these has been needed yet.                                                                                                                                  |

Frontend stays fully TypeScript end to end (landing-page, atlas-web,
atlas-admin, atlas-mobile, and the shared UI/theme/API-client
packages). Only the backend (`atlas-backend`) is Rust.

---

## 2) Confirmed Application & Package Architecture

This section is the source of truth for app/package naming and boundaries.
Earlier drafts of this doc and the Nx scaffolding used `atlas-tenant`;
that name is retired in favor of `atlas-web` (see rationale below).

### Apps

| App                 | Role(s) served                                                             | Rendering                                                   | Status                                                             |
| ------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------ |
| `landing-page`      | public/anonymous                                                           | Nuxt 4 (Nitro) SSR                                          | scaffolded (config-only)                                           |
| `atlas-web`     | organization roles + school roles, RBAC-gated nested layouts in one app    | Nuxt 4 (Nitro) SSR                                          | planned                                                            |
| `atlas-admin`   | platform roles only (`platform_admin`, `platform_ops`, `platform_support`) | separate, intentionally lightweight app                     | planned                                                            |
| `atlas-backend` | API for all apps                                                           | Rust (Axum), own Cargo workspace, not an Nx-native language | planned                                                            |
| `atlas-mobile`  | school-facing roles (parent/student/staff), native-feeling access          | Ionic + Capacitor, client-only SPA (no SSR)                 | app shell scaffolded, no native platforms or ui-app components yet |

Why `atlas-web` and not `atlas-tenant`: "tenant" is not a role in
this product's role model (unlike Stayar, a rental platform, where "tenant"
is a literal renter role and its own app). Reusing that name also collided
with this doc's own multi-tenancy usage of "tenant" (row-level isolation via
`organization_id`/`school_id`). `atlas-web` matches this doc's original
folder-structure naming (`apps/web`) and carries no role-specific meaning,
which is correct since one app serves multiple role tiers.

Why `atlas-backend` is Rust while everything else is TypeScript: a
deliberate, committed decision (not the "introduce Rust selectively later"
threshold `TypescriptRust.md` originally proposed - that document's
reasoning is superseded, kept only for historical context). This means
`atlas-backend` sits outside Nx's TS-native tooling (no `@nx/nest`
generator, no shared `nest-lib`-style TS lib) - it's its own Cargo workspace,
wired into the Nx graph via `nx:run-commands` wrapping `cargo build`/`cargo run`
rather than an Nx-native language plugin. `@atlasapi` (the typed client
lib) stays TypeScript regardless, since every consumer of it is a TS
frontend app - it just needs to match whatever shape the Rust API's JSON
responses take.

Why `atlas-admin` is a separate app, not a role layer inside
`atlas-web`: internal platform tooling (suspend an org, view cross-org
usage, support tooling) has a different design bar and release cadence than
the customer-facing product. Keeping it separate avoids coupling internal
tooling velocity to customer-facing releases.

Why `atlas-web` stays server-rendered (Nuxt) while `atlas-mobile` is
a separate Capacitor app rather than a Capacitor wrapper around
`atlas-web`: Capacitor wraps a client-rendered SPA in a webview; it does
not cleanly wrap an SSR framework. Since every school/org subdomain needs
independent SEO (prospective parents/students must be able to find a specific
school's public page via search, not just the main marketing site), `atlas-web`
must remain SSR. Mobile access is therefore a separate, additive app that
shares component and API libraries rather than a retrofit of the web app.

### Shared packages/libs

| Package              | Purpose                                                                                                  | Consumed by                                                 |
| -------------------- | -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `@atlasui-core` | single source of truth for cross-platform Vue components (buttons, inputs, cards, form primitives, etc.) | `landing-page`, `atlas-web`, `atlas-mobile`         |
| `@atlasui-app`  | thin platform-specific layer on `ui-core`: Ionic navigation/tab-bar/action-sheet chrome only             | `atlas-mobile`                                          |
| `@atlasui-web`  | thin platform-specific layer on `ui-core`: desktop-only chrome only                                      | `landing-page`, `atlas-web`, `atlas-admin`          |
| `@atlastheme`   | design tokens, Tailwind config                                                                           | all frontend apps                                           |
| `@atlasapi`     | typed API client (mirrors Stayar's `stayar-api`)                                                         | all frontend apps, so no app hand-rolls its own fetch layer |

`ui-web` and `ui-app` are not full parallel component libraries - they exist
only for genuinely platform-specific chrome (desktop-only vs Ionic-native
navigation/gestures). Every other component lives once in `ui-core` and is
consumed directly. See `docs/repo/CONTEXT.md` for the current state of this
split and the `SubmitButton` reference example.

### Dependency management: pnpm strict mode + catalog, not hoisting

Klassivox runs pnpm's **default strict mode** (symlinked, non-flat
`node_modules`, no `node-linker: hoisted` / `shamefully-hoist` flag set in
`pnpm-workspace.yaml`). A package can only import what it explicitly
declares in its own `package.json` - it cannot reach a dependency that
happens to be installed for some other package in the monorepo, even if
both packages share a lockfile.

**Version pinning uses pnpm's `catalog:` protocol**, not root-level
installs. `zod`, `vee-validate`, and `@vee-validate/zod` are pinned once in
`pnpm-workspace.yaml`'s `catalog:` block; every package that needs one lists
it in its own `dependencies` as `"<pkg>": "catalog:"`. This gives one locked
version across the whole workspace with zero per-package version drift,
without granting any package silent access to something it didn't declare.
`pnpm add <pkg> -w` (root-level install) is reserved for repo-wide tooling
the root itself runs (eslint, prettier, husky, typescript, tsup, etc.) - it
is not a mechanism for sharing runtime dependencies across apps/packages.

**Hoisted/flat `node_modules` (`node-linker: hoisted` or
`shamefully-hoist: true`) was considered and explicitly rejected.** It's a
one-line config change, and would make dependency installs feel more like
Stayar's setup (install once, available everywhere without a per-package
declaration) - but it reintroduces phantom dependencies: a package can end
up working only because some unrelated package's dependency tree happens to
hoist the thing it needs, and that stops working silently the moment the
donor package's dependencies shift, with no error at the point of breakage.
For a project at this scale, handling KYC, billing, and multi-tenant school
data, a silent dependency-resolution failure surfacing in production instead
of at install time is not an acceptable trade for the convenience of
skipping a `package.json` line. Strict mode plus `catalog:` is the
permanent choice here, not a placeholder.

**If `pnpm install` rejects the lockfile with a `minimumReleaseAge`
violation** (a package resolved to a version published more recently than
`pnpm-workspace.yaml`'s `minimumReleaseAge` window allows - a real
supply-chain protection, not a bug), do not relax the policy and do not
hand-edit `pnpm-lock.yaml`. Run the root script `pnpm run fix:lockfile`
(`pnpm clean --lockfile && pnpm install`), which forces a fresh resolution
that respects the policy from the start.

**If `pnpm install` interactively asks to approve build scripts** (a
numbered/checkbox prompt after deleting `pnpm-lock.yaml` and reinstalling
from scratch), it means a dependency with a `postinstall`/`preinstall`
script isn't yet listed in `pnpm-workspace.yaml`'s `onlyBuiltDependencies` -
pnpm blocks unapproved install scripts by default as a supply-chain
protection, same spirit as `minimumReleaseAge`. Don't approve interactively
each time; add the package name to `onlyBuiltDependencies` once so the
install is non-interactive going forward. `vue-demi`'s postinstall (Vue
2/3 detection, harmless) was the one missing entry as of this writing.

**If `pnpm install` hangs** (0% CPU for several minutes, no error, no
progress), it has been `nuxt prepare` running as `atlas-web`'s or
`landing-page`'s `postinstall` script in this environment specifically -
not a general problem with those apps, just this sandbox's networking.
Kill the stuck process and rerun with `pnpm install --ignore-scripts`,
which skips `nuxt prepare` entirely. That's safe: `nuxt prepare` only
regenerates `.nuxt/`'s type-check scaffolding, and `nuxt dev`/`nuxt build`
regenerate it automatically the first time either app runs, so nothing is
lost by skipping it at install time.

### Shared modal/dialog engine

`KlassivoxDialog`: a context-based modal engine modeled on Stayar's
`StayarDialog` (`packages/stayer-ui/app/src/contexts/StayarDialog.tsx`), which
bundles loading/error/success state into the modal contract itself (paired
components: `DialogContent`, `DialogHeader`, `DialogLoader`,
`DialogErrorBanner`, `DialogSuccessPanel`). This satisfies this doc's own
"Reusability and UI State Standards" (Section 3 below) for centralized modal
infrastructure with a predictable async state contract, rather than each app
building its own modal primitive. Lives in `@atlasui-core`.

---

## 3) Frontend Architecture Convention: Hooks Own Logic, Pages Stay Thin

Captured from Stayar-V2's actual code (`packages/stayer-ui/app/src/hooks/`,
`.../pages/`), since this is the pattern Klassivox's frontend apps
(`atlas-web`, `atlas-admin`, `atlas-mobile`) should follow, not
just a stylistic preference. Full source examples are preserved in
`docs/frontend/dialog.md`'s sibling reasoning and below.

### Convention 1: one hook per business capability, not per component

Every auth/business flow gets its own hook, named `use<Capability>Service`
(exception: pure utilities like `useCountDown`, `formatNaira` don't need the
`Service` suffix - that suffix specifically marks "this hook talks to the
API and owns a flow", not "this hook is a small reusable utility"). Stayar's
actual set: `useLoginService`, `useSignUpService`, `useForgotPasswordService`,
`useResetPasswordService`, `useEmailVerificationService`, `useLogOutService`,
`useGoogleAuthentication`, `useRoleChangeService`.

A service hook owns, in one place:

1. **Form state + validation** - `react-hook-form` + a `zod` schema via
   `zodResolver`, so a page never manually manages input state.
2. **Loading/busy state** - a local `isLoading`/`isBusy` boolean (occasionally
   an in-flight `useRef` guard to block double-submits, see
   `useSignUpService` below).
3. **The actual API call** - imported from the typed API client
   (`stayar-api` in Stayar; `@atlasapi` in Klassivox), never a raw
   `fetch` in the page.
4. **Error handling** - a shared toast hook (`useStayarToast` in Stayar;
   `useKlassivoxToast` in Klassivox) presents the error, extracted from a
   loosely-typed caught error (`err.message || err.detail || 'fallback'`).
5. **Navigation side-effects** - `history.replace`/`history.push` after
   success or on specific error codes (e.g. a `403` on login redirects to
   email verification; a `409` triggers a role-conflict dialog instead).
6. **Dialog triggers where relevant** - a hook can call `triggerDialog(...)`
   from the shared dialog engine (see `docs/frontend/dialog.md`) as part of
   its error/edge-case handling, not just from page-level click handlers.

The hook returns a small, deliberate API surface (`formMethods`,
`handleFormSubmission`, `isLoading`, plus flow-specific extras like
`continueWithGoogle`) - the page destructures exactly what it needs to wire
into JSX and nothing else.

Real example (`useForgotPasswordService`, Stayar):

```tsx
export function useForgotPasswordService() {
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const { presentToast } = useStayarToast();
  const history = useHistory();

  const formMethods = useForm<ForgotPasswordFormValues>({
    resolver: zodResolver(ForgotPasswordSchema),
  });

  const handleFormSubmission = formMethods.handleSubmit(async (formData) => {
    try {
      const email = formData.email.trim();
      setIsLoading(true);
      const { message } = await forgotPassword({ email });
      presentToast(message || 'Reset code sent to your email');
      history.push({
        pathname: '/auth/reset-password',
        search: `?email=${encodeURIComponent(email)}`,
      });
    } catch (error: unknown) {
      const err = error as { message?: string; detail?: string };
      presentToast(
        err.message ||
          err.detail ||
          'Failed to send reset email. Please try again.',
        {
          type: 'error',
        },
      );
    } finally {
      setIsLoading(false);
    }
  });

  return { formMethods, handleFormSubmission, isLoading };
}
```

A logout hook can differ in shape (no form involved) but keeps the same
principle - it owns the whole flow (clear query cache, clear local storage,
remove auth cookie, navigate away) behind one call:

```tsx
export const useLogOutService = () => {
  const history = useHistory();
  const query = useQueryClient();
  const { hideLoader, showLoader } = useStayarPageLoader();
  const { clearStorage } = useLocalStorage();
  const { removeCookie } = useCookie();
  return async (prop: { callbackPath: string }) => {
    try {
      showLoader();
      await logout().catch((error) => console.log(`sign-out Error:${error}`));
      clearStorage();
      removeCookie(COOKIE_NAMES.authUser);
      query.clear();
    } finally {
      hideLoader();
      history.replace(prop?.callbackPath);
    }
  };
};
```

**Klassivox equivalents to scaffold** (once auth endpoints exist on
`atlas-backend`): `useLoginService`, `useSignUpService`,
`useForgotPasswordService`, `useResetPasswordService`,
`useEmailVerificationService`, `useLogOutService`, and - specifically called
out as needed - **`useNotificationService`** (Stayar doesn't have a direct
equivalent by that name, but the same-shaped convention applies: own the
notification fetch/subscribe/mark-read flow behind one hook, not scattered
`fetch` calls across dashboard components). All of these live in
`@atlasui-core` (or `@atlasui-app`/`@atlasui-web` only if the
flow is genuinely platform-specific), under a `hooks/auth_hooks/` (or
`hooks/notification_hooks/`) subfolder, matching Stayar's grouping.

### Convention 2: shared toast/notification hook

Stayar wraps Ionic's toast primitive in one hook so every service hook calls
the same `presentToast(message, { type })` API rather than each page
managing its own toast state:

```tsx
export const useStayarToast = () => {
  const [present] = useIonToast();
  const presentToast = (message: string, options?: PresentToastArgs) => {
    present({
      message,
      duration: 4000,
      position: 'top',
      buttons: options?.buttons,
      icon: options?.icon,
    });
  };
  return { presentToast };
};
```

Klassivox needs a `useKlassivoxToast` equivalent in `@atlasui-core`,
with a genuinely Ionic-specific variant in `@atlasui-app` only if
`ui-core`'s default cannot express the native toast affordance, same
`presentToast(message, { type })` contract, mirroring how the dialog engine
also needs a base implementation and only a platform-specific override where
truly required (see `dialog.md`).

### Convention 3: whole pages are shared, not just components

This is the part worth stating explicitly since it's easy to under-scope:
Stayar doesn't just share buttons and inputs across apps - it shares entire
**page components**. `LogInPage`, `SignUpPage`, `WalletPage`, `SavingsPage`,
`MessagesInboxPage`, `PropertyDetailPage`, `DashboardLayout` all live inside
`@stayar/ui-app`, parameterized by a `userRole` prop (or similar) where the
page needs to branch on role. Each concrete app (`stayar-agent`,
`stayar-admin`, etc.) then has only a **thin wrapper file** in its own
`src/app/pages/` that imports the shared page/hook and wires it into that
app's router - it does not reimplement the page.

Real example (`apps/stayar-agent/src/app/pages/dashboard/Wallet.tsx`,
paraphrased from the actual import):

```tsx
import { WalletPage, useWalletService } from '@stayar/ui-app';
// ...thin wrapper: call useWalletService(), pass result into WalletPage,
// route this file at /dashboard/wallet in this app's router.
```

The `LogInPage` component itself takes `userRole` as a prop and calls
`useLoginService({ userRole, onSessionUpdate })` internally - the page is
generic across `stayar-agent`/`stayar-landlord`/`stayar-tenant`, not
duplicated three times with `role: 'agent'` hardcoded in each copy.

**What this means for Klassivox**: given the earlier architecture decision
that `atlas-web` serves both organization and school roles in one app
(not per-role apps like Stayar), the "share a page across apps" pattern
applies slightly differently here - `atlas-web` itself needs pages that
branch on role internally (e.g. a `DashboardLayout` that renders different
nav depending on `org_admin` vs `school_admin` vs `parent_guardian`), while
the cross-**app** sharing (the literal Stayar pattern) applies specifically
between `atlas-web` and `atlas-mobile` - both need the same
auth pages (`LogInPage`, `SignUpPage`, `ForgotPasswordPage`) and the same
service hooks, living once in `@atlasui-core` and consumed by both, not
reimplemented per app. `atlas-admin`, being intentionally lightweight
and platform-role-only, likely only shares the lowest-level pieces (dialog
engine, toast, buttons/inputs) rather than full pages, since its screens
(suspend an org, view usage) have no equivalent in `atlas-web`.

### Where this lives once scaffolded

```
packages/atlas-ui/core/src/
├── hooks/
│   ├── auth_hooks/
│   │   ├── useLoginService.ts
│   │   ├── useSignUpService.ts
│   │   ├── useForgotPasswordService.ts
│   │   ├── useResetPasswordService.ts
│   │   ├── useEmailVerificationService.ts
│   │   └── useLogOutService.ts
│   ├── notification_hooks/
│   │   └── useNotificationService.ts
│   └── useKlassivoxToast.ts
├── contexts/
│   └── KlassivoxDialog.ts          # see docs/frontend/dialog.md
└── pages/
    └── auth/
        ├── LogInPage.vue
        ├── SignUpPage.vue
        └── ForgotPasswordPage.vue
```

### Folder Structure (Nx Monorepo)

Actual app/package names in this repo:

```text
apps/
├── landing-page/       # Nuxt 4 (Nitro) SSR - public marketing site + local blog (@nuxt/content)
├── atlas-web/       # Nuxt 4 (Nitro) SSR - org + school roles (RBAC nested layouts) + server/api proxy to atlas-backend
├── atlas-admin/     # separate lightweight app - platform roles only
├── atlas-backend/   # Rust (Axum) API - own Cargo workspace, not Nx-native
└── atlas-mobile/    # Ionic + Capacitor client SPA - app shell scaffolded, no native platforms yet

packages/
└── atlas-ui/
    ├── core/            # @atlasui-core - cross-platform Vue components (source of truth)
    ├── app/             # @atlasui-app - Ionic-specific chrome on top of ui-core (for atlas-mobile)
    └── web/              # @atlasui-web - desktop-only chrome on top of ui-core (web apps)

libs/
├── shared/theme/        # @atlastheme - design tokens, Tailwind config
└── api/                 # @atlasapi - typed API client (all frontend apps)
```

Separation of concerns:

- Frontend apps own UI + routing (`landing-page`, `atlas-web`, `atlas-admin`, `atlas-mobile`)
- Backend owns business logic (`atlas-backend`)
- Each Nuxt app's own `server/` (Nitro) directory owns the security boundary for its authenticated calls - no separate proxy app
- Shared packages/libs provide a single source of truth (components, theme, API client)

### Reusability and UI State Standards

- Standardize async UI states for all data-driven components: `loading`, `error`, `empty`, `success`.
- Add shared primitives in `@atlasui-core` such as `PageLoader`, `SectionSkeleton`, `InlineError`, `EmptyState`, and `RetryAction`.
- Ensure every route-level screen has a predictable state contract (initial load, background refresh, mutation pending, mutation failure).
- Centralize modal infrastructure (base modal, confirm modal, form modal, destructive action modal) and reuse across modules.
- Keep components domain-agnostic where possible, then compose domain wrappers in feature modules.
- Validate accessibility in reusable components (focus trap, keyboard nav, aria labels, screen-reader messages).

---

## 4) System Architecture

### High-Level Architecture Diagram (Text)

Internet
-> Cloudflare (DNS + Edge) - Public landing site (Nuxt SSR) - Role-based dashboards (Nuxt nested layouts) - /api/\* -> Nitro server route inside the Nuxt app itself, deployed as a Cloudflare Pages Function (proxy layer, no separate `proxy` app) - Rate limiting (IP + user) - Cookie -> Bearer token injection (HttpOnly, never exposed to JS) - x-school-slug header from subdomain - x-org-slug header from subdomain or resolved school
-> Railway (Rust API, Axum, stateless) - JWT auth middleware - RBAC middleware/extractor - Ownership check (organization_id + school_id ownership) - Services -> sqlx
-> PostgreSQL (single DB, row-level tenancy)
-> Redis (cache + job queue, e.g. `apalis`, replacing BullMQ)

### Data Flow (Client -> Proxy -> API -> DB)

1. Client (browser) -> Cloudflare Pages (Nuxt SSR HTML + assets)
2. Any authenticated API call hits `/api/*` -> Cloudflare Function (proxy):
   - Reads `access_token` and `refresh_token` HttpOnly cookies
   - Injects `Authorization: Bearer <token>` header
   - Adds `x-school-slug` and/or `x-org-slug` from subdomain
   - Applies edge rate limiting
3. Rust API (Railway, Axum):
   - Auth middleware validates the JWT
   - RBAC middleware/extractor checks permissions
   - A `CurrentUser` extractor supplies `organization_id` and `school_id` to handlers
   - Service layer enforces ownership
   - `sqlx` queries are always scoped by tenant fields
4. Response flows back through proxy -> client (cookies updated transparently on refresh)

This architecture ensures the frontend never sees tokens and the backend remains decoupled.

---

## 5) Database - Technical Implementation (PostgreSQL + sqlx)

### Multi-Tenant Strategy Chosen

Row-level isolation with RLS. **Decided and implemented**, not aspirational:
see `migrations/20260716000006_enable_rls.sql` and the full rationale in
[CONTEXT.md](CONTEXT.md) "Locked architecture decisions", decision 5. An
earlier version of this section left RLS ambiguous while
[kyc-schema-api.md](../backend/kyc-schema-api.md) listed it as open; both
now say the same thing.

Why this over schema-per-tenant or separate DBs?

- Simpler operations
- Single connection pool
- Easier analytics across schools
- RLS adds defense-in-depth
- Row-level isolation stays correct at any tenant count; what changes with
  scale is physical layout, not the isolation model

Two mechanics that are part of the decision, not implementation detail:

- Policies read transaction-local settings, so **every request touching
  tenant data opens a transaction**. Pooled connections are reused, so the
  setting cannot be applied once at connect time.
- The application connects as a non-superuser role (`klassivox_api`,
  granted `klassivox_app`), and tenant tables use `FORCE ROW LEVEL
SECURITY`. A superuser, or the table owner without `FORCE`, bypasses
  every policy and would make the whole mechanism decorative.

**Promotion path when one database is no longer enough.** The earlier claim
that this "scales to thousands of schools on one DB" and can "migrate to
schema-per-tenant later" is wrong on the second half and understates the
first. Schema-per-tenant is the wrong destination at national scale: 20,000
schools means 20,000 schemas, and every migration becomes 20,000 DDL
statements. The order is:

1. Index and query tuning on one database. Sufficient well past the point
   most products ever reach.
2. Read replicas for reporting and analytics, which is where cross-school
   aggregate load lands first.
3. Partition the heavy tables (attendance, results, audit rows) by time,
   and by tenant where access patterns justify it.
4. Only then, database-per-shard with organizations pinned to a shard.
   Row-level tenancy is what makes this possible without touching
   application query logic, because the tenant predicate is already on
   every row.

None of steps 2 through 4 is built or needed now. They are recorded so the
first one is not chosen by accident.

### Audit Pattern

Every business table should include:

- `organization_id` (where applicable)
- `school_id` (where applicable)
- `created_at`
- `updated_at`
- `created_by`
- `updated_by`
- `deleted_at` (soft delete)

All queries are scoped by tenant IDs in services + RLS policy.

### Core Schema Excerpt (plain SQL, run via `sqlx migrate`)

`sqlx` has no schema-DSL layer like Drizzle - migrations are plain `.sql`
files under `migrations/`, applied with `sqlx migrate run`, and query results
are mapped onto Rust structs derived with `#[derive(sqlx::FromRow)]`.

**The real migrations under `apps/atlas-backend/migrations/` are the
source of truth for schema.** The excerpt that used to sit here was written
before they existed and contradicted them in two ways that would actively
mislead an implementer: it used `DEFAULT gen_random_uuid()` (v4, unordered)
where the real schema generates UUIDv7 app-side with no default, and it gave
`users` a `NOT NULL organization_id`, which contradicts the one-user-many-
roles model that `user_roles` exists to express. Read the migrations rather
than a sketch. The shape, abbreviated:

```sql
/* No DEFAULT on id. Postgres 16 has no uuidv7(); ids come from
 * Uuid::now_v7() in Rust so a missing id fails loudly instead of
 * silently becoming an unordered v4. */
CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    parent_organization_id UUID REFERENCES organizations(id), -- diocese, chain
    name TEXT NOT NULL,
    kind organization_kind NOT NULL,
    ...
    deleted_at TIMESTAMPTZ
);

CREATE TABLE schools (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    state_id UUID NOT NULL REFERENCES states(id),
    slug TEXT NOT NULL UNIQUE,      -- state-prefixed: lagos-unity-college
    custom_domain TEXT UNIQUE,      -- reserved, unread
    is_published BOOLEAN NOT NULL DEFAULT false,
    brand_color TEXT,               -- branding depth level 2
    public_template TEXT NOT NULL DEFAULT 'default', -- level 3, reserved
    timezone TEXT NOT NULL DEFAULT 'Africa/Lagos',
    locale TEXT NOT NULL DEFAULT 'en-NG',
    currency CHAR(3) NOT NULL DEFAULT 'NGN',
    ...
    deleted_at TIMESTAMPTZ
);

/* users is deliberately NOT tenant-scoped: one human, many role grants
 * across many organizations. Tenancy lives entirely in user_roles. */
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    ...
    deleted_at TIMESTAMPTZ
);
```

Corresponding Rust struct (for the `users` table, as an example):

```rust
#[derive(sqlx::FromRow)]
struct User {
    id: uuid::Uuid,
    organization_id: uuid::Uuid,
    school_id: Option<uuid::Uuid>,
    email: String,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}
```

All other tables follow the same tenant + audit pattern.

### Indexing Strategy

- Composite indexes on `(organization_id, school_id, ...)` for frequent queries
- Composite indexes on `(school_id, ...)` for school-centric access patterns
- GIN full-text on student names (or trigram indexes if needed)
- Phase 3 partitioning candidates: `attendance_records` and `grades` by `school_id + academic_year_id`

---

## 6) API Design (REST)

VITE_API_BASE_URL=https://schain-api-production.up.railway.app/api/v1

### Response Format

```json
{
  "success": true,
  "data": {},
  "meta": { "page": 1, "limit": 20, "total": 147 }
}
```

### Error Format (Axum error-handling middleware / `IntoResponse` impl)

```json
{
  "success": false,
  "error": {
    "code": "STUDENT_NOT_FOUND",
    "message": "...",
    "statusCode": 404,
    "timestamp": "..."
  }
}
```

### Example Endpoints

- `POST /auth/login` -> sets HttpOnly cookies
- `GET /students?classId=...` -> scoped by tenant
- `POST /payments` -> Paystack webhook at `/webhooks/paystack`

### Authentication Flow (Proxy + Cookies)

1. Login -> API sets two HttpOnly cookies (`access_token` 15 min JWT + `refresh_token` 7d opaque)
2. Client calls `/api/students` -> Proxy reads cookie -> injects Bearer -> forwards
3. On `401`, `atlas-web`'s Nitro proxy calls `/api/auth/refresh` with refresh cookie and retries once

---

## 7) RBAC Enforcement

For the role/permission _model_ (which roles exist, what they can do), see
the PRD's role model and permissions matrix. This section covers only how
it's enforced in code, in three layers (Axum):

1. JWT auth middleware (`tower` layer) for protected routes
2. A `RequireRole(...)` extractor (or route-level middleware) for route-level authorization
3. Service-level ownership guard for data-level checks (`organization_id`, `school_id`, and relationship assertions)

---

## 8) Scalability Considerations

- Database: read replicas for reporting (Phase 2+)
- Partition heavy tables by tenant + year
- Caching: Redis TTL for timetables, published report cards, and school settings
- Background jobs: BullMQ + Redis for bulk invoices, PDF generation, notifications
- Horizontal scaling: stateless Rust/Axum service + Railway autoscaling
- Rate limiting: Cloudflare edge + a `tower` rate-limit layer in Axum
- Observability: structured logs + OpenTelemetry (Phase 1 target)

---

## 9) Bootstrap / Scaffold Order

For the concrete, command-level "how do I add a new app/package" recipe,
see the "Scaffolding a new app or package" section of `README.md`. This section covers the order/rationale;
that doc covers the exact steps.

Day 1-2 (Monorepo bootstrap):

- Monorepo already bootstrapped (Nx + pnpm, root config in place).
- Add apps (see Section 2 for naming rationale):
  - `pnpm dlx nuxi@latest init apps/landing-page` (Nuxt 4, wired into Nx via `nx:run-commands` wrapping `nuxt dev`/`build`)
  - `pnpm dlx nuxi@latest init apps/atlas-web` (Nuxt 4, same wiring; owns the `server/api` proxy to `atlas-backend`)
  - `atlas-admin`: plain Vue + Vite SPA, intentionally lightweight (no Nx Vue generator needed for a single small app)
  - `atlas-backend`: not an Nx generator - `cargo new apps/atlas-backend`
    (Axum + `sqlx` + `tokio`), then add a `project.json` with `nx:run-commands`
    targets (`build` -> `cargo build --release`, `serve` -> `cargo run`) so
    `nx run-many` and `nx affected` still pick it up alongside the TS apps
  - No separate proxy app - the BFF/cookie-handling layer is `atlas-web`'s own `server/api` directory (Nitro), deployed as Cloudflare Pages Functions
  - `atlas-mobile` (Ionic + Capacitor): scaffold when mobile is prioritized, not part of initial bootstrap
- Generate shared packages/libs:
  - `packages/atlas-ui/core` (`@atlasui-core`, cross-platform Vue components)
  - `packages/atlas-ui/app` (`@atlasui-app`, Ionic-specific chrome)
  - `packages/atlas-ui/web` (`@atlasui-web`, desktop-specific chrome)
  - `libs/shared/theme` (`@atlastheme`)
  - `libs/api` (`@atlasapi`, typed client - mirrors Stayar's `stayar-api`)
- Set path aliases and dependency constraints in `nx.json`

Week 1 (cross-cutting foundations):

- Design system and theme package (burgundy palette + light/dark mode)
- Shared types + Zod schemas + React Hook Form integration (`@hookform/resolvers/zod`)
- Auth module in Rust/Axum (JWT, OTP, middleware)
- Cloudflare API proxy cookie flow
- `sqlx` setup with tenant-scoping query helpers
- RBAC middleware + a `CurrentUser` extractor

Week 2-4 (Phase 1 MVP):

- Auth -> User/Role -> Student CRUD -> Basic Academic -> Attendance -> Report Card
- Use `nx affected:build` and `nx affected:test`
- Implement one role end-to-end first (Admin)
- Add Storybook early

Ongoing:

- Feature branches + PR workflow
- Testing: unit + E2E (Playwright)
- Local dev with Docker Compose (Postgres + Redis)
- CI/CD with GitHub Actions + Nx affected + Cloudflare/Railway deploy
- ESLint + Prettier + dependency graph checks
- Storybook-driven UI contracts for reusable components and modal variants
- Component acceptance checklist: handles loading/error/empty states before merge

Phase progression:

- Finish Phase 1 and deploy internal beta for one school
- Then Phase 2 (payments, timetable, communication)
- Then Phase 3 (advanced reporting and scale)

Key tip: start with shared packages and auth. Once auth + proxy + DB scoping works end-to-end, adding modules becomes plug-and-play.
