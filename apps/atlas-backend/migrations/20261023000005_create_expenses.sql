/*
 * FR-24 (Store Expense Management). Categories are their own small table
 * ("configurable by authorized users") rather than a free-text column, so
 * a typo can't silently fork a category in two.
 */
CREATE TABLE atlas.expense_categories (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, name)
);

ALTER TABLE atlas.expense_categories ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.expense_categories FORCE ROW LEVEL SECURITY;
CREATE POLICY expense_categories_isolation ON atlas.expense_categories
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TYPE atlas.expense_status AS ENUM ('draft', 'submitted', 'approved', 'rejected');
CREATE TYPE atlas.expense_payment_status AS ENUM ('unpaid', 'paid');

/*
 * `status` is the approval workflow (FR-24's "create, edit before
 * approval, submit, approve, reject"); `payment_status` is separate - an
 * approved expense is still owed until someone actually pays it.
 */
CREATE TABLE atlas.expenses (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES atlas.expense_categories(id),
    amount_minor BIGINT NOT NULL CHECK (amount_minor > 0),
    expense_date DATE NOT NULL,
    description TEXT NOT NULL,
    reference TEXT,
    status atlas.expense_status NOT NULL DEFAULT 'draft',
    payment_status atlas.expense_payment_status NOT NULL DEFAULT 'unpaid',

    created_by_user_id UUID NOT NULL REFERENCES atlas.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    submitted_at TIMESTAMPTZ,
    decided_by_user_id UUID REFERENCES atlas.users(id),
    decided_at TIMESTAMPTZ
);

CREATE INDEX expenses_store_date_idx ON atlas.expenses (store_id, expense_date DESC);
CREATE INDEX expenses_category_idx ON atlas.expenses (category_id);
CREATE INDEX expenses_status_idx ON atlas.expenses (tenant_id, status);

ALTER TABLE atlas.expenses ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.expenses FORCE ROW LEVEL SECURITY;
CREATE POLICY expenses_isolation ON atlas.expenses
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

UPDATE atlas.roles
SET permissions = permissions || ARRAY['expense.manage']::atlas.permission[]
WHERE is_system AND name IN ('owner', 'admin', 'store_manager', 'accountant', 'platform_admin');
