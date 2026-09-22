UPDATE atlas.roles
SET permissions = permissions || 'product.manage'::atlas.permission
WHERE name IN ('owner', 'admin', 'store_manager');
