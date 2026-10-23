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

## Inventory

Stock is tracked per store: every product has an independent balance at each location, and a sale only touches the store it happened at. The overall total is never stored; it is the sum of the per-store balances at read time.

Two tables carry this (`apps/atlas-backend/migrations`, `...24` and `...25`):

- `stock_movements` - append-only audit ledger. Every change to stock writes one signed row (`initial_stock`, `purchase_receipt`, `sale`, `adjustment`, `transfer_out`, `transfer_in`), with who and why.
- `stock_levels` - the current balance per (store, product[, batch/serial]), with `CHECK (quantity >= 0)`. It is only written in the same transaction as the ledger row that explains the change, and it is what row-locks concurrent sales.

All routes are under `/api/v1/stock`, need `Authorization: Bearer <token>`, and return `{ message, data, success }`. Quantities are decimals and serialize as strings.

| Route | Permission | What it does |
| --- | --- | --- |
| `POST /inventory` | `stock.adjust` at the store | Put a product in a store with initial stock. `409 INVENTORY_EXISTS` if it is already there. |
| `GET /inventory?store_id=&product_id=&low_stock=` | `stock.view` (filtered to your stores) | Every product with its quantity per store. |
| `GET /products/{product_id}` | `stock.view` (filtered to your stores) | One product across stores, with `total_quantity`. |
| `GET /on-hand?store_id=&product_id=&low_stock=&max_quantity=` | `stock.view` at the store | One store's balances. `low_stock` keeps products at or below the listing's `low_stock_threshold`. |
| `GET /movements?store_id=&product_id=&limit=` | `stock.view` (filtered to your stores) | The audit trail, newest first (default 50, max 200). |
| `POST /restocks` | `stock.adjust` at the store | Incoming supplier stock (`purchase_receipt`). |
| `POST /adjustments` | `stock.adjust` at the store | Signed correction. A reason is required; cannot go below zero. |
| `POST /transfers` | `stock.transfer` at both stores | Move stock. Debits the source, credits the destination, one transaction. |

Sales are `POST /api/v1/sales` (also used by `POST /api/v1/sync/sales`). The server resolves price from `price_rules` and converts the sale unit to the product's base unit; any price in the request is ignored. It returns the receipt (the sale plus its lines).

Errors use `{ message, success: false, code }`. The codes you will see:

