/*
 * Supports returns and voids. A sale line is priced and sold in a sale
 * unit (a carton), but stock moves in the product's base unit (pieces), and
 * the carton's conversion factor can be edited or removed later. So the
 * exact base quantity that left stock is recorded on the line at sale time,
 * and a return or void puts back precisely that, never a recomputed guess.
 *
 * Returns of one request share a `return_id` (one row per returned line);
 * `base_quantity` is how much went back to stock-equivalent for that row.
 * Voided sales keep their lines and ledger; only status and the void
 * columns change.
 */
SET LOCAL app.bypass_tenant = 'on';

ALTER TABLE atlas.sale_lines ADD COLUMN base_quantity NUMERIC;

UPDATE atlas.sale_lines sl
SET base_quantity = sl.quantity * COALESCE(
    CASE WHEN p.base_unit_id = sl.unit_id THEN 1::numeric
         ELSE (SELECT psu.factor_to_base FROM atlas.product_sale_units psu
               WHERE psu.product_id = sl.product_id AND psu.unit_id = sl.unit_id)
    END, 1)
FROM atlas.products p
WHERE p.id = sl.product_id;

ALTER TABLE atlas.sale_lines
    ALTER COLUMN base_quantity SET NOT NULL,
    ADD CONSTRAINT sale_lines_base_quantity_positive CHECK (base_quantity > 0);

ALTER TABLE atlas.sale_returns
    ADD COLUMN return_id UUID,
    ADD COLUMN base_quantity NUMERIC,
    ADD COLUMN returned_to_stock BOOLEAN NOT NULL DEFAULT true;

UPDATE atlas.sale_returns SET return_id = id, base_quantity = quantity;

ALTER TABLE atlas.sale_returns
    ALTER COLUMN return_id SET NOT NULL,
    ALTER COLUMN base_quantity SET NOT NULL,
    ADD CONSTRAINT sale_returns_base_quantity_positive CHECK (base_quantity > 0),
    ADD CONSTRAINT sale_returns_refund_not_negative CHECK (refund_amount_minor >= 0);

CREATE INDEX sale_returns_line_idx ON atlas.sale_returns (sale_line_id);

ALTER TABLE atlas.sales ADD COLUMN void_reason TEXT;
ALTER TABLE atlas.sales
    ADD CONSTRAINT sales_void_fields_together CHECK (
        (status = 'voided') = (voided_at IS NOT NULL AND voided_by_user_id IS NOT NULL AND void_reason IS NOT NULL)
    );
