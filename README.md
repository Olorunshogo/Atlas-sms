# Atlas SMS

Nx-powered monorepo for Atlas, a multi-store management platform: one system that runs many stores, each with its own inventory, prices, staff and sales, with shared catalogue, rules and reporting across every branch.

---

## Project Structure

```
.
├── apps/
│   ├── atlas-backend/          # Rust/Axum/sqlx API
│   ├── atlas-web/        # Nuxt - public site, auth, owner/manager dashboard
│   └── atlas-pos/              # Ionic/Capacitor - till login and session home
│
├── packages/atlas-ui/
│   ├── core/                   # @atlas/ui-core
│   ├── web/                    # @atlas/ui-web
│   └── app/                    # @atlas/ui-app
│
├── libs/shared/
│   ├── theme/                  # @atlas/theme
│   ├── composables/            # @atlas/composables
│   └── validation/             # @atlas/validation
│
└── docs/
    ├── atlas/                  # architecture, auth flow, product notes
    └── v2.md                   # scope deferred past v1
```

`libs/shared/*` is shared by the web app and the POS. See `docs/atlas/CONTEXT.md`.

---

## Apps

New feature work targets the three apps in this repo:

- `atlas-backend` is the API.
- `atlas-web` is the public site and the owner/manager dashboard.
- `atlas-pos` is the point-of-sale client.

Tenant isolation is a `TenantTx` Axum extractor plus Postgres row-level security. Auth is JWT plus a rotating refresh token. The UI packages are Vue libraries built with `tsup` and Storybook.

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
# Postgres on host port 5434, then the API
docker compose -f docker-compose.atlas.yml up -d postgres
# apply migrations with ATLAS_DATABASE_ADMIN_URL, then:
nx dev atlas-backend

# Marketing site and dashboard (Nitro proxy to the API)
nx dev atlas-web

# POS
nx dev atlas-pos
```

Copy `.env.example` to `.env` first. The API reads `ATLAS_DATABASE_URL`,
`ATLAS_JWT_SECRET`, and `ATLAS_PORT`. `atlas-web` reads `ATLAS_API_BASE`.
Connect as `atlas_api`, never the superuser. If a Postgres volume was created
before that role existed, create `atlas_api` or recreate the volume. Init SQL
runs only on a new volume.

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

- [`docs/atlas/README.md`](docs/atlas/README.md) - product design: store types, the product model, units, pricing rules
- [`docs/atlas/CONTEXT.md`](docs/atlas/CONTEXT.md) - locked architecture decisions and current gaps
- [`docs/atlas/auth-flow.md`](docs/atlas/auth-flow.md) - auth API, proxy, marketing screens, POS session
- [`docs/SystemDesign.md`](docs/SystemDesign.md) - how the running system is put together
- [`docs/RolesAndPermissions.md`](docs/RolesAndPermissions.md) - roles, grants, and what they can do
- [`docs/v2.md`](docs/v2.md) - scope deliberately deferred past v1

---

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for branch naming, commit conventions, and PR rules - enforced by git hooks, not just documentation.

---

## License

MIT
