/*
 * "4. Module toggles per store" (docs/Atlas/README.md): recipes/ingredient
 * depletion, table orders, layaway, repairs, delivery, loyalty. Behind
 * each toggle is real code, not configuration - a store with the module
 * off never sees it and never pays for it. One row per store; a module
 * with no row is off by default.
 */
CREATE TYPE atlas.store_module AS ENUM (
    'recipes', 'table_orders', 'layaway', 'repairs', 'delivery', 'loyalty'
);

CREATE TABLE atlas.store_module_toggles (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    module atlas.store_module NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT true,

    UNIQUE (store_id, module)
);

ALTER TABLE atlas.store_module_toggles ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.store_module_toggles FORCE ROW LEVEL SECURITY;
CREATE POLICY store_module_toggles_isolation ON atlas.store_module_toggles
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
