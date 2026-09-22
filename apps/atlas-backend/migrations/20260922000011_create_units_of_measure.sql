/*
 * "2. Units of measure" (docs/Atlas/README.md). A tenant defines its own
 * units (piece, kg, metre, carton...); a product picks one base unit that
 * stock is held in, plus zero or more sale units with a conversion factor
 * back to base. `factor_to_base` is NUMERIC, never integer - a hardware
 * shop selling half-bags needs 0.5, not a rounding error.
 */
CREATE TABLE atlas.units_of_measure (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    abbreviation TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, name)
);

CREATE INDEX units_of_measure_tenant_idx ON atlas.units_of_measure (tenant_id);

ALTER TABLE atlas.units_of_measure ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.units_of_measure FORCE ROW LEVEL SECURITY;
CREATE POLICY units_of_measure_isolation ON atlas.units_of_measure
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );
