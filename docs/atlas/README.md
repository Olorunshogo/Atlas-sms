# Store Management System

One system that runs many stores. Each store keeps its own inventory, prices, staff, customers and sales. The system above them holds the shared catalogue, the shared rules and the reporting that spans every branch. One codebase, one schema, many isolated stores.

---

## Why this exists

Small chains run on WhatsApp, exercise books and one spreadsheet per branch. Any single store is manageable that way. Visibility across stores is not.

The moment there is a second branch, the owner loses the ability to answer basic questions without phoning someone:

- How much stock is in Ikeja right now?
- Which branch sold the most yesterday, and at what margin?
- Did the cashier at Lekki close short last night?
- Who marked those twelve cartons as damaged, and when?

This is a stock and cash control problem before it is anything else. Staff get a faster till and less counting. The owner gets numbers that were true five seconds ago, across every branch, without asking anyone.

---

## Design goal

Model a store well enough that a pharmacy, a boutique, a hardware shop and a small restaurant all fit without forking the code.

That constraint drives every decision below.

---

## What differs between store types

Worth naming precisely, because it decides the schema.

A pharmacy needs batch numbers, expiry dates, controlled drug registers and a named pharmacist against certain sales. A boutique needs size and colour variants, and one shirt is one shirt. A hardware shop sells by metre, by kilo and by half bag, so quantity is decimal and one product has several sale units. A restaurant does not sell stock in the usual sense. It sells dishes that consume ingredients, which means recipes and depletion. A phone shop tracks serial numbers down to the individual unit for warranty. A supermarket needs weighing scale barcodes and fast checkout above all else.

The pattern is narrow. What varies is the product model, the unit model, the identity of a stock item, and the checkout flow. Almost nothing else.

---

## The core that never varies

Built once, identical for every store type. This is roughly eighty percent of the work.

### Stores
Create a store, set its currency, tax treatment, opening hours, address, receipt layout, and its own settings that override tenant defaults.

### People and access
Owners, managers, cashiers, stock clerks. A person belongs to one or more stores, with a role in each.

### Catalogue and inventory
Products, variants, barcodes, units, cost price and selling price per store, stock levels, low stock alerts, transfers between stores, stock counts, adjustments with a mandatory reason on every movement.

### Sales
Point of sale, receipts, discounts, returns, held carts, shift open and close with cash reconciliation.

### Purchasing
Suppliers, purchase orders, goods received notes, supplier balances.

### Customers
Profiles, purchase history, store credit, loyalty where the store wants it.

### Money
Daily takings per store, expenses, profit per store, profit per product, comparisons across branches.

**Refunds are recorded, not paid.** A return or a void works out and stores how much the customer is owed (`sale_returns.refund_amount_minor`, and the sale total for a void), but nothing pays that money back, because there is no payment system yet. The amount is inferred from the sale until the payment system is implemented, and how it is paid out (cash from the till, a card reversal, store credit) is not decided here. When payments land, a refund should become a payment-system action that references the return, rather than a second source of truth for the amount.

### Audit
Who changed what, when, from which device. In a system handling stock and cash this is not optional and it is not a later feature.

---

## How variation is absorbed

Four mechanisms handle nearly all of it.

### 1. Product types with attribute schemas

A product belongs to a product type. The type declares its attributes: name, data type, required or not, searchable or not, and whether the attribute identifies a distinct stock item.

No columns for expiry, size, serial or grade. A tenant defines new types without a deployment. Values live in JSON on the row, `jsonb` on PostgreSQL or a JSON column with computed indexes on SQL Server. Index only the handful of attributes that genuinely need querying.

### 2. Units of measure

A product has one base unit that stock is held in, plus one or more sale units with conversion factors back to the base.

Quantity is decimal everywhere. Never integer. This is trivial at the start and expensive to retrofit.

### 3. Stock item identity

Some products track by product alone. Some by batch. Some by serial. Some by both.

That is a small enumeration on the product type, and every stock movement carries the identifying values. The pharmacy and phone shop behaviour falls out of this without special cases in the sales code.

### 4. Module toggles per store

Recipes and ingredient depletion, table orders, layaway, repairs, delivery, loyalty.

Behind each toggle is real code, not configuration. A store with the module off never sees it and never pays for it.

### Pricing, which sits slightly apart

Price is not a column on a product. It is the resolved answer to a question: this product, this store, this customer group, this quantity, this date.

Model it as price rules with a defined resolution order. Branch pricing, wholesale tiers and promotions all come from that one mechanism.

---

## Where the line sits

Configuration is right when the variation is data shaped: fields, units, prices, tax rates, receipt layout, permissions.

Code behind a toggle is right when the variation is behaviour shaped: recipe depletion, controlled drug logging, serial capture at checkout.

Confusing the two produces a rules engine harder to use than the module would have been to write. This is the main way a system like this fails.

---

## Tenancy

Two levels of isolation, and they are not the same thing.

A tenant is the business. A store is a branch inside it. Data is isolated at tenant level and partitioned at store level.

Working default: a shared database, `TenantId` on every row, a global query filter in EF Core.

Rules that go with it:

- Resolve the tenant from the authenticated principal. Never from a header, route value or anything else the client controls.
- `TenantId` comes first in every composite index.
- One integration test proves that a query written without the filter fails.

Database per tenant is cleaner on paper and will cost more in migration pain than it returns, at least until a customer demands it for compliance. Keep the option open. Do not start there.

### Permissions

A permission is not "can void sale". It is "can void sale, in store 3".

The grant is user plus store plus role. Both halves are checked on every call.

---

## Offline

A shop in Lagos cannot stop selling because a modem blinked.

If offline selling is in scope, three things are needed from the first commit:

1. Client generated identifiers on sales.
2. A device identifier on every record.
3. An idempotent sync endpoint.

Bolting this on later means rewriting the point of sale. Decide before writing the sales module, not after.

---

## Proposed stack

- ASP.NET Core for the API
- EF Core for the domain, Dapper for reporting queries and anything hot
- PostgreSQL as primary, given the JSON attribute model
- Docker, with CI/CD through Azure DevOps

Substitutable. The design above does not depend on any of it.

---

## Build order

1. Tenancy, stores, users, roles, permissions, audit.
2. Product types, attributes, units, products.
3. Stock movements and the ledger they write to.
4. Point of sale and shifts.
5. Purchasing and suppliers.
6. Reporting.
7. Modules, one at a time, behind toggles.

Steps 1 to 3 are the foundation. Every shortcut taken there is paid back with interest.

---

## Open decisions

- Which two store types to build against first. Pick two that are genuinely unlike each other, a pharmacy and a boutique for instance, and make both work before generalising further.
- Whether offline selling is in scope for version one.
- Whether the point of sale is web, desktop or mobile.
- Whether tenants can define product types themselves, or only support staff can.
