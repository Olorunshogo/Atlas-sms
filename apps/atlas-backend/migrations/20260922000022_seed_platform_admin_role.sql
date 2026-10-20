/*
 * Atlas's own internal staff role - scope 'platform', tenant_id NULL like
 * every other system role. Never self-assignable (no signup path grants
 * it), never invite-flow-assignable by a tenant (the invite design in
 * docs/atlas/auth-flow.md only ever grants tenant/store-scope roles) -
 * provisioned by direct database action until an internal admin tool
 * exists.
 */
INSERT INTO atlas.roles (id, tenant_id, name, permissions, is_system) VALUES
    ('00000000-0000-7000-8000-000000000007', NULL, 'platform_admin', ARRAY[
        'sale.create', 'sale.void', 'sale.refund',
        'stock.view', 'stock.adjust', 'stock.transfer',
        'purchase.create', 'supplier.manage',
        'staff.invite', 'staff.remove',
        'store.create', 'store.update',
        'role.create', 'role.assign',
        'report.view', 'tenant.manage', 'product.manage'
    ]::atlas.permission[], true);
