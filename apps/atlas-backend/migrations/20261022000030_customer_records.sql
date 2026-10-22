/*
 * Customer records for FR-19: optional contact details, approved business
 * fields, consent, and the phone number normalised so one person can't end
 * up as three profiles ("0803...", "+234803...", "234 803 ...").
 *
 * Rows created before this migration keep phone_normalized NULL until
 * their next edit; the uniqueness rule only applies to normalised numbers.
 */
ALTER TABLE atlas.customers
    ADD COLUMN phone_normalized TEXT,
    ADD COLUMN address TEXT,
    ADD COLUMN notes TEXT,
    ADD COLUMN tax_identification_number TEXT,
    ADD COLUMN marketing_consent BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN consent_recorded_at TIMESTAMPTZ,
    ADD COLUMN home_store_id UUID REFERENCES atlas.stores(id),
    ADD COLUMN created_by_user_id UUID REFERENCES atlas.users(id),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE UNIQUE INDEX customers_tenant_phone_idx
    ON atlas.customers (tenant_id, phone_normalized)
    WHERE deleted_at IS NULL AND phone_normalized IS NOT NULL;

CREATE INDEX customers_tenant_name_idx ON atlas.customers (tenant_id, full_name, id);

-- Purchase history is read newest first per customer.
CREATE INDEX sales_customer_created_idx ON atlas.sales (customer_id, created_at DESC);
