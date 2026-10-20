# Systems Requirement Document

A **Store Management System (SMS)** is a software system used to manage a store’s products, inventory, sales, purchases, customers, suppliers, and staff.

 ## Main requirements of a Store Management System

 ### 1\. User Management

 The system should allow:

 - User registration and login
- Different roles such as **Administrator, Manager, Cashier, and Storekeeper**
- Role-based permissions
- Password management
- User activity/audit logs

 ### 2\. Product Management

 The system should allow staff to:

 - Add, edit, and delete products
- Assign product categories
- Record product codes/SKUs
- Record prices and costs
- Track product quantities
- Set minimum/reorder stock levels
- Search and filter products
- Support product images or descriptions where necessary

 ### 3\. Inventory/Stock Management

 The system should:

 - Record incoming stock
- Record outgoing stock
- Automatically update stock after sales and purchases
- Track damaged, expired, or returned goods
- Provide current stock levels
- Generate low-stock alerts
- Maintain stock movement/history
- Support stock adjustments and physical stock counts

 ### 4\. Sales Management

 The system should support:

 - Creating sales transactions
- Generating receipts/invoices
- Calculating totals, discounts, and taxes where applicable
- Multiple payment methods
- Sales returns/refunds
- Updating inventory automatically after a sale
- Viewing sales history

 ### 5\. Purchasing Management

 The system should allow:

 - Creating purchase orders
- Recording suppliers
- Recording goods received
- Tracking purchase costs
- Updating inventory when purchases are received
- Managing cancelled or returned purchases

 ### 6\. Supplier Management

 The system should store:

 - Supplier name and contact details
- Products supplied
- Purchase history
- Outstanding balances, if credit purchases are supported
- Supplier transaction history

 ### 7\. Customer Management

 Depending on the type of store, the system may include:

 - Customer registration
- Customer contact information
- Purchase history
- Loyalty points
- Credit/debt tracking
- Customer returns

 ### 8\. Reporting

 The system should generate reports such as:

 - Daily/weekly/monthly sales
- Profit and revenue reports
- Inventory reports
- Low-stock reports
- Purchase reports
- Product performance
- Customer purchase history
- Supplier reports
- Stock movement reports

 ### 9\. Dashboard

 A dashboard can provide an overview of:

 - Total sales
- Number of transactions
- Current inventory
- Low-stock products
- Recent purchases
- Recent sales
- Revenue/profit summaries

 ### 10\. Security

 The system should provide:

- Authentication and authorization
- Password protection
- Role-based access control
- Data validation
- Database backups
- Audit trails
- Protection against unauthorized modification or deletion

 ## Non-functional requirements

 The system should also have qualities such as:

 - **Usability:** Easy for store employees to learn and use.
- **Performance:** Sales and inventory transactions should be processed quickly.
- **Reliability:** The system should minimize data loss and errors.
- **Scalability:** It should be possible to add more products, users, or branches.
- **Availability:** The system should be accessible when required.
- **Maintainability:** Developers should be able to modify and maintain it easily.
- **Security:** Store and customer information should be protected.
- **Backup and recovery:** Data should be recoverable after failures.

 ## Basic database requirements

 A typical system might have tables/entities such as:

- `Users`
- `Roles`
- `Products`
- `Categories`
- `Inventory`
- `Sales`
- `SaleItems`
- `Purchases`
- `PurchaseItems`
- `Suppliers`
- `Customers`
- `Payments`
- `Returns`
- `StockMovements`

 ### Example workflow

 **Purchase stock → Receive goods → Inventory increases → Customer buys product → Sale recorded → Inventory decreases → Receipt generated → Sales/reporting data updated.**

 If you're writing this for a **school/college project**, these requirements can be turned into a formal **System Requirements Specification (SRS)** containing functional requirements, non-functional requirements, use cases, database requirements, and system constraints.


## Functional Requirements (30)

### Accounts and access

FR-01 The system shall let customers register and log in with email, phone, or social login, and shall verify email or phone before the first order.
FR-02 The system shall let customers manage their profile, saved addresses, saved payment tokens, and notification preferences.
FR-03 Admins shall create staff accounts and assign roles (Super Admin, Store Manager, Inventory Officer, Order Fulfilment Staff, Customer Support, Finance). Staff shall not self-register.
FR-04 The system shall enforce role-based permissions on every admin action, configurable per role.

### Catalogue

FR-05 Staff shall create, edit, archive, and bulk import/export products with SKU, barcode, categories, brand, attributes, images, description, cost price, and selling price.
FR-06 The system shall support product variants (size, color, etc.), each with its own SKU, price, stock, and images.
FR-07 The system shall keep a price history for every product and support scheduled price changes and sale prices with start and end dates.
FR-08 Customers shall be able to search by keyword, with autocomplete, typo tolerance, and filters (category, price, brand, rating, attributes, availability) and sorting.
FR-09 The system shall show personalized recommendations: related items, frequently bought together, and recently viewed.

### Cart, checkout, and payment

FR-10 Customers shall be able to add items to a cart and wishlist. The cart shall persist across devices for logged-in users and reserve stock for a set time during checkout.
FR-11 Checkout shall support guest and registered users and shall calculate subtotal, tax, shipping, and discounts before payment.
FR-12 The system shall support multiple payment methods (cards, bank transfer, USSD, mobile wallets, pay on delivery) through PCI-compliant gateways, and handle failed or pending payments.
FR-13 The system shall support coupons, automatic promotions, flash sales, bundle deals, and loyalty points, with rules for eligibility, limits, and stacking.
FR-14 The system shall issue invoices and receipts with tax breakdown and send them by email.

