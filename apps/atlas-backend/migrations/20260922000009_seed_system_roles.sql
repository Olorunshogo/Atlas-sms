/*
 * The seeded roles every tenant starts with (tenant_id NULL, is_system
 * true, never editable - see roles::service and CONTEXT.md item 2). IDs
 * are fixed UUIDs, not app-generated, so signup can look a role up by a
 * known constant without a name lookup on every request.
 */
INSERT INTO atlas.roles (id, tenant_id, name, permissions, is_system) VALUES
    ('00000000-0000-7000-8000-000000000001', NULL, 'owner', ARRAY[
        'sale.create', 'sale.void', 'sale.refund',
        'stock.view', 'stock.adjust', 'stock.transfer',
        'purchase.create', 'supplier.manage',
        'staff.invite', 'staff.remove',
        'store.create', 'store.update',
        'role.create', 'role.assign',
        'report.view', 'tenant.manage'
    ]::atlas.permission[], true),

    ('00000000-0000-7000-8000-000000000002', NULL, 'admin', ARRAY[
        'sale.create', 'sale.void', 'sale.refund',
        'stock.view', 'stock.adjust', 'stock.transfer',
        'purchase.create', 'supplier.manage',
        'staff.invite', 'staff.remove',
        'store.create', 'store.update',
        'role.create', 'role.assign',
        'report.view'
    ]::atlas.permission[], true),

    ('00000000-0000-7000-8000-000000000003', NULL, 'store_manager', ARRAY[
        'sale.create', 'sale.void', 'sale.refund',
        'stock.view', 'stock.adjust', 'stock.transfer',
        'purchase.create', 'supplier.manage',
        'staff.invite',
        'report.view'
    ]::atlas.permission[], true),

    ('00000000-0000-7000-8000-000000000004', NULL, 'cashier', ARRAY[
        'sale.create', 'stock.view'
    ]::atlas.permission[], true),

    ('00000000-0000-7000-8000-000000000005', NULL, 'stock_clerk', ARRAY[
        'stock.view', 'stock.adjust', 'stock.transfer'
    ]::atlas.permission[], true),

    ('00000000-0000-7000-8000-000000000006', NULL, 'accountant', ARRAY[
        'stock.view', 'report.view'
    ]::atlas.permission[], true);
