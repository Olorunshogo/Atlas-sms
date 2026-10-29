# Atlas SMS

Nx-powered monorepo for Atlas, a multi-store management platform: one system that runs many stores, each with its own inventory, prices, staff and sales, with shared catalogue, rules and reporting across every branch.

---

## Project Structure

```
.
├── apps/
│   ├── atlas-backend/          # Rust/Axum/sqlx API
│   ├── atlas-web/              # Nuxt - public site, auth, owner/manager dashboard
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

## Inventory and catalogue

Stock is tracked per store. Each product has its own balance at each branch, a sale or return only touches the store it happened at, and the overall total is summed from the branch balances at read time, never stored. Balances are not a column: on-hand is the sum of the append-only `stock_movements` ledger, and every change goes through one central inventory service that takes a per-(store, product) lock, so two concurrent sales or transfers cannot take the same units.

The full endpoint list, with permissions, request and response shapes and error codes, is [`docs/atlas/api-reference.md`](docs/atlas/api-reference.md). A test fails if a mounted route is missing from it. In short:

- **Catalogue setup** (needs `product.manage`): `/units`, `/product-types`, `/products` (create, read, edit, deactivate), `/products/{id}/sale-units`, `/products/{id}/price-rules`, `/customer-groups`, and `/stores` (create, read, edit, deactivate).
- **Stock**: `GET /stock/inventory` and `GET /stock/products/{id}` (every store's balance side by side, with a derived total, limited to the stores the caller may view), `GET /stock/movements` (the audit trail), `GET /stock/on-hand`, and `POST /stock/adjustments` and `/stock/transfers`.
- **Selling**: server-priced checkout with split payments, receipts, returns with approval, and refunds, under `/sales`, `/returns` and `/payments`.

```bash
# Rice at three branches, then see them side by side with a total
curl localhost:5754/api/v1/stock/products/<rice-id> -H "authorization: Bearer $TOKEN"
```

```json
{
  "message": "Product inventory fetched.",
  "success": true,
  "data": {
    "product_id": "...",
    "product_name": "Rice",
    "total_quantity": "43",
    "locations": [
      { "store_id": "...", "store_name": "Lagos", "quantity": "20" },
      { "store_id": "...", "store_name": "Abuja", "quantity": "15" },
      { "store_id": "...", "store_name": "Kaduna", "quantity": "8" }
    ]
  }
}
```

Every error, including a malformed body, a bad id in a path or query, an unknown route or a wrong method, is JSON in the same `{ message, success, code }` shape. Request bodies must be JSON objects sent as `Content-Type: application/json`.

Refund amounts are recorded and confirmed inside Atlas, but no payment gateway is integrated yet, so no money actually moves; see [`docs/atlas/README.md`](docs/atlas/README.md).

### Running the tests

The backend integration tests use `#[sqlx::test]`: each test gets its own freshly migrated database and drives the real router. They need a `DATABASE_URL` for a Postgres superuser that can create databases, plus the non-superuser login role that row level security depends on:

```bash
# Skip CREATE ROLE if atlas_api already exists (docker-compose.atlas.yml creates it).
psql -U atlas postgres -c "CREATE ROLE atlas_api LOGIN PASSWORD 'atlas_api_dev'"

export DATABASE_URL=postgres://atlas:atlas@localhost:5434/postgres
cargo test --manifest-path apps/atlas-backend/Cargo.toml
```

Because `#[sqlx::test]` connects as a superuser, which bypasses row level security, the tests cannot rely on RLS to keep tenants apart. Every ownership check in the code therefore carries an explicit `tenant_id` predicate as well, and the tests assert on it; `rls_tests.rs` separately checks the policies themselves as `atlas_api`.

### Migrations: two things to know

- **Run `sqlx migrate` with `PGOPTIONS='-c search_path=public'`.** The `nx migrate`, `nx migrate:revert` and `nx dev` targets and the root `db:migrate` scripts already do. Without it, when the Postgres superuser is named `atlas` (as in `docker-compose.atlas.yml`) it matches the `atlas` schema, so sqlx's bookkeeping table lands in a different schema on the second run and every migration replays (`relation "users" already exists`).
- **The very first test run against a brand new Postgres server can fail once** with `role "atlas_app" already exists`: the role is cluster-wide, and many test databases migrate in parallel before any of them has created it. Run the tests again and it passes.

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
