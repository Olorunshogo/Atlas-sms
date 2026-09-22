# Atlas SMS

Nx-powered monorepo for Atlas, a multi-store management platform: one system that runs many stores, each with its own inventory, prices, staff and sales, with shared catalogue, rules and reporting across every branch.

---

## Project Structure

```
.
├── apps/
│   ├── atlas-backend/          # Rust/Axum/sqlx API - tenants, stores, roles, auth
│   ├── atlas-marketing/        # Nuxt app - public pages + owner/manager dashboard
│   ├── atlas-pos/              # Ionic/Capacitor - offline-first point of sale
│   │
│   ├── atlas-backend/      # Existing Rust/Axum API, kept as a pattern
│   ├── atlas-web/          # reference during Atlas's build-out - see
│   ├── atlas-mobile/       # "Why atlas-* is still here" below
│   └── landing-page/
│
├── packages/
│   ├── atlas-ui/
│   │   ├── core/                # @atlas/ui-core - shared Vue primitives, Storybook
│   │   ├── web/                 # @atlas/ui-web - browser-only additions
│   │   └── app/                 # @atlas/ui-app - Ionic/Capacitor additions
│   └── atlas-ui/            # existing component packages (see below)
│
├── libs/shared/
│   ├── theme/                   # @atlastheme - Tailwind design tokens
│   ├── composables/             # @atlascomposables - framework-agnostic helpers
│   └── validation/               # @atlasvalidation
│
└── docs/
    ├── Atlas/                   # Product/investor/tech PRDs for the store platform
    ├── atlas/CONTEXT.md         # Atlas's own locked architecture decisions
    ├── v2.md                    # Atlas features deliberately deferred
    └── ...
```

The `libs/shared/*` packages keep the `@atlas*` npm scope even though they're consumed by Atlas apps too - they're genuinely framework/product-agnostic (design tokens, small utilities), and renaming an npm scope with no functional reason to is exactly the kind of disruptive change this repo avoids. See `docs/atlas/CONTEXT.md` for the full reasoning.

---

## Why `atlas-*` is still here

Atlas was scaffolded by reusing a proven, already-built Rust/Axum + Nuxt + Nx setup rather than starting from zero - the `atlas-*` apps and packages are that source. Concretely, Atlas's own code borrows their *patterns*, not their code: row-level tenant isolation via a `TenantTx`-style Axum extractor, JWT + refresh-token auth, the Nx/pnpm workspace shape, and the tsup/Storybook component-package layout.

Klassivox itself is a separate, real product (multi-tenant school management) with its own repository. The copy of it in this repo exists only as scaffolding reference and is retired incrementally, one piece at a time, as each part gets its own Atlas-specific equivalent (tests, auth, the RLS migrations, and so on) - not as a single wholesale rename. New feature work targets the `atlas-*` apps; treat anything under a `atlas-*` path as read-only reference unless you're specifically the one retiring it.

---

## Tech Stack

- **Backend:** Rust, Axum, `sqlx`, PostgreSQL - row-level security for tenant isolation, a dedicated Postgres server per product
- **Web:** Nuxt 4, Vue 3, Tailwind CSS v4, Pinia
- **POS / mobile:** Ionic Vue, Capacitor
- **Monorepo:** Nx, pnpm workspaces
- **Component libraries:** Vue packages built with `tsup`, documented in Storybook

---

## Getting Started

### Prerequisites

- Node.js 24+
- pnpm 10+
- Rust (stable) + [`sqlx-cli`](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
- Docker (for each product's isolated Postgres container)

### Installation

```bash
pnpm install
pnpm run prepare   # git hooks
```

### Running Atlas

```bash
# Backend - its own Postgres server, isolated from atlas-backend's
nx docker atlas-backend    # starts atlas-backend's own Postgres container
nx dev atlas-backend       # runs migrations, then the API

# Web dashboard
nx dev atlas-marketing

# POS (Ionic/Capacitor)
nx dev atlas-pos
```

Copy `.env.example` to `.env` first - the `ATLAS_*` variables are documented inline there, including why they're prefixed (this repo's one root `.env` is shared with `atlas-backend`, which uses unprefixed names).

---

## Development

```bash
pnpm test               # nx run-many -t test
pnpm lint / lint:fix
pnpm format / format:check

nx dev <project>
nx build <project>
nx affected -t test      # only what changed
nx graph                 # visualize the project graph
```

---

## Tenant Isolation and Roles

Every tenant-scoped `atlas-backend` handler takes a `TenantTx` instead of a raw database connection, so writing an unscoped query is structurally impossible - the extractor decodes the caller's JWT, sets a transaction-local Postgres setting, and every tenant table enforces row level security against it. Roles are dynamic: a tenant's `owner` can grant `admin` or any custom role built from a fixed permission catalogue, scoped to the whole tenant or to specific stores, and a granter can never hand out more permission than they hold themselves.

Full reasoning: [`docs/atlas/CONTEXT.md`](docs/atlas/CONTEXT.md).

---

## Documentation

- [`docs/Atlas/README.md`](docs/Atlas/README.md) - product design: store types, the product/attribute model, units of measure, pricing rules
- [`docs/atlas/CONTEXT.md`](docs/atlas/CONTEXT.md) - Atlas's locked architecture decisions
- [`docs/v2.md`](docs/v2.md) - scope deliberately deferred past v1

---

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for branch naming, commit conventions, and PR rules - enforced by git hooks, not just documentation.

---

## License

MIT