- 409: `INSUFFICIENT_STOCK`, `INVENTORY_EXISTS`, `ALREADY_EXISTS` (duplicate slug, SKU, unit name or sale unit), `STORE_INACTIVE`, `PRODUCT_INACTIVE`.
- 400: `INVALID_QUANTITY`, `REASON_REQUIRED`, `SAME_STORE`, `NO_PRICE`, `INVALID_UNIT`, `VALIDATION_FAILED`, `INVALID_REFERENCE`, `INVALID_VALUE`.
- 404: a store, product, unit, shift or customer that does not exist in your tenant (another tenant's ids look nonexistent).

Edits are partial: absent fields are unchanged, an empty body is a 400, and unknown or immutable fields are rejected with 422 rather than ignored.

### Customers and customer groups

Customers belong to the tenant, not a store, so a cashier at one store can register and look up customers. A customer group is a pricing tier: a price rule can target a group, and customers in it are charged that price.

| Route | Permission | What it does |
| --- | --- | --- |
| `GET /customers?q=&customer_group_id=&limit=&offset=` | `sale.create` at any store | Search by name, phone or email (case-insensitive substring; `%` and `_` are literal). Paged, 50 by default, 200 at most. |
| `POST /customers` `{full_name, phone?, email?, customer_group_id?}` | `sale.create` at any store | Register a customer. Emails are stored lowercase. |
| `GET` / `PATCH` / `DELETE /customers/{id}` | `sale.create` at any store; delete needs `sale.refund` | `PATCH` is partial, `null` clears phone, email or group. Delete is soft, so past sales keep their customer. Store credit cannot be edited (422). |
| `GET /customer-groups` | any member | List groups. |
| `POST /customer-groups` / `PATCH` / `DELETE /customer-groups/{id}` | `product.manage` | Create, rename, delete. A group that still has customers or price rules cannot be deleted (`GROUP_IN_USE`). |

Putting a customer in a group (on create or edit) also needs `product.manage`, because it changes what they pay. A cashier can register a customer but cannot move anyone into a cheaper tier.

### Returns and voids

Both live under `/api/v1/sales/{id}` and only apply to a completed sale.

| Route | Permission | What it does |
| --- | --- | --- |
| `GET /sales/{id}` | any member | The receipt: the sale (with `status` and `voided_at`) plus its lines. |
| `POST /sales/{id}/returns` `{reason, lines: [{sale_line_id, quantity}], return_to_stock?}` | `sale.refund` at the sale's store | Return some or all of selected lines, in each line's own unit. Stock goes back and the refund is computed by the server. |
| `GET /sales/{id}/returns` | any member | The returns recorded against the sale. |
| `POST /sales/{id}/void` `{reason}` | `sale.void` at the sale's store | Cancel the whole sale. The sale keeps its lines; stock goes back; the full total is the refund. |

- **Refunds** are the line's share of the sale total, so a sale-level discount and tax are refunded proportionally. A line split across several returns always adds up to exactly its share.
- **Stock** goes back in the product's base unit, using what was actually deducted at sale time, so editing a unit's conversion later cannot change it. With `return_to_stock: false` (damaged goods) the refund is recorded but stock is not increased.
- **Limits.** You cannot return more than was sold and not yet returned (`RETURN_EXCEEDS_SALE`). A voided sale accepts nothing further (`SALE_VOIDED`). A sale that already has returns cannot be voided (`SALE_HAS_RETURNS`): return the rest instead. A return with any invalid line records nothing.
- **Audit.** The original sale ledger rows are never edited. Returns and voids add `sale_return` rows beside them, with the reason and who did it.
- Refunds are recorded as amounts; paying money back is outside this system.

Every error, including a malformed body, bad id in a path or query, unknown route or wrong method, is JSON in the same `{ message, success, code }` shape. Request bodies must be JSON objects with `Content-Type: application/json`. Codes for those: `INVALID_BODY` (400/415/422), `INVALID_PATH`, `INVALID_QUERY`, `METHOD_NOT_ALLOWED`.

Inactive stores and products cannot sell, be restocked or take part in transfers. Adjustments are still allowed on them, so a closed store's leftover stock can be written off.

### Setting up the catalogue

A product cannot be sold until it has a price. Everything below needs the `product.manage` permission (owner, admin and store manager have it) and is under `/api/v1`:

| Route | What it does |
| --- | --- |
| `POST /stores` `{name, slug}` | Open a branch. The slug is lowercase letters, digits and single hyphens, unique per tenant. |
| `GET /stores/{id}` / `PATCH /stores/{id}` `{name?, slug?, address?, is_active?}` | Read or edit a branch. Needs `store.update` at that store, so a store-scoped admin edits their own branch only. A store with an open shift cannot be deactivated (`SHIFT_OPEN`). Currency is not editable. |
| `GET /products/{id}` / `PATCH /products/{id}` `{name?, sku?, attributes?, is_active?}` | Read or edit a product. `sku: null` clears it. Product type and base unit cannot change (422). Set `is_active: false` instead of deleting: stock, ledger and sale history are kept. |
| `POST /units` `{name, abbreviation}` / `GET /units` | Units of measure (piece, kg, carton). |
| `POST /product-types` / `POST /products` | Define a product with its base unit. Stock is held in the base unit. |
| `POST /products/{id}/sale-units` `{unit_id, factor_to_base}` | Let a product be sold in another unit, e.g. a carton of 12 pieces is `12`. |
| `POST /price-rules` `{product_id, unit_price_amount_minor, store_id?, customer_group_id?, min_quantity?, priority?, starts_at?, ends_at?}` | Set a price per base unit, in minor currency units. |
| `GET /price-rules?product_id=` / `DELETE /price-rules/{id}` | List or remove rules. Past sales keep the price they sold at. |

When several rules match a sale, the most specific wins: a store rule beats an all-stores rule, a customer-group rule beats an all-customers rule, then the highest `min_quantity` tier reached, then `priority`, then the newest. Rules outside their `starts_at` and `ends_at` window are ignored.


```bash
# Put rice in a store with 50 bags
curl -X POST localhost:5755/api/v1/stock/inventory -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"store_id":"<location-one-id>","product_id":"<rice-id>","quantity":50}'

# Sell 10 at that store
curl -X POST localhost:5755/api/v1/sales -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"id":"<new-uuid-v7>","store_id":"<location-one-id>","shift_id":"<open-shift-id>","device_id":"till-1",
       "lines":[{"product_id":"<rice-id>","unit_id":"<piece-unit-id>","quantity":10}]}'

# Move 10 from Location One to Location Two
curl -X POST localhost:5755/api/v1/stock/transfers -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"from_store_id":"<location-one-id>","to_store_id":"<location-two-id>","product_id":"<rice-id>","quantity":10,"reason":"Rebalancing"}'

curl localhost:5755/api/v1/stock/products/<rice-id> -H "authorization: Bearer $TOKEN"
```

```json
{
  "message": "Product inventory fetched.",
  "success": true,
  "data": {
    "product_id": "...",
    "product_name": "Rice",
    "total_quantity": "90",
    "locations": [
      { "store_id": "...", "store_name": "Location One", "quantity": "30" },
      { "store_id": "...", "store_name": "Location Two", "quantity": "40" },
      { "store_id": "...", "store_name": "Location Three", "quantity": "20" }
    ]
  }
}
```

### Running the tests

The integration tests (`apps/atlas-backend/tests/inventory.rs` for stock and sales, `catalog.rs` and `edit.rs` for stores, products, units and pricing, `customers.rs` for customers and groups, `returns.rs` for returns and voids, `errors.rs` for the JSON error shape) drive the real router against a real Postgres, with real JWTs, permissions and row level security. They do not run without one:

```bash
# A throwaway database, plus the non-superuser login role RLS depends on.
# Skip the CREATE ROLE if atlas_api already exists (docker-compose.atlas.yml creates it).
psql -U atlas postgres -c "CREATE DATABASE atlas_test" -c "CREATE ROLE atlas_api LOGIN PASSWORD 'atlas_api_dev'" \
  -c "GRANT CONNECT ON DATABASE atlas_test TO atlas_api"

export ATLAS_TEST_DATABASE_ADMIN_URL=postgres://atlas:atlas@localhost:5434/atlas_test   # superuser: migrations + fixtures
export ATLAS_TEST_DATABASE_URL=postgres://atlas_api:atlas_api_dev@localhost:5434/atlas_test  # what the app connects as
cargo test --manifest-path apps/atlas-backend/Cargo.toml
```

Each test creates its own tenant, so tests share the database but not rows and run in parallel. Migrations run once per test process.

### Migrations: two things to know

- **Run `sqlx migrate` with `PGOPTIONS='-c search_path=public'`.** The `nx migrate`, `nx migrate:revert` and `nx dev` targets already do. Without it, when the Postgres superuser is named `atlas` (as in `docker-compose.atlas.yml`) it matches the `atlas` schema, so sqlx's bookkeeping table lands in a different schema on the second run and every migration replays (`relation "users" already exists`).
- **If your local database was migrated before this change, recreate it.** Migration `...06` was made safe to run on a second database in the same Postgres server (`atlas_app` is a cluster-wide role, so it is now created only if missing). That changed the file, so sqlx reports `migration 20260922000006 was previously applied but has been modified` for databases migrated earlier. Local development data can be thrown away: `docker compose -f docker-compose.atlas.yml down -v`, then `nx dev atlas-backend`. Nothing has been deployed, so no shared environment is affected.

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