### Orders, fulfilment, and returns

FR-15 The system shall manage the order lifecycle (pending, paid, processing, packed, shipped, delivered, cancelled, returned) and record every status change with user and timestamp.
FR-16 The system shall integrate with courier services to generate shipping labels, calculate rates by zone and weight, and provide tracking numbers.
FR-17 Customers shall be able to track orders and receive email/SMS/push notifications at each status change.
FR-18 Customers shall be able to cancel orders before shipping and request returns within the return window. Staff shall approve or reject returns, and approved returns shall trigger a refund and a stock update.
FR-19 Staff shall be able to process full and partial refunds to the original payment method or to store credit.

### Inventory and purchasing

FR-20 The system shall track stock per warehouse or location and record every movement (sale, purchase, return, transfer, adjustment, write-off) in a stock movement log.
FR-21 The system shall track batch/lot numbers and expiry dates where applicable and allocate stock first-expiry-first-out.
FR-22 The system shall alert staff when stock falls below the reorder level and suggest reorder quantities.
FR-23 Staff shall manage suppliers and create purchase orders, receive full or partial deliveries, and record purchase returns. Received goods shall update stock automatically.
FR-24 Staff shall be able to transfer stock between locations and run physical stock counts, with discrepancies recorded as adjustments.
FR-25 Adjustments, write-offs, refunds, and discounts above set thresholds shall require manager approval.

### Customers, support, and content

FR-26 Verified buyers shall be able to rate and review products, and staff shall moderate reviews.
FR-27 Customers shall be able to contact support through live chat, tickets, or a contact form, and staff shall see the customer’s order history while handling the request.
FR-28 Admins shall manage site content: banners, homepage sections, static pages, and FAQs.

### Reporting and audit

FR-29 The system shall provide a dashboard and reports on sales, revenue, profit margin, inventory value, low stock, product performance, customer behaviour, supplier performance, and returns, filterable by date and exportable to CSV/PDF.
FR-30 The system shall keep an audit log of all admin actions (who, what, when, before/after values) that cannot be edited or deleted.

## Non-Functional Requirements (30)

### Performance

NFR-01 Product and category pages shall load within 2 seconds at the 95th percentile on a 4G connection.
NFR-02 Search results shall return within 500 ms for a catalogue of up to 1 million SKUs.
NFR-03 Checkout, from “Place Order” to confirmation, shall complete within 5 seconds, excluding external payment authentication.
NFR-04 Admin reports covering up to one year of data shall generate within 10 seconds.

### Scalability and capacity

NFR-05 The system shall support 10,000 concurrent users and 500 orders per minute without degrading beyond NFR-01.
NFR-06 The system shall scale horizontally to handle 5× normal traffic during sales events.
NFR-07 The architecture shall support adding new warehouses, payment providers, and couriers without changes to core modules.

### Availability and reliability

NFR-08 The storefront shall have 99.9% monthly uptime.
NFR-09 Stock levels shall stay consistent under concurrent orders, so no item is oversold.
NFR-10 Payment processing shall be idempotent, so a retry never charges a customer twice.
NFR-11 If a third-party service (courier, SMS, email) fails, the system shall queue and retry the request without blocking checkout.

### Backup and recovery

NFR-12 Full database backups shall run daily, with incremental backups at least every 15 minutes, stored in a separate region.
NFR-13 Recovery Time Objective shall be 1 hour or less, and Recovery Point Objective 15 minutes or less.
NFR-14 Restores from backup shall be tested at least once per quarter.

### Security

NFR-15 All traffic shall use TLS 1.2 or higher, and sensitive data shall be encrypted at rest with AES-256.
NFR-16 Passwords shall be hashed with bcrypt or Argon2. Accounts shall lock after 5 failed login attempts.
NFR-17 Staff accounts shall require two-factor authentication. Admin sessions shall expire after 15 minutes of inactivity.
NFR-18 The system shall be PCI-DSS compliant and shall never store full card numbers or CVVs.
NFR-19 The system shall protect against the OWASP Top 10 and pass a penetration test before launch and yearly after.
NFR-20 APIs shall be rate-limited to prevent abuse and bot attacks.

### Usability and accessibility

NFR-21 The site shall be fully responsive from 320 px mobile screens to desktop.
NFR-22 The storefront shall meet WCAG 2.1 Level AA.
NFR-23 A first-time customer shall be able to complete a purchase in 4 checkout steps or fewer.
NFR-24 New staff shall be able to process an order end-to-end after 1 hour of training.

### Compatibility and maintainability

NFR-25 The site shall work on the latest two versions of Chrome, Safari, Firefox, and Edge, and on Android 10+ and iOS 15+.
NFR-26 Code shall have at least 70% automated test coverage on core modules (checkout, payments, inventory), and all public APIs shall be documented.
NFR-27 Deployments shall cause zero downtime and support rollback within 10 minutes.
NFR-28 The system shall log errors and performance metrics centrally and alert the on-call team within 5 minutes of a critical failure.

### Compliance and localization

NFR-29 The system shall comply with data protection laws (NDPA in Nigeria, GDPR for EU customers), including consent management and customer requests to export or delete their data within 30 days.
NFR-30 The system shall support multiple currencies, languages, and tax rules, and pages shall be SEO-friendly with clean URLs, meta tags, and a sitemap.