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
