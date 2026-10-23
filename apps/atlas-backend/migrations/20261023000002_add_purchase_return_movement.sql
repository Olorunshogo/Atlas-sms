-- FR-23. Own migration: can't use a new enum value in the transaction that adds it.
ALTER TYPE atlas.stock_movement_type ADD VALUE 'purchase_return';
