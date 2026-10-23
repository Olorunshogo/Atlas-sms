# Atlas API Reference

Every HTTP endpoint `apps/atlas-backend` serves, with its permission,
request and response shape, and the error codes a client should handle.
The code is the source of truth: each handler's doc comment names its
route, and `src/lib.rs::api_router` is where the routers are mounted. If
this file and a handler disagree, the handler wins and this file is the
bug.

Auth mechanics (cookies vs tokens, refresh rotation, tenant choice) are
explained in [auth-flow.md](auth-flow.md); permissions and roles in
[../RolesAndPermissions.md](../RolesAndPermissions.md); the workflows
behind checkout and returns in
[SystemDesignDocument.md](SystemDesignDocument.md) Section 7.

---

## Contents

1. [Conventions](#1-conventions)
2. [Health](#2-health)
3. [Auth](#3-auth)
4. [Tenant, roles and staff](#4-tenant-roles-and-staff)
5. [Stores and store settings](#5-stores-and-store-settings)
6. [Catalogue administration](#6-catalogue-administration)
7. [Till catalogue: search, barcode, quote, snapshot](#7-till-catalogue)
8. [Stock](#8-stock)
9. [Purchasing](#9-purchasing)
10. [Shifts](#10-shifts)
11. [Sales and checkout](#11-sales-and-checkout)
12. [Receipts](#12-receipts)
13. [Payments and payment methods](#13-payments-and-payment-methods)
14. [Returns and refunds](#14-returns-and-refunds)
15. [Customers](#15-customers)
16. [Reports](#16-reports)
17. [Error codes](#17-error-codes)

---

## 1. Conventions

**Base path.** Everything except `/health` lives under `/api/v1`
(`API_PREFIX` in `src/lib.rs`).

**Authentication.** Unless an endpoint says otherwise, it needs
`Authorization: Bearer <access_token>` (a 15-minute JWT carrying the user
and tenant). The web app never holds the token itself: its Nitro proxy
reads the HttpOnly cookie and adds the header. The POS sends it directly.
Every authenticated request runs in one database transaction scoped to
the token's tenant by row-level security, so no endpoint can return
another tenant's rows.

**Headers.**

| Header          | Used by        | Meaning                                                                                              |
| --------------- | -------------- | ---------------------------------------------------------------------------------------------------- |
| `Authorization` | all but auth   | `Bearer <access_token>`                                                                              |
| `X-Client-Type` | auth endpoints | `web` (default; tokens arrive as cookies) or `app` (POS; tokens arrive in the JSON body)             |
| `x-store-id`    | customers      | The store the caller is working at. Customer endpoints check permissions against it, see Section 15. |

**Permissions.** "Permission" below names the `atlas.permission` the
caller must hold. "at the store" means at the store the request concerns
(from the body, the path, or the entity it touches); a tenant-wide or
platform grant always qualifies, a store-scoped grant only for its own
store. "any member" means any authenticated user of the tenant.

**Success envelope.** Every JSON success response is:

```json
{ "message": "Sale recorded.", "data": { ... }, "success": true }
```

Only `data` is shown in the examples below. Two endpoints can also answer
`text/plain` (receipts), and say so.

**Error envelope.** Every error is:

```json
{
  "message": "Payments of 4500 don't cover the 5000 due.",
  "success": false,
  "code": "PAYMENT_INSUFFICIENT"
}
```

`code` is present when the client can act on it; Section 17 lists them.
Failures the framework produces before any handler runs use the same
envelope: a malformed or wrong-shaped JSON body, a bad id in a path or
query string, a wrong content type, an unknown route, a wrong method.
Request bodies must be JSON objects sent as `Content-Type: application/json`.
Status codes: `400` invalid input or a rule the request breaks, `401` no
or bad token, `403` missing permission, `404` not found (or not visible to
this tenant), `409` conflicts with current state (stock, duplicates,
idempotency, wrong status), `429` rate limited (auth only), `500` server
error with a generic message (details are logged, never returned).

**Money** is always an integer in minor units (kobo for NGN) in fields
ending `_amount_minor` or `_minor`. Never a float. A store's currency is
`stores.currency`.

**Quantities** are decimals sent and returned as JSON strings (`"2"`,
`"0.5"`), because half a kilogram is a real quantity. Rates are basis
points as integers: `750` is 7.5%.

**IDs** are UUIDv7 strings. The server generates them, except where an
endpoint says the client does (sales, sale lines, payments and returns
from the POS), in which case the id is also the idempotency key.

**Timestamps** are RFC 3339 in UTC, e.g. `2026-10-22T14:03:00Z`.

**Pagination.** List endpoints that can grow return
`{ "items": [...], "next_cursor": "<uuid>" | null }`. Pass `cursor=<next_cursor>`
to get the next page; `limit` defaults to 50 (20 for catalogue search)
and is capped at 200 (100 for catalogue search). A `null` cursor means
there are no more pages.

**Idempotent writes.** `POST /sales`, `POST /sync/sales`,
`POST /sales/{id}/payments` and `POST /sales/{id}/returns` take a
client-generated `id`. Sending the same body again returns the original
result without doing anything twice. Sending a different body under the
same id is `409 IDEMPOTENCY_CONFLICT`.

---

## 2. Health

### `GET /health`

No auth, no envelope. Returns the plain text `ok`.

---

## 3. Auth

All of `/api/v1/auth/*` is rate limited per client IP (`429` when
exceeded). Full flow and cookie details: [auth-flow.md](auth-flow.md).

**Session response** (login, refresh, switch-tenant), as `data`:

```json
{
  "access_token": "eyJ...", // X-Client-Type: app only
  "refresh_token": "opaque...", // X-Client-Type: app only
  "tenant_id": "0192...",
  "user": {
    "id": "0192...",
    "full_name": "Ada Obi",
    "email": "ada@example.com"
  }
}
```

For `X-Client-Type: web` the two tokens are omitted from the body and set
as `access_token` / `refresh_token` HttpOnly cookies instead.

| Method & path                       | Auth           | Body                                                               | `data` on success                                                 |
| ----------------------------------- | -------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------- |
| `POST /auth/signup`                 | none           | `{ full_name, email, phone, password, tenant_name }`               | `{ email }`, status `201`; emails an OTP                          |
| `POST /auth/verify-email`           | none           | `{ email, code }`                                                  | `null`; activates the owner grant                                 |
| `POST /auth/login`                  | none           | `{ email, password, tenant_id? }`                                  | session                                                           |
| `POST /auth/refresh`                | refresh token  | `{ refresh_token }` (app) or empty with cookie (web)               | session (refresh token rotated)                                   |
| `POST /auth/logout`                 | refresh token  | same as refresh                                                    | `null`; revokes the refresh token                                 |
| `GET /auth/me`                      | access token   | -                                                                  | `{ user, tenant_id, is_platform_admin, tenants: [{ id, name }] }` |
| `POST /auth/switch-tenant`          | access token   | `{ tenant_id }`                                                    | session for the other tenant                                      |
| `POST /auth/forgot-password`        | none           | `{ email }`                                                        | `null`; always succeeds, emails an OTP if the account exists      |
| `POST /auth/reset-password`         | OTP            | `{ email, code, new_password }`                                    | `null`                                                            |
| `POST /auth/invites/{token}/accept` | invite token   | `{ full_name, phone, password }` (ignored for an existing account) | `null`                                                            |
| `GET /auth/platform/tenants`        | platform grant | -                                                                  | `[{ id, name }]`                                                  |
| `POST /auth/platform/invite`        | platform grant | `{ email }`                                                        | `null`                                                            |

Auth-specific errors:

- `400 EMAIL_ALREADY_REGISTERED` on signup.
- `400 EMAIL_NOT_VERIFIED` on login.
- `409 TENANT_REQUIRED` / `PLATFORM_TENANT_REQUIRED` on login when the user belongs to more than one tenant: the error body carries `data: { tenants: [{ id, name }] }`; retry with `tenant_id`.
- `409 ROLE_NOT_FOUND` when the account has no active role in any tenant; `403` when `tenant_id` names a tenant the user has no active role in.

---

## 4. Tenant, roles and staff

### `GET /tenants/me`

Any member. `data`: `{ id, name, status, legal_name, tax_identification_number, created_at }`.

### `PATCH /tenants/me`

Permission `tenant.manage`. Sets what receipts print as the business.

```json
{ "legal_name": "Ada Stores Ltd", "tax_identification_number": "12345678-0001" }
```

Both optional. Applies to receipts issued from now on; issued receipts
keep their snapshot. Returns the tenant.

### `GET /roles`

Any member. The system roles plus the tenant's own:
`[{ id, tenant_id, name, permissions: ["sale.create", ...], is_system }]`.

### `POST /roles/invite`

Permission `staff.invite` at the store (or tenant-wide when `store_id` is
null), and the caller must hold every permission the role carries.

```json
{
  "email": "cashier@example.com",
  "role_id": "0000...0004",
  "store_id": "0192..."
}
```

`data`: `{ "outcome": "granted_existing" }` when the email already has a
verified account (the grant is active immediately), or
`{ "outcome": "invited" }` when an invite email was sent. `403` when the
role holds a permission the inviter doesn't.

---

## 5. Stores and store settings

### `GET /stores`

Any member. `[{ id, tenant_id, name, slug, currency, address, is_active }]`.

### `POST /stores`

Permission `store.create`. Body `{ name, slug, address? }`. `name` is
required and up to 100 characters; `slug` is lowercase letters, digits and
single hyphens, up to 63, unique per tenant (`409 ALREADY_EXISTS`). Returns
the store.

### `GET /stores/{id}`

Any member. The store, as listed above. `404` for an unknown id or another
tenant's store.

### `PATCH /stores/{id}`

Permission `store.update` at that store, so a store-scoped admin can edit
their own branch and no other.

```json
{
  "name": "Lekki Branch",
  "slug": "lekki",
  "address": "12 Admiralty Way",
  "is_active": false
}
```

All optional; an empty body is `400`, `address: null` (or blank) clears the
address, and a field left out is unchanged. Unknown or fixed fields
(`currency`, `tenant_id`, `id`) are refused with `422` instead of being
ignored. Deactivating a store with an open shift is `409 SHIFT_OPEN`: close
the shift first. A deactivated store keeps its stock and history but cannot
sell or take part in transfers; adjustments still work so its leftover stock
can be written off. Audited with before and after values.

### `GET /stores/{id}/settings`

Permission `sale.create` or `store.update` at the store. `data`:

```json
{
  "id": "0192...",
  "currency": "NGN",
  "timezone": "Africa/Lagos",
  "prices_include_tax": true,
  "default_tax_rate_id": "0192...",
  "cash_rounding_increment_minor": 1,
  "max_cashier_discount_basis_points": 0,
  "allow_negative_stock": false,
  "allow_pay_later": false,
  "address": "12 Allen Avenue, Ikeja",
  "phone": "+2348031234567",
  "receipt_prefix": "IKJ",
  "receipt_header": null,
  "receipt_footer": "Thank you for shopping with us",
  "return_window_days": 7,
  "refund_approval_threshold_minor": null,
  "restockable_conditions": ["resellable"]
}
```

| Field                               | Effect                                                                                                       |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `prices_include_tax`                | Price rules already include tax (tax is carved out) or exclude it (tax is added on top).                     |
| `default_tax_rate_id`               | Rate for products with no rate of their own. `null` means no tax.                                            |
| `cash_rounding_increment_minor`     | `1` is no rounding; `500` rounds a cash total to the nearest N5. The difference is a separate rounding line. |
| `max_cashier_discount_basis_points` | Discount (share of gross) anyone may give; above it needs `sale.discount`.                                   |
| `allow_negative_stock`              | Lets a live sale take stock below zero instead of failing `PRODUCT_OUT_OF_STOCK`.                            |
| `allow_pay_later`                   | Lets a sale with a named customer complete before it's fully paid.                                           |
| `receipt_prefix`                    | Prefix of receipt and return numbers; defaults to the first four letters/digits of the slug, upper-cased.    |
| `return_window_days`                | Returns need approval after the end of this many local days after the sale.                                  |
| `refund_approval_threshold_minor`   | A return refund at or above this needs approval. `null` means no amount limit.                               |
| `restockable_conditions`            | Item conditions that go back into saleable stock on return.                                                  |

### `PATCH /stores/{id}/settings`

Permission `store.update` at the store. Send any subset of the fields
above except `id`, `currency` and `timezone`. Nullable fields
(`default_tax_rate_id`, `refund_approval_threshold_minor`) are cleared by
sending `null` explicitly; leaving a field out keeps it. Returns the
updated settings. Audited with before and after values.

---

## 6. Catalogue administration

All writes need `product.manage` (tenant-wide). Reads are any member,
except price rules (see below). Names are trimmed and length-checked
(`400 VALIDATION_FAILED`), and a unit, product type, product, store or
customer group named in a request must belong to the caller's tenant (`404`
otherwise); a duplicate SKU, unit name or group name is `409 ALREADY_EXISTS`.

### `GET /units` / `POST /units`

A unit of measure (piece, kg, carton). Create body `{ name, abbreviation }`,
up to 50 and 10 characters; the name is unique per tenant. Shape
`{ id, name, abbreviation }`. A product's base unit and its sale units are
rows here.

### `GET /product-types` / `POST /product-types`

Create body: `{ name, attribute_schema?: [], identity_mode?: "by_product" | "by_batch" | "by_serial" | "by_both" }`.
`identity_mode` decides whether a sale line must name a batch or serial
(`stock_item_id`). Returns `{ id, tenant_id, name, attribute_schema, identity_mode }`.

### `GET /products` / `POST /products`

Create body: `{ product_type_id, base_unit_id, name, sku?, attributes?: {} }`.
`attributes` must be an object, `sku` is unique per tenant, and
`identity_mode` on a product type must be one of the four values below.
Product shape:

```json
{
  "id": "0192...",
  "tenant_id": "0192...",
  "product_type_id": "0192...",
  "base_unit_id": "0192...",
  "name": "Rice 50kg",
  "sku": "RICE-50",
  "attributes": {},
  "is_active": true,
  "tax_rate_id": null,
  "is_returnable": true
}
```

### `GET /products/{id}`

Any member. The product shape above; `404` if unknown or another tenant's.

### `PATCH /products/{id}`

```json
{
  "name": "Premium Rice",
  "sku": "RICE-PREM",
  "attributes": { "grade": "A" },
  "is_active": true,
  "is_returnable": false,
  "tax_rate_id": "0192..."
}
```

All optional, but an empty body is `400`. `tax_rate_id: null` clears it back
to the store default and `sku: null` clears the SKU. The product type and
base unit cannot be changed. Setting `is_active: false` stops the product
being sold or transferred but keeps its stock, ledger and sale history.

### `PUT /products/{id}/listings/{store_id}`

Lists the product at a store (or updates the listing). Only listed,
active products can be sold there.

```json
{ "cost_amount_minor": 62000, "low_stock_threshold": "10", "is_active": true }
```

All optional. Returns `{ id, product_id, store_id, cost_amount_minor, low_stock_threshold, is_active }`.

### `GET /products/{id}/sale-units` / `POST /products/{id}/sale-units`

An extra unit the product sells in: `{ unit_id, factor_to_base: "50" }`
(a 50kg bag of a product whose base unit is the kilogram). The factor must
be positive, the base unit cannot be declared again, and declaring the same
unit twice is `409 ALREADY_EXISTS`. The list is `[{ id, product_id, unit_id, factor_to_base }]`.

### `GET /products/{id}/price-rules` / `DELETE /products/{id}/price-rules/{rule_id}`

Both need `product.manage`. The list is every rule on the product, oldest
first, in the shape `POST` returns. Deleting is safe for history: a sale
line keeps the price it sold at. A rule is only deletable through its own
product (`404` otherwise).

### `POST /products/{id}/price-rules`

```json
{
  "store_id": "0192...", // null = every store
  "customer_group_id": null, // null = every customer
  "min_quantity": "10", // in base units; default 1
  "unit_price_amount_minor": 1500, // per base unit
  "priority": 0,
  "starts_at": null,
  "ends_at": null
}
```

The price at the till is the most specific matching rule: store-specific
beats all-stores, a customer group beats everyone, then higher
`priority`, then the bigger quantity break, then the longest-running rule.
A product with no matching rule can't be sold without a price override.
`unit_price_amount_minor` may be `0` (a giveaway) but not negative,
`min_quantity` must be above zero, and `ends_at` must be after `starts_at`.

### `POST /products/{id}/barcodes`

`{ barcode, unit_id? }`; `unit_id` defaults to the base unit, and must be
one of the product's units, so a carton barcode adds a carton. Barcodes
are unique per tenant: `409 BARCODE_EXISTS`.

### `GET /customer-groups` / `POST /customer-groups` / `PATCH /customer-groups/{id}` / `DELETE /customer-groups/{id}`

A customer group is a pricing tier: a price rule can target one, and every
customer in it is charged that price. Reads are any member; the rest need
`product.manage`. Body `{ name }` (up to 100 characters, unique per tenant),
shape `{ id, name }`. `DELETE` is `409 GROUP_IN_USE` while any customer or
price rule still points at the group, because the price rules would
otherwise vanish silently and change what those customers pay.

### `GET /tax-rates` / `POST /tax-rates` / `PATCH /tax-rates/{id}`

Create body `{ name, rate_basis_points }` (0 to 10000). A rate's
percentage never changes after creation; to change VAT, create a new rate
and point products or the store default at it. `PATCH` takes
`{ name?, is_active? }`; a retired rate falls through to the next one.
Shape: `{ id, name, rate_basis_points, is_active }`.

---

## 7. Till catalogue

All need `sale.create` at the store. These are the reads a till makes
while building a cart; adding, removing and changing quantities happen in
the till's own cart, which calls the quote endpoint to show the total.

### `GET /stores/{id}/catalog?q=&limit=`

Active products listed at the store whose name or SKU contains `q`, or
whose barcode is exactly `q` (sorted first).

```json
[
  {
    "product_id": "0192...",
    "name": "Long Grain Rice",
    "sku": "RICE",
    "identity_mode": "by_product",
    "is_returnable": true,
    "base_unit_id": "0192...",
    "tax_rate_id": "0192...",
    "tax_rate_basis_points": 750,
    "on_hand": "1000",
    "units": [
      {
        "unit_id": "0192...",
        "name": "Kilogram",
        "abbreviation": "kg",
        "factor_to_base": "1",
        "unit_price_amount_minor": 1500,
        "barcodes": []
      },
      {
        "unit_id": "0192...",
        "name": "Bag 50kg",
        "abbreviation": "bag",
        "factor_to_base": "50",
        "unit_price_amount_minor": 75000,
        "barcodes": ["6151234500017"]
      }
    ]
  }
]
```

`unit_price_amount_minor` is the walk-in price for one of that unit right
now, or `null` when no rule prices it.

### `GET /stores/{id}/catalog/barcode/{code}`

`data`: `{ unit_id, item }`, where `item` is one catalogue entry as above
and `unit_id` is the unit the barcode represents. `404` when the barcode
is unknown or its product isn't sold at this store.

### `POST /stores/{id}/catalog/quote`

Prices a cart without saving anything.

```json
{
  "customer_id": null,
  "lines": [
    { "product_id": "0192...", "unit_id": "0192...", "quantity": "2" },
    {
      "product_id": "0192...",
      "unit_id": "0192...",
      "quantity": "1",
      "unit_price_amount_minor": 300,
      "price_override_reason": "Dented tin",
      "discount": { "amount_minor": 50, "reason": "Loyal customer" }
    }
  ],
  "cart_discount": { "percent_basis_points": 500, "reason": "Promo" }
}
```

Line fields: `id?` (client line id), `product_id`, `stock_item_id?`
(required for batch- or serial-tracked products; a serial is quantity 1),
`unit_id`, `quantity`, `unit_price_amount_minor?` (only to override),
`price_override_reason?`, `discount?`. A discount takes exactly one of
`amount_minor` or `percent_basis_points`, and needs a `reason`.

`data`:

```json
{
  "store_id": "0192...",
  "currency": "NGN",
  "prices_include_tax": true,
  "customer_id": null,
  "customer_group_id": null,
  "lines": [
    {
      "line_number": 1,
      "client_line_id": null,
      "product_id": "0192...",
      "product_name": "Radio",
      "sku": "RADIO",
      "is_returnable": true,
      "stock_item_id": null,
      "unit_id": "0192...",
      "unit_name": "Each",
      "quantity": "1",
      "base_quantity": "1",
      "list_unit_price_amount_minor": 107500,
      "unit_price_amount_minor": 107500,
      "price_rule_id": "0192...",
      "price_overridden": false,
      "price_override_reason": null,
      "gross_amount_minor": 107500,
      "line_discount_amount_minor": 0,
      "allocated_cart_discount_amount_minor": 0,
      "net_amount_minor": 107500,
      "tax_rate_id": "0192...",
      "tax_rate_basis_points": 750,
      "tax_amount_minor": 7500,
      "line_total_amount_minor": 107500,
      "on_hand": "10"
    }
  ],
  "subtotal_amount_minor": 107500,
  "discount_amount_minor": 0,
  "cart_discount_amount_minor": 0,
  "discount_reason": null,
  "tax_amount_minor": 7500,
  "total_amount_minor": 107500,
  "cash_total_amount_minor": 107500,
  "cash_rounding_increment_minor": 1,
  "price_overridden": false,
  "priced_at": "2026-10-22T14:03:00Z",
  "violations": []
}
```

How totals are computed (one definition, `pricing::engine`): per line,
gross is `round(price x quantity)`; the line discount comes off; the cart
discount is split across lines in proportion, by largest remainder, so
the parts sum exactly; tax is added on top (exclusive) or carved out
(inclusive); header totals are sums of the lines. Rounding is half away
from zero. `cash_total_amount_minor` is what a customer paying cash owes
after cash rounding.

`violations` lists policy problems without failing the quote, so the
cashier sees all of them at once:
`PRICE_OVERRIDE_FORBIDDEN`, `DISCOUNT_FORBIDDEN`, `PRODUCT_OUT_OF_STOCK`
(each `{ code, message, line_number }`). Structural problems fail the
request: `400 PRODUCT_NOT_SOLD_HERE`, `400 INVALID_UNIT`,
`400 STOCK_ITEM_REQUIRED`, `409 PRODUCT_NOT_PRICED`, `400 STORE_INACTIVE`.

### `GET /stores/{id}/catalog/snapshot?updated_since=`

What an offline till caches. `data`:
`{ store_id, currency, prices_include_tax, cash_rounding_increment_minor, generated_at, items: [catalogue entries], price_rules: [...], tax_rates: [...] }`.
With `updated_since`, only products whose product row, price rules or
barcodes changed after that time, and only price rules changed after it.
Save `generated_at` and send it as the next `updated_since`.

---

## 8. Stock

On-hand is never stored; it's the sum of the stock ledger. Every
movement goes through the central inventory service.

### `GET /stock/on-hand?store_id=`

Permission `stock.view` at the store. `[{ product_id, on_hand: "98" }]`,
in base units.

### `GET /stock/inventory?store_id=&product_id=&low_stock=`

Every product with its on-hand at each store the caller holds `stock.view`
at, so a store-scoped clerk sees their own branch and a tenant-wide grant
sees all of them. `store_id` narrows to one store (`403` if the caller
cannot view it), `product_id` to one product, and `low_stock=true` keeps
only locations at or below their listing's `low_stock_threshold` (and only
those count toward the total). `403` when the caller can view no store at
all. Products by name, locations by store creation order:

```json
[
  {
    "product_id": "0192...",
    "product_name": "Rice",
    "total_quantity": "43",
    "locations": [
      { "store_id": "0192...", "store_name": "Lagos", "quantity": "20" },
      { "store_id": "0192...", "store_name": "Abuja", "quantity": "15" },
      { "store_id": "0192...", "store_name": "Kaduna", "quantity": "8" }
    ]
  }
]
```

`total_quantity` is summed from the locations at read time; it is never
stored, so it cannot drift from the branch balances.

### `GET /stock/products/{product_id}`

One product, in the shape above, across the stores the caller may view.
`404` if the product is unknown; a known product nobody has stocked is
`total_quantity: "0"` with no locations.

### `GET /stock/movements?store_id=&product_id=&limit=`

The audit trail, newest first, limited to stores the caller may view
(`limit` defaults to 50 and is clamped to 1..200). Each row is
`{ id, store_id, product_id, stock_item_id, movement_type, quantity, reason, reference_id, created_by_user_id, created_at }`;
the two legs of a transfer share one `reference_id`.

### `POST /stock/adjustments`

Permission `stock.adjust` at the store. A count correction or write-off;
never blocked by the stock check.

```json
{
  "store_id": "0192...",
  "product_id": "0192...",
  "stock_item_id": null,
  "quantity": "-3",
  "reason": "Damaged in storage"
}
```

`quantity` is signed and non-zero, in base units. Returns the movement:
`{ id, store_id, product_id, stock_item_id, movement_type, quantity, reason }`.

### Inter-store transfers (FR-22)

Staged: `requested` -> `approved` -> `dispatched` -> `received`, with
`rejected` and `cancelled` as exits. Stock actually leaves the sending
store only at dispatch and arrives at the receiving store only at
receive - the gap between the two is a documented in-transit state, so a
transfer is never countable as available at both stores at once.

### `GET /stock/transfers?store_id=`

Permission `stock.view` at the store. Every transfer touching that store,
either direction, newest first.

### `POST /stock/transfers`

Permission `stock.transfer` at the sending store.

```json
{
  "from_store_id": "0192...",
  "to_store_id": "0192...",
  "product_id": "0192...",
  "stock_item_id": null,
  "quantity": "60",
  "reason": "Rebalance"
}
```

`409 PRODUCT_OUT_OF_STOCK` if the sending store doesn't have enough right
now - an honest early rejection, not a hold; the authoritative check is at
dispatch. `409 STORE_INACTIVE` or `PRODUCT_INACTIVE` if either store or
the product is deactivated, and `404` for a store or product that is not
in the caller's tenant (this applies to every stock write: sales, returns,
purchases and adjustments as well).

### `POST /stock/transfers/{id}/approve`

Permission `stock.transfer` at both stores - moving stock into a branch is
as much a receiving-store decision as a sending-store one. `409
TRANSFER_NOT_REQUESTED` outside the `requested` state.

### `POST /stock/transfers/{id}/reject`

Permission `stock.transfer` at the receiving store. `409
TRANSFER_NOT_REQUESTED` outside the `requested` state.

### `POST /stock/transfers/{id}/cancel`

Permission `stock.transfer` at the sending store. Allowed from
`requested` or `approved` only - once dispatched, the stock has already
left, so there is nothing left to simply cancel. `409
TRANSFER_NOT_CANCELLABLE` otherwise.

### `POST /stock/transfers/{id}/dispatch`

Permission `stock.transfer` at the sending store. This is where stock
actually leaves: posts the `transfer_out` movement, under the same
atomic-claim-then-post pattern as a sale, so two concurrent dispatches
can't both take the same units. `409 TRANSFER_NOT_APPROVED` outside the
`approved` state; `409 PRODUCT_OUT_OF_STOCK` if the posting itself would
go negative.

### `POST /stock/transfers/{id}/receive`

Permission `stock.transfer` at the receiving store. Posts the
`transfer_in` movement. `409 TRANSFER_NOT_DISPATCHED` outside the
`dispatched` state.

---

## 9. Purchasing

### `GET /suppliers` / `POST /suppliers` / `PATCH /suppliers/{id}`

List is any member; writes need `supplier.manage`. Body
`{ name, phone?, email? }` (all optional on `PATCH`). Shape:
`{ id, name, phone, email, balance_amount_minor }`; the balance moves only
through goods receipts.

### `GET /purchase-orders?store_id=`

Permission `purchase.create` at the store.
`[{ id, store_id, supplier_id, status }]`, newest first. Status is
`draft`, `ordered`, `partially_received`, `received` or `cancelled`.

### `POST /purchase-orders`

Permission `purchase.create` at the store.

```json
{
  "store_id": "0192...",
  "supplier_id": "0192...",
  "lines": [
    {
      "product_id": "0192...",
      "quantity_ordered": "10",
      "unit_cost_amount_minor": 500
    }
  ]
}
```

### `PATCH /purchase-orders/{id}` (FR-21)

Permission `purchase.create` at the order's store. `draft` only - edit
before approval.

```json
{
  "supplier_id": "0192...",
  "lines": [
    {
      "product_id": "0192...",
      "quantity_ordered": "20",
      "unit_cost_amount_minor": 450
    }
  ]
}
```

Both fields optional; `lines`, when given, replaces the order's lines
wholesale. `409 PURCHASE_ORDER_NOT_DRAFT` outside `draft`.

### `POST /purchase-orders/{id}/submit` (FR-21)

Permission `purchase.create` at the order's store. `draft` -> `ordered` -
this schema's granularity merges submit and approve into one step rather
than a separate pending-approval state. `409 PURCHASE_ORDER_NOT_DRAFT`
outside `draft`.

### `POST /purchase-orders/{id}/cancel` (FR-21)

Permission `purchase.create` at the order's store. `draft` or `ordered`
only - not once anything has been received. `409
PURCHASE_ORDER_NOT_CANCELLABLE` otherwise.

### `GET /purchase-orders/{id}/lines`

Permission `purchase.create` at the order's store.
`[{ id, purchase_order_id, product_id, quantity_ordered, quantity_received, unit_cost_amount_minor }]`.

### `POST /purchase-orders/{id}/receive`

Permission `purchase.create` at the order's store. Records a goods
received note: stock comes in, the supplier's balance goes up, the
order's status is recomputed.

```json
{ "lines": [{ "purchase_order_line_id": "0192...", "quantity_received": "6" }] }
```

`data`: `{ id, purchase_order }`. `400` when a line would receive more
than is left on it.

### `POST /purchase-returns` (FR-23)

Permission `purchase.create` at the order's store. References a goods
received note line, not the order line directly - that is what tracks
how much was actually received and so is eligible to go back.

```json
{
  "purchase_order_id": "0192...",
  "reason": "Damaged in transit",
  "lines": [{ "goods_received_note_line_id": "0192...", "quantity": "3" }]
}
```

Posts a `purchase_return` stock movement per line (reducing stock) and
reduces the supplier's balance by the line's original received cost.
`409 RETURN_QUANTITY_EXCEEDED` if a line would return more than that GRN
line has received minus what it has already returned.

---

## 9b. Expenses (FR-24)

### `GET /expense-categories` / `POST /expense-categories`

List is any member; create needs `expense.manage`. Body `{ name }`.

### `GET /expenses?store_id=&category_id=&status=&created_by_user_id=&from=&to=`

Requires `expense.manage` at `store_id` when given. All filters optional.

### `POST /expenses`

Requires `expense.manage` at the store.

```json
{
  "store_id": "0192...",
  "category_id": "0192...",
  "amount_minor": 450000,
  "expense_date": "2026-10-23",
  "description": "Electricity",
  "reference": null
}
```

Starts `draft`, `payment_status` `unpaid`.

### `PATCH /expenses/{id}`

`draft` only. All fields optional. `409 EXPENSE_NOT_DRAFT` otherwise.

### `POST /expenses/{id}/submit`

`draft` -> `submitted`. `409 EXPENSE_NOT_DRAFT` otherwise.

### `POST /expenses/{id}/approve` / `POST /expenses/{id}/reject`

`submitted` -> `approved`/`rejected`. `409 EXPENSE_NOT_SUBMITTED` otherwise.

### `POST /expenses/{id}/pay`

`approved` only, sets `payment_status` to `paid`. `409 EXPENSE_NOT_APPROVED` otherwise.

---

## 10. Shifts

A sale belongs to a shift; a store has at most one open shift.

### `POST /shifts`

Permission `sale.create` at the store.
Body `{ store_id, opening_cash_amount_minor? }`. Returns the shift:
`{ id, store_id, opened_by_user_id, opened_at, opening_cash_amount_minor, closed_by_user_id, closed_at, closing_cash_amount_minor, status }`.
`409 SHIFT_ALREADY_OPEN`.

### `POST /shifts/{id}/close`

Permission `sale.create` at the shift's store.
Body `{ closing_cash_amount_minor }` (what was counted). `data` is the
closed shift plus its drawer count:

```json
{
  "id": "0192...",
  "status": "closed",
  "...": "...",
  "cash": {
    "opening_cash_minor": 0,
    "cash_collected_minor": 5000,
    "cash_refunded_minor": 0,
    "expected_cash_minor": 5000,
    "counted_cash_minor": 4500,
    "variance_minor": -500,
    "by_method": [{ "...": "see Section 16" }]
  }
}
```

Cash collected is the amount applied to sales, change already excluded.
A negative variance is a shortage.

---

## 11. Sales and checkout

### Sale shape

Returned by checkout, `GET /sales/{id}` and lookups:

```json
{
  "id": "0192...",
  "store_id": "0192...",
  "shift_id": "0192...",
  "customer_id": null,
  "cashier_user_id": "0192...",
  "device_id": "till-1",
  "status": "completed",
  "receipt_number": "IKJ-00000042",
  "currency": "NGN",
  "subtotal_amount_minor": 150000,
  "discount_amount_minor": 0,
  "cart_discount_amount_minor": 0,
  "discount_reason": null,
  "tax_amount_minor": 0,
  "total_amount_minor": 150000,
  "rounding_adjustment_minor": 0,
  "amount_paid_minor": 150000,
  "amount_refunded_minor": 0,
  "payment_status": "paid",
  "prices_include_tax": true,
  "price_overridden": false,
  "client_created_at": null,
  "created_at": "2026-10-22T14:03:00Z",
  "completed_at": "2026-10-22T14:03:00Z",
  "voided_at": null,
  "voided_by_user_id": null,
  "lines": ["...sale lines..."],
  "payments": ["...payments, Section 13..."],
  "refunds": ["...refunds, Section 14..."]
}
```

- `status`: `awaiting_payment` (recorded, waiting on a pending payment; stock not yet taken), `completed`, `voided`.
- `payment_status`: `unpaid`, `partially_paid` or `paid`, against `total_amount_minor + rounding_adjustment_minor`. Refunds don't change it; they show in `amount_refunded_minor`.
- `receipt_number` is set when the sale completes.

A sale line is the priced line from the quote, frozen: `id, sale_id,
line_number, product_id, product_name, sku, stock_item_id, unit_id,
unit_name, quantity, base_quantity, list_unit_price_amount_minor,
unit_price_amount_minor, price_rule_id, price_overridden,
price_override_reason, line_discount_amount_minor,
allocated_cart_discount_amount_minor, discount_reason, net_amount_minor,
tax_rate_id, tax_rate_basis_points, tax_amount_minor,
line_total_amount_minor`. History never re-reads the live catalogue.

### `POST /sales`

Permission `sale.create` at the store. Checkout: prices the cart on the
server, checks the payments, records everything and takes the stock, in
one transaction.

```json
{
  "id": "0192f6a0-...", // client-generated UUIDv7, the idempotency key
  "store_id": "0192...",
  "shift_id": "0192...",
  "customer_id": null, // null = walk-in
  "device_id": "till-1",
  "client_created_at": "2026-10-22T14:02:58Z",
  "lines": [{ "product_id": "0192...", "unit_id": "0192...", "quantity": "2" }],
  "cart_discount": null,
  "payments": [
    {
      "id": "0192...",
      "payment_method_id": "0192...",
      "amount_minor": 100000,
      "tendered_amount_minor": 120000
    },
    {
      "id": "0192...",
      "payment_method_id": "0192...",
      "amount_minor": 50000,
      "reference": "TRF-55501234"
    }
  ],
  "expected_total_amount_minor": 150000
}
```

- `lines` are the same as a quote's. Prices, discounts and tax are never taken from the client.
- `expected_total_amount_minor` is what the cashier saw; if the server's total differs the sale is refused with `409 TOTAL_MISMATCH` so the till can show the new total first.
- Each payment: `id?` (client id), `payment_method_id`, `amount_minor` (applied to the sale, > 0), `tendered_amount_minor?` (cash only; change is tendered minus amount), `reference?` (required by methods that say so), `status?`: `succeeded` (default) or `pending` (e.g. a card terminal still processing; never for cash).

The payments decide the outcome:

| Payments                                           | Result                                                      |
| -------------------------------------------------- | ----------------------------------------------------------- |
| Successful payments equal the amount due           | `completed`, `paid`, receipt issued, stock taken            |
| Successful + pending equal the amount due          | `awaiting_payment`; completes when the pending one succeeds |
| Short, store allows pay-later, sale has a customer | `completed`, `partially_paid` or `unpaid`                   |
| Short otherwise                                    | `400 PAYMENT_INSUFFICIENT`, nothing written                 |
| More than due                                      | `400 PAYMENT_OVERPAID`                                      |

The amount due includes cash rounding when any payment is cash.

Errors: everything the quote can fail with, plus `403 PRICE_OVERRIDE_FORBIDDEN`,
`403 DISCOUNT_FORBIDDEN`, `409 PRODUCT_OUT_OF_STOCK` (checked under a
lock), `409 SHIFT_NOT_OPEN` (closed shift) or `400 SHIFT_NOT_OPEN`
(another store's shift), `409 TOTAL_MISMATCH`, `400 PAYMENT_INSUFFICIENT`,
`400 PAYMENT_OVERPAID`, `400 PAYMENT_REFERENCE_REQUIRED`,
`400 PAYMENT_METHOD_INVALID`, `409 DUPLICATE_PAYMENT_REFERENCE`,
`409 IDEMPOTENCY_CONFLICT`.

### `POST /sync/sales`

Permission `sale.create` at the store. Same body and response as
`POST /sales`; this is what the POS sends for each sale it rang up
offline. Because the sale has already happened:

- a shift that has since closed is accepted;
- prices are resolved as of `client_created_at`;
- a device price that differs from the rules is kept and recorded as an audit entry, not refused;
- short stock is posted anyway and recorded as a stock exception for a manager to review.

Payment rules and idempotency are the same as the live path.

### `GET /sales`

Query: `store_id, shift_id, from, to, cashier_user_id, customer_id,
status, payment_status, q, cursor, limit`. All optional. `from`/`to`
filter on when the sale was rung up (`from` inclusive, `to` exclusive).
`q` matches an exact receipt number or the start of a transaction id.

With `sale.view`, searches every store that permission reaches (`403` if
`store_id` names one it doesn't). Without it, a caller only sees their own
sales in shifts that are still open.

`data`: `{ items, next_cursor }`, newest first; each item is a summary:
`{ id, receipt_number, store_id, store_name, shift_id, cashier_user_id,
cashier_name, customer_id, customer_name, status, payment_status,
currency, total_amount_minor, amount_paid_minor, amount_refunded_minor,
created_at, completed_at }`.

### `GET /sales/{id}`

`sale.view` at the sale's store, or the caller's own sale in a shift
that's still open. Returns the full sale with lines, payments and refunds.

### `GET /sales/{id}/lines`

Same access as above. Just the lines.

### `GET /sales/by-receipt/{receipt_number}`

`sale.view` or `sale.return` at the sale's store. The full sale; how a
return starts from the receipt a customer brings back.

### `POST /sales/{id}/void`

Permission `sale.void` at the store. Body `{ reason }`. Cancels a sale
the same day: stock goes back, live payments are reversed (pending ones
failed), status becomes `voided`. Only while the sale's shift is open and
before anything has been returned or refunded; otherwise
`409 SALE_NOT_VOIDABLE` (use a return instead).

### `POST /sales/{id}/payments`

Permission `sale.create` at the store. Pays down a sale that is awaiting
payment, or a pay-later balance. Body is one payment as in checkout.
Returns the payment. `400 PAYMENT_OVERPAID` beyond what is owed (pending
payments count as owed-for). Once covered, an `awaiting_payment` sale
completes and its stock is taken. A later cash payment lands in whatever
shift is open at the store now.

---

## 12. Receipts

A receipt is numbered and saved as a snapshot when its sale completes.
Every view and reprint renders that snapshot, so a receipt reproduces
the original values even after prices, names or store details change.
Numbers are `{prefix}-{8 digits}`, sequential per store with no gaps.

### `GET /sales/{id}/receipt?format=json|text&width=32|48`

Same access as reading the sale. Viewing doesn't count as printing.

JSON (`format` omitted or `json`), `data`:

```json
{
  "sale_id": "0192...",
  "receipt_number": "IKJ-00000042",
  "issued_at": "...",
  "snapshot": {
    "version": 1,
    "business": {
      "name": "Ada Stores",
      "legal_name": "Ada Stores Ltd",
      "tax_identification_number": "..."
    },
    "store": {
      "id": "...",
      "name": "...",
      "address": "...",
      "phone": "...",
      "receipt_header": null,
      "receipt_footer": "...",
      "currency": "NGN",
      "timezone": "Africa/Lagos"
    },
    "transaction_id": "0192...",
    "short_transaction_id": "9F3A12C4",
    "receipt_number": "IKJ-00000042",
    "issued_at": "...",
    "local_time": "2026-10-22 15:03",
    "cashier_name": "Bola",
    "customer_name": "Walk-in customer",
    "device_id": "till-1",
    "prices_include_tax": true,
    "lines": [
      {
        "line_number": 1,
        "name": "Pressing Iron",
        "sku": "IRON",
        "quantity": "1",
        "unit_name": "Each",
        "unit_price_amount_minor": 21500,
        "gross_amount_minor": 21500,
        "discount_amount_minor": 0,
        "tax_rate_basis_points": 750,
        "tax_amount_minor": 1500,
        "line_total_amount_minor": 21500
      }
    ],
    "subtotal_amount_minor": 23650,
    "discount_amount_minor": 0,
    "tax_amount_minor": 1650,
    "tax_breakdown": [
      {
        "rate_basis_points": 750,
        "taxable_amount_minor": 23650,
        "tax_amount_minor": 1650
      }
    ],
    "rounding_adjustment_minor": 0,
    "total_amount_minor": 23650,
    "amount_due_minor": 23650,
    "payments": [
      {
        "method_name": "Bank transfer",
        "kind": "transfer",
        "amount_minor": 13650,
        "tendered_amount_minor": null,
        "change_amount_minor": 0,
        "reference": "****1234",
        "status": "succeeded"
      }
    ],
    "amount_paid_minor": 23650,
    "balance_minor": 0
  },
  "current": {
    "status": "completed",
    "payment_status": "paid",
    "amount_paid_minor": 23650,
    "amount_refunded_minor": 0,
    "balance_minor": 0
  },
  "print_count": 1
}
```

`snapshot` never changes. `current` is where the sale stands now (later
pay-later payments, refunds). Payment references show their last four
characters only. `404` while the sale has no receipt (not yet
completed).

Text (`format=text`): `text/plain` fixed-width layout for a 58mm (`width=32`,
default) or 80mm (`width=48`) thermal printer, ready to send as-is.
Marked as a reprint once the receipt has been printed.

### `POST /sales/{id}/receipt/prints`

Records a print and returns what to print. The first print needs only
the right to read the sale; every later one is a reprint and needs
`receipt.reprint` at the store.

```json
{ "device_id": "till-1", "reason": "Customer lost it", "width": 32 }
```

All optional. `data`:
`{ kind: "original" | "reprint", copy_number, text, receipt }`, where
`receipt` is the JSON view above and `text` carries
`*** REPRINT (copy N) ***` at the top and bottom of every reprint.
Reprints are audited.

---

## 13. Payments and payment methods

### Payment shape

```json
{
  "id": "0192...",
  "sale_id": "0192...",
  "payment_method_id": "0192...",
  "method": "cash",
  "method_name": "Cash",
  "amount_minor": 5000,
  "tendered_amount_minor": 10000,
  "change_amount_minor": 5000,
  "refunded_amount_minor": 0,
  "currency": "NGN",
  "status": "succeeded",
  "reference": null,
  "status_reason": null,
  "created_at": "...",
  "processed_at": "..."
}
```

`method` is the built-in kind (`cash`, `card`, `transfer`, `wallet`,
`store_credit`, `other`); `method_name` is the tenant's label. Statuses:
`pending`, `succeeded`, `failed`, `reversed`, `partially_refunded`,
`refunded`. Moves allowed: `pending -> succeeded | failed`,
`succeeded -> reversed`; refund states are set only by refunds.

### `GET /payments?sale_id=`

Permission `sale.create` at the sale's store. The sale's payments, oldest
first.

### `POST /payments/{id}/status`

Permission `payment.manage` at the store. Confirms or fails a pending
payment: `{ "status": "succeeded" | "failed", "reference": "AUTH-7781", "reason": "Declined" }`
(`reference` and `reason` optional). Confirming the payment a sale was
waiting on completes the sale and takes its stock. Returns the payment.
`409 INVALID_PAYMENT_TRANSITION` from any other status.

### `POST /payments/{id}/reverse`

Permission `payment.manage` at the store. Body `{ reason }`. Takes a
successful payment off the sale (a bounced transfer, the wrong
terminal); the sale's paid amount and status drop with it. `409
INVALID_PAYMENT_TRANSITION` if any of it has been refunded.

### `POST /payments/{id}/refund`

Permission `sale.refund` at the store. First half of a step-up refund
that isn't tied to returned goods (a goodwill price adjustment); goods
coming back go through returns. Body `{ amount_minor, reason }`.

`data`: `{ refund, confirmation_token }`. The token is shown once and
expires in 15 minutes. The amount is reserved immediately against both
the payment and the sale. `409 REFUND_AMOUNT_EXCEEDED` beyond what's left
on either.

### `POST /payments/refunds/{id}/confirm`

Permission `sale.refund` at the store. Body `{ confirmation_token }`.
Executes the refund: the payment's refunded amount and status and the
sale's refunded total move. `400` for a wrong, expired or already-used
token.

### `GET /payment-methods`

Any member. `[{ id, code, name, kind, requires_reference, is_active, sort_order }]`.
Every tenant starts with `cash`, `card`, `transfer` (reference required)
and `store_credit`.

### `POST /payment-methods` / `PATCH /payment-methods/{id}`

Permission `store.update` tenant-wide. Create:
`{ code, name, kind, requires_reference?, sort_order? }`, e.g. a second
card terminal reconciled separately:
`{ "code": "moniepoint", "name": "Moniepoint POS", "kind": "card" }`.
`409 PAYMENT_METHOD_EXISTS` for a duplicate code. `PATCH` takes
`{ name?, requires_reference?, is_active?, sort_order? }`; a method's
`code` and `kind` don't change.

---

## 14. Returns and refunds

### Return shape

```json
{
  "id": "0192...",
  "store_id": "0192...",
  "sale_id": "0192...",
  "return_number": "R-IKJ-00000003",
  "status": "completed",
  "reason_code": "faulty",
  "notes": null,
  "refund_method": "original",
  "refund_payment_method_id": null,
  "outside_window": false,
  "refund_total_minor": 100000,
  "requested_by_user_id": "0192...",
  "approved_by_user_id": null,
  "approved_at": null,
  "rejected_by_user_id": null,
  "rejection_reason": null,
  "completed_at": "...",
  "device_id": "till-1",
  "created_at": "...",
  "lines": [
    {
      "id": "...",
      "return_id": "...",
      "sale_line_id": "...",
      "product_id": "...",
      "product_name": "Kettle",
      "stock_item_id": null,
      "unit_id": "...",
      "unit_name": "Each",
      "quantity": "1",
      "base_quantity": "1",
      "condition": "resellable",
      "restock": true,
      "refund_amount_minor": 100000,
      "tax_refund_minor": 0,
      "stock_movement_id": "..."
    }
  ],
  "refunds": ["...refunds, below..."]
}
```

`status`: `pending_approval`, `completed` or `rejected`. `return_number`
is set when it completes.

A refund: `{ id, sale_id, return_id, payment_id, payment_method_id,
method_name, method_kind, amount_minor, status, reason, reference,
requested_by_user_id, approved_by_user_id, approved_at, executed_at,
created_at }`. `payment_id` is null for refunds to store credit or out
through another method.

### `POST /sales/{id}/returns`

Permission `sale.return` at the sale's store.

```json
{
  "id": "0192...", // client-generated, the idempotency key
  "device_id": "till-1",
  "reason_code": "faulty",
  "notes": "Does not heat",
  "lines": [
    { "sale_line_id": "0192...", "quantity": "1", "condition": "resellable" }
  ],
  "refund": { "method": "original" }
}
```

- `condition`: `resellable`, `opened`, `damaged`, `defective` or `expired`. Only conditions in the store's `restockable_conditions` go back into stock; the rest are recorded but never re-enter saleable stock.
- `refund.method`:
  - `original`: back through the sale's payments, card/transfer first and cash last, each up to what's left on it.
  - `store_credit`: onto the sale's customer's balance (`400 CUSTOMER_REQUIRED` for a walk-in sale).
  - `payment_method`: out through `refund.payment_method_id`, with `refund.reference` if that method requires one.
- Each line's refund is its share of what the customer actually paid for it (after discounts, including tax); the last unit returned takes whatever is left, so repeated partial returns add up exactly.

The return completes immediately unless it needs approval, in which case
it is saved as `pending_approval` with its quantities held. It needs
approval when any of these holds:

- it's after the end of the `return_window_days`-th local day after the sale (`outside_window: true`);
- its refund is at or above `refund_approval_threshold_minor`;
- the caller doesn't hold `sale.refund`.

Errors: `409 SALE_NOT_COMPLETED`, `409 RETURN_QUANTITY_EXCEEDED` (the
message says how many are left), `400 ITEM_NOT_RETURNABLE`,
`409 REFUND_AMOUNT_EXCEEDED` (more than was paid and not yet refunded,
e.g. an unsettled pay-later sale), `400 CUSTOMER_REQUIRED`,
`400 PAYMENT_REFERENCE_REQUIRED`, `409 IDEMPOTENCY_CONFLICT`.

### `GET /sales/{id}/returns`

`sale.view` or `sale.return` at the store. The sale's returns (headers
only), oldest first.

### `GET /returns?store_id=&status=&from=&to=&cursor=&limit=`

Covers every store where the caller holds `sale.view` or `sale.return`.
`{ items, next_cursor }`, newest first, headers only.

### `GET /returns/{id}`

`sale.view` or `sale.return` at the store. The return with lines and
refunds.

### `POST /returns/{id}/approve`

Permission `sale.refund` at the store, and a different person from the
one who started the return (`403 APPROVAL_SELF_FORBIDDEN`). No body.
Completes the return: number, restock, refund. `409 RETURN_NOT_PENDING`
if it isn't waiting; `409 REFUND_AMOUNT_EXCEEDED` if the money has been
refunded elsewhere since.

### `POST /returns/{id}/reject`

Permission `sale.refund` at the store. Body `{ reason }`. Nothing moves;
the held quantities are released for a future return.

### `GET /returns/{id}/receipt?width=`

`sale.view` or `sale.return` at the store. `text/plain` return slip:
return number, original receipt number, lines with condition and refund,
and where the money went.

---

## 15. Customers

Customers belong to the tenant, not to a store, so these endpoints check
permissions against the store the caller is working at, sent as
`x-store-id`. A store-scoped cashier without that header gets `403`;
tenant-wide grants don't need it.

### Customer shape

```json
{
  "id": "0192...",
  "full_name": "Ada Obi",
  "phone": "0803******567",
  "email": "a***@example.com",
  "address": "***",
  "notes": null,
  "tax_identification_number": null,
  "customer_group_id": null,
  "store_credit_amount_minor": 100000,
  "marketing_consent": true,
  "home_store_id": "0192...",
  "created_at": "...",
  "updated_at": "...",
  "contact_masked": true
}
```

Callers without `customer.history` get the masked view shown: phone keeps
its first four and last three characters, email its first letter and
domain, address and tax id become `***`, notes are hidden, and
`contact_masked` is `true`.

A customer's group decides which price rules apply to them, so choosing one
(`customer_group_id` on create or edit, including clearing it) also needs
`product.manage` tenant-wide, and the group must be one of the caller's own
(`404` otherwise). A cashier can register and edit customers but cannot
move anyone to a cheaper pricing tier.

### `GET /customers?q=&cursor=&limit=`

Permission `customer.view`. `q` matches name, phone or email, or the
phone in any format (`0803 123 4567` and `+2348031234567` find the same
person). `{ items, next_cursor }`, by name. The old `query=` parameter
still works.

### `GET /customers/{id}`

Permission `customer.view`. `404` once erased.

### `POST /customers`

Permission `customer.manage` (cashiers hold it).

```json
{
  "full_name": "Ada Obi",
  "phone": "0803 123 4567",
  "email": "ada@example.com",
  "address": "12 Allen Ave",
  "notes": null,
  "tax_identification_number": null,
  "customer_group_id": null,
  "marketing_consent": true
}
```

Only `full_name` is required; a walk-in sale needs no customer at all.
`409 CUSTOMER_PHONE_EXISTS` when the number (in any format) is already
on file, so the till looks that customer up instead.

### `PATCH /customers/{id}`

Permission `customer.manage`. Any subset of the create fields; send
`null` to clear an optional field. Audited with before and after values.

### `DELETE /customers/{id}`

Permission `customer.manage`. Erases the person: name becomes
`Deleted customer`, contact details are removed, the record is soft
deleted. Their past sales stay, so totals still add up.

### `GET /customers/{id}/sales?cursor=&limit=`

Permission `customer.history` (no `x-store-id` needed). The customer's
purchase history, newest first, from the stores where the caller holds
`customer.history` only:

```json
{
  "items": [
    {
      "sale_id": "...",
      "receipt_number": "IKJ-00000042",
      "store_id": "...",
      "store_name": "Ikeja",
      "status": "completed",
      "payment_status": "paid",
      "total_amount_minor": 1000,
      "amount_refunded_minor": 0,
      "created_at": "..."
    }
  ],
  "next_cursor": null
}
```

---

## 16. Reports

### `GET /reports/payments/reconciliation?store_id=&shift_id=&from=&to=`

Permission `report.view` or `sale.view` at the store. `store_id` is
required; the rest narrow it to a shift or a time window (by when the sale
was rung up).

Payments and sales are summed independently over the same set of sales,
so a mismatch can't cancel itself out:

```json
{
  "store_id": "...",
  "shift_id": null,
  "from": null,
  "to": null,
  "by_method": [
    {
      "payment_method_id": "...",
      "name": "Cash",
      "kind": "cash",
      "count": 3,
      "collected_minor": 6500,
      "refunded_minor": 0,
      "reversed_minor": 0,
      "pending_minor": 0,
      "failed_minor": 0,
      "net_minor": 6500
    }
  ],
  "sales": {
    "completed_count": 2,
    "awaiting_payment_count": 0,
    "voided_count": 0,
    "due_minor": 10000,
    "amount_paid_minor": 10000,
    "outstanding_minor": 0,
    "refunded_minor": 0
  },
  "non_payment_refunds_minor": 0,
  "payments_net_minor": 10000,
  "sales_net_minor": 10000,
  "variance_minor": 0,
  "discrepancies": []
}
```

- `collected_minor` is money taken that still counts toward sales, including payments later refunded; `net_minor` is collected minus refunded.
- `non_payment_refunds_minor` is money refunded without going back through a payment (store credit, or a different method).
- `variance_minor` is `payments_net_minor - sales_net_minor` and should be `0`.
- `discrepancies` lists sale ids whose own paid or refunded totals disagree with their payment and refund rows.

---

## 17. Error codes

| Code                          | Status      | Meaning / what the client should do                                                                   |
| ----------------------------- | ----------- | ----------------------------------------------------------------------------------------------------- |
| `EMAIL_ALREADY_REGISTERED`    | 400         | Signup with an email in use. Offer login.                                                             |
| `EMAIL_NOT_VERIFIED`          | 400         | Login before verifying. Offer the OTP step.                                                           |
| `TENANT_REQUIRED`             | 409         | User belongs to several tenants; pick one from `data.tenants` and log in again.                       |
| `PLATFORM_TENANT_REQUIRED`    | 409         | Same, for platform staff.                                                                             |
| `ROLE_NOT_FOUND`              | 409         | The account has no active role in any tenant.                                                         |
| `SHIFT_ALREADY_OPEN`          | 409         | The store already has an open shift.                                                                  |
| `SHIFT_NOT_OPEN`              | 409/400     | The shift is closed (409) or belongs to another store (400).                                          |
| `STORE_INACTIVE`              | 400/409     | The store is deactivated (400 at checkout, 409 for transfers).                                        |
| `PRODUCT_NOT_SOLD_HERE`       | 400         | Unknown, inactive, or not listed at this store.                                                       |
| `PRODUCT_NOT_PRICED`          | 409         | No price rule applies; add one or override the price.                                                 |
| `INVALID_UNIT`                | 400         | The unit isn't one of the product's units.                                                            |
| `STOCK_ITEM_REQUIRED`         | 400         | Batch- or serial-tracked product without a valid `stock_item_id`.                                     |
| `PRODUCT_OUT_OF_STOCK`        | 409         | Not enough on hand (message has requested and available).                                             |
| `PRICE_OVERRIDE_FORBIDDEN`    | 403         | Override without `sale.price_override`, or without a reason.                                          |
| `DISCOUNT_FORBIDDEN`          | 403         | Discount above the cashier limit without `sale.discount`.                                             |
| `TOTAL_MISMATCH`              | 409         | The till's total is stale; re-quote and show the new total.                                           |
| `IDEMPOTENCY_CONFLICT`        | 409         | A different request already used this id; generate a new id.                                          |
| `PAYMENT_INSUFFICIENT`        | 400         | Payments don't cover the amount due.                                                                  |
| `PAYMENT_OVERPAID`            | 400         | Payments exceed what's due; record cash change as tendered.                                           |
| `PAYMENT_REFERENCE_REQUIRED`  | 400         | This method needs a transaction reference.                                                            |
| `PAYMENT_METHOD_INVALID`      | 400         | Unknown or inactive method, or one that can't be used here (store credit at checkout).                |
| `PAYMENT_METHOD_EXISTS`       | 409         | Payment method code already in use.                                                                   |
| `DUPLICATE_PAYMENT_REFERENCE` | 409         | That reference is already on a live payment of this method.                                           |
| `INVALID_PAYMENT_TRANSITION`  | 409         | The payment can't move to that status from where it is.                                               |
| `SALE_NOT_COMPLETED`          | 409         | Returns and later payments need a completed (or awaiting-payment) sale.                               |
| `SALE_NOT_VOIDABLE`           | 409         | Shift closed, or goods already returned/refunded; use a return.                                       |
| `RETURN_QUANTITY_EXCEEDED`    | 409         | More than is left to return on that line (message says how many).                                     |
| `ITEM_NOT_RETURNABLE`         | 400         | The product is marked non-returnable.                                                                 |
| `REFUND_AMOUNT_EXCEEDED`      | 409         | More than was paid and not yet refunded, on the sale or the payment.                                  |
| `RETURN_NOT_PENDING`          | 409         | The return isn't waiting for approval.                                                                |
| `APPROVAL_SELF_FORBIDDEN`     | 403         | The person who started a return can't approve it.                                                     |
| `CUSTOMER_REQUIRED`           | 400         | Store credit needs a customer on the sale.                                                            |
| `CUSTOMER_PHONE_EXISTS`       | 409         | A customer with this phone already exists; look them up.                                              |
| `BARCODE_EXISTS`              | 409         | The barcode is already assigned to a product.                                                         |
| `ALREADY_EXISTS`              | 409         | A record with these details exists (duplicate slug, SKU, unit, group or sale unit).                   |
| `GROUP_IN_USE`                | 409         | The customer group still has customers or price rules.                                                |
| `SHIFT_OPEN`                  | 409         | Close the store's open shift before deactivating the store.                                           |
| `PRODUCT_INACTIVE`            | 409         | The product is deactivated (transfers). Adjustments are still allowed.                                |
| `VALIDATION_FAILED`           | 400         | A field is missing, blank, too long, malformed or out of range; the message names it.                 |
| `INVALID_REFERENCE`           | 400         | A referenced record does not exist.                                                                   |
| `INVALID_VALUE`               | 400         | A value is outside what the database allows.                                                          |
| `INVALID_REQUEST`             | 400/415/422 | The body, path or query could not be read (bad JSON, wrong shape, wrong content type, unknown field). |
| `ROUTE_NOT_FOUND`             | 404         | No such endpoint.                                                                                     |
| `METHOD_NOT_ALLOWED`          | 405         | The endpoint exists but not for this HTTP method.                                                     |

`403` responses without a code carry `Missing permission: <name>`.
