/*
 * Hands the new permissions to the seeded system roles. Managers and above
 * get all of them. A cashier can look up and register customers and start
 * a return; anything above the store's refund approval threshold still
 * needs a `sale.refund` holder to approve it. The accountant reads sales
 * and customer history but never changes either.
 */
UPDATE atlas.roles
SET permissions = permissions || ARRAY[
        'sale.view', 'sale.discount', 'sale.price_override', 'sale.return',
        'receipt.reprint', 'customer.view', 'customer.manage', 'customer.history',
        'payment.manage'
    ]::atlas.permission[]
WHERE is_system AND name IN ('owner', 'admin', 'store_manager', 'platform_admin');

UPDATE atlas.roles
SET permissions = permissions || ARRAY['customer.view', 'customer.manage', 'sale.return']::atlas.permission[]
WHERE is_system AND name = 'cashier';

UPDATE atlas.roles
SET permissions = permissions || ARRAY['sale.view', 'customer.history']::atlas.permission[]
WHERE is_system AND name = 'accountant';
