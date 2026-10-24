/*
 * The online storefront (approved beyond SRS v1.0's scope): storefront
 * settings, shipping zones, shopper accounts, carts, wishlists, orders with
 * online payments and stock reservations, promotions, loyalty points,
 * shipments, and product views for recommendations.
 */

CREATE TYPE atlas.shop_order_status AS ENUM ('pending_payment', 'paid', 'processing', 'shipped', 'delivered', 'cancelled', 'expired');
CREATE TYPE atlas.online_payment_method AS ENUM ('card', 'bank_transfer', 'ussd', 'mobile_wallet', 'pay_on_delivery');
CREATE TYPE atlas.online_payment_status AS ENUM ('pending', 'succeeded', 'failed', 'expired', 'refunded');
CREATE TYPE atlas.promotion_kind AS ENUM ('coupon', 'automatic', 'flash_sale', 'bundle');
CREATE TYPE atlas.promotion_discount AS ENUM ('percentage', 'fixed_amount', 'free_shipping');
CREATE TYPE atlas.reservation_status AS ENUM ('active', 'released', 'converted');
CREATE TYPE atlas.loyalty_entry_kind AS ENUM ('earn', 'redeem', 'reverse');
CREATE TYPE atlas.shipment_status AS ENUM ('created', 'in_transit', 'delivered', 'failed');

-- === Storefront settings

/*
 * One per store that sells online. `handle` is the public address
 * (/shop/{handle}), unique across every tenant. NULL pay-on-delivery limit
 * turns it off; NULL loyalty rate turns loyalty off.
 */
CREATE TABLE atlas.storefronts (
    store_id UUID PRIMARY KEY REFERENCES atlas.stores(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    handle TEXT NOT NULL CHECK (handle ~ '^[a-z0-9][a-z0-9-]{1,62}$'),
    is_enabled BOOLEAN NOT NULL DEFAULT false,
    reservation_minutes INT NOT NULL DEFAULT 15 CHECK (reservation_minutes > 0),
    ussd_window_minutes INT NOT NULL DEFAULT 30 CHECK (ussd_window_minutes > 0),
    bank_transfer_window_minutes INT NOT NULL DEFAULT 1440 CHECK (bank_transfer_window_minutes > 0),
    guest_cart_days INT NOT NULL DEFAULT 30 CHECK (guest_cart_days > 0),
    pay_on_delivery_max_minor BIGINT CHECK (pay_on_delivery_max_minor > 0),
    pay_on_delivery_requires_account BOOLEAN NOT NULL DEFAULT true,
    loyalty_minor_per_point BIGINT CHECK (loyalty_minor_per_point > 0),
    loyalty_point_value_minor BIGINT NOT NULL DEFAULT 100 CHECK (loyalty_point_value_minor > 0),
    loyalty_max_redeem_basis_points INT NOT NULL DEFAULT 2000 CHECK (loyalty_max_redeem_basis_points BETWEEN 0 AND 10000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX storefronts_handle_idx ON atlas.storefronts (lower(handle));

ALTER TABLE atlas.storefronts ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.storefronts FORCE ROW LEVEL SECURITY;
CREATE POLICY storefronts_isolation ON atlas.storefronts
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Delivery fee by zone and weight; `regions` are the states it covers
CREATE TABLE atlas.shipping_zones (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    regions TEXT[] NOT NULL DEFAULT '{}',
    base_fee_minor BIGINT NOT NULL CHECK (base_fee_minor >= 0),
    per_kg_fee_minor BIGINT NOT NULL DEFAULT 0 CHECK (per_kg_fee_minor >= 0),
    allows_pay_on_delivery BOOLEAN NOT NULL DEFAULT false,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX shipping_zones_store_idx ON atlas.shipping_zones (store_id);

ALTER TABLE atlas.shipping_zones ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shipping_zones FORCE ROW LEVEL SECURITY;
CREATE POLICY shipping_zones_isolation ON atlas.shipping_zones
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Shoppers

/*
 * A shopper account belongs to one business and is tied to that business's
 * customer record. `password_hash` is NULL for an account made through a
 * social login only.
 */
CREATE TABLE atlas.shoppers (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    customer_id UUID NOT NULL REFERENCES atlas.customers(id),
    email TEXT NOT NULL,
    full_name TEXT NOT NULL,
    phone TEXT,
    password_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX shoppers_tenant_email_idx ON atlas.shoppers (tenant_id, lower(email));

ALTER TABLE atlas.shoppers ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shoppers FORCE ROW LEVEL SECURITY;
CREATE POLICY shoppers_isolation ON atlas.shoppers
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.shopper_identities (
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    subject TEXT NOT NULL,
    shopper_id UUID NOT NULL REFERENCES atlas.shoppers(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (tenant_id, provider, subject)
);

ALTER TABLE atlas.shopper_identities ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shopper_identities FORCE ROW LEVEL SECURITY;
CREATE POLICY shopper_identities_isolation ON atlas.shopper_identities
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Carts and wishlists

-- A shopper's cart, or a guest's (found by the hash of an opaque token, kept guest_cart_days)
CREATE TABLE atlas.carts (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    shopper_id UUID REFERENCES atlas.shoppers(id) ON DELETE CASCADE,
    guest_token_hash TEXT,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT cart_owner CHECK ((shopper_id IS NULL) <> (guest_token_hash IS NULL))
);

CREATE UNIQUE INDEX carts_shopper_idx ON atlas.carts (store_id, shopper_id) WHERE shopper_id IS NOT NULL;
CREATE UNIQUE INDEX carts_guest_idx ON atlas.carts (guest_token_hash) WHERE guest_token_hash IS NOT NULL;

ALTER TABLE atlas.carts ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.carts FORCE ROW LEVEL SECURITY;
CREATE POLICY carts_isolation ON atlas.carts
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- `added_unit_price_minor` is the price when added, so the cart can say when it changed
CREATE TABLE atlas.cart_items (
    cart_id UUID NOT NULL REFERENCES atlas.carts(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    added_unit_price_minor BIGINT NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (cart_id, product_id)
);

ALTER TABLE atlas.cart_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.cart_items FORCE ROW LEVEL SECURITY;
CREATE POLICY cart_items_isolation ON atlas.cart_items
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.wishlist_items (
    shopper_id UUID NOT NULL REFERENCES atlas.shoppers(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (shopper_id, store_id, product_id)
);

ALTER TABLE atlas.wishlist_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.wishlist_items FORCE ROW LEVEL SECURITY;
CREATE POLICY wishlist_items_isolation ON atlas.wishlist_items
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Who looked at what, for "recently viewed"; `viewer_key` is a shopper id or a guest cart token hash
CREATE TABLE atlas.product_views (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    viewer_key TEXT NOT NULL,
    product_id UUID NOT NULL REFERENCES atlas.products(id) ON DELETE CASCADE,
    viewed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX product_views_viewer_idx ON atlas.product_views (store_id, viewer_key, viewed_at DESC);

ALTER TABLE atlas.product_views ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.product_views FORCE ROW LEVEL SECURITY;
CREATE POLICY product_views_isolation ON atlas.product_views
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Promotions

/*
 * One table for every kind. Eligibility narrows by products, categories,
 * customer group, first order and minimum spend; NULL or empty means no
 * restriction. `discount_value` is basis points for a percentage and minor
 * units for a fixed amount. A flash sale sets `flash_price_minor` (per base
 * unit) and caps how many units sell at it.
 */
CREATE TABLE atlas.promotions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID REFERENCES atlas.stores(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind atlas.promotion_kind NOT NULL,
    discount atlas.promotion_discount NOT NULL,
    discount_value BIGINT NOT NULL DEFAULT 0 CHECK (discount_value >= 0),
    coupon_code TEXT,
    product_ids UUID[] NOT NULL DEFAULT '{}',
    category_ids UUID[] NOT NULL DEFAULT '{}',
    bundle_product_ids UUID[] NOT NULL DEFAULT '{}',
    customer_group_id UUID REFERENCES atlas.customer_groups(id),
    first_order_only BOOLEAN NOT NULL DEFAULT false,
    min_spend_minor BIGINT,
    min_quantity NUMERIC,
    flash_price_minor BIGINT CHECK (flash_price_minor >= 0),
    flash_stock_cap NUMERIC CHECK (flash_stock_cap > 0),
    stackable BOOLEAN NOT NULL DEFAULT false,
    starts_at TIMESTAMPTZ,
    ends_at TIMESTAMPTZ,
    total_usage_limit INT CHECK (total_usage_limit > 0),
    per_customer_limit INT CHECK (per_customer_limit > 0),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT coupon_has_code CHECK ((kind = 'coupon') = (coupon_code IS NOT NULL)),
    CONSTRAINT flash_has_price CHECK ((kind = 'flash_sale') = (flash_price_minor IS NOT NULL))
);

CREATE UNIQUE INDEX promotions_coupon_idx ON atlas.promotions (tenant_id, upper(coupon_code)) WHERE coupon_code IS NOT NULL;

ALTER TABLE atlas.promotions ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.promotions FORCE ROW LEVEL SECURITY;
CREATE POLICY promotions_isolation ON atlas.promotions
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Orders

CREATE TABLE atlas.shop_orders (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    order_number TEXT NOT NULL,
    invoice_number TEXT NOT NULL,
    receipt_number TEXT,
    shopper_id UUID REFERENCES atlas.shoppers(id),
    customer_id UUID REFERENCES atlas.customers(id),
    email TEXT NOT NULL,
    phone TEXT NOT NULL,
    full_name TEXT NOT NULL,
    status atlas.shop_order_status NOT NULL DEFAULT 'pending_payment',
    delivery_address JSONB NOT NULL,
    shipping_zone_id UUID NOT NULL REFERENCES atlas.shipping_zones(id),
    weight_kg NUMERIC NOT NULL DEFAULT 0,
    payment_method atlas.online_payment_method NOT NULL,
    payment_expires_at TIMESTAMPTZ,
    prices_include_tax BOOLEAN NOT NULL,
    currency TEXT NOT NULL,
    subtotal_minor BIGINT NOT NULL,
    discount_minor BIGINT NOT NULL,
    shipping_minor BIGINT NOT NULL,
    tax_minor BIGINT NOT NULL,
    total_minor BIGINT NOT NULL,
    coupon_code TEXT,
    points_redeemed INT NOT NULL DEFAULT 0,
    points_value_minor BIGINT NOT NULL DEFAULT 0,
    idempotency_key TEXT NOT NULL,
    access_token_hash TEXT NOT NULL,
    invoice_snapshot JSONB NOT NULL,
    paid_at TIMESTAMPTZ,
    shipped_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (store_id, idempotency_key),
    UNIQUE (store_id, order_number)
);

CREATE INDEX shop_orders_store_status_idx ON atlas.shop_orders (store_id, status, created_at DESC);
CREATE INDEX shop_orders_shopper_idx ON atlas.shop_orders (shopper_id) WHERE shopper_id IS NOT NULL;

ALTER TABLE atlas.shop_orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shop_orders FORCE ROW LEVEL SECURITY;
CREATE POLICY shop_orders_isolation ON atlas.shop_orders
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.shop_order_lines (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    order_id UUID NOT NULL REFERENCES atlas.shop_orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id),
    product_name TEXT NOT NULL,
    sku TEXT,
    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    unit_price_minor BIGINT NOT NULL,
    discount_minor BIGINT NOT NULL,
    net_minor BIGINT NOT NULL,
    tax_rate_basis_points INT NOT NULL,
    tax_minor BIGINT NOT NULL,
    total_minor BIGINT NOT NULL,
    unit_cost_minor BIGINT
);

CREATE INDEX shop_order_lines_order_idx ON atlas.shop_order_lines (order_id);

ALTER TABLE atlas.shop_order_lines ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shop_order_lines FORCE ROW LEVEL SECURITY;
CREATE POLICY shop_order_lines_isolation ON atlas.shop_order_lines
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Every status change, who made it and when (FR-15)
CREATE TABLE atlas.shop_order_events (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    order_id UUID NOT NULL REFERENCES atlas.shop_orders(id) ON DELETE CASCADE,
    status atlas.shop_order_status NOT NULL,
    actor_user_id UUID REFERENCES atlas.users(id),
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX shop_order_events_order_idx ON atlas.shop_order_events (order_id, created_at);

ALTER TABLE atlas.shop_order_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shop_order_events FORCE ROW LEVEL SECURITY;
CREATE POLICY shop_order_events_isolation ON atlas.shop_order_events
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- One row per payment attempt; at most one succeeds
CREATE TABLE atlas.online_payments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    order_id UUID NOT NULL REFERENCES atlas.shop_orders(id) ON DELETE CASCADE,
    method atlas.online_payment_method NOT NULL,
    status atlas.online_payment_status NOT NULL DEFAULT 'pending',
    amount_minor BIGINT NOT NULL,
    idempotency_key TEXT NOT NULL,
    gateway_ref TEXT,
    redirect_url TEXT,
    failure_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    confirmed_at TIMESTAMPTZ,

    UNIQUE (order_id, idempotency_key)
);

CREATE UNIQUE INDEX online_payments_gateway_ref_idx ON atlas.online_payments (gateway_ref) WHERE gateway_ref IS NOT NULL;
CREATE UNIQUE INDEX online_payments_one_success_idx ON atlas.online_payments (order_id) WHERE status = 'succeeded';

ALTER TABLE atlas.online_payments ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.online_payments FORCE ROW LEVEL SECURITY;
CREATE POLICY online_payments_isolation ON atlas.online_payments
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- Stock held for an order between checkout and payment; NULL expiry for pay on delivery
CREATE TABLE atlas.stock_reservations (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    order_id UUID NOT NULL REFERENCES atlas.shop_orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES atlas.products(id),
    quantity NUMERIC NOT NULL CHECK (quantity > 0),
    status atlas.reservation_status NOT NULL DEFAULT 'active',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX stock_reservations_active_idx ON atlas.stock_reservations (store_id, product_id) WHERE status = 'active';

ALTER TABLE atlas.stock_reservations ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.stock_reservations FORCE ROW LEVEL SECURITY;
CREATE POLICY stock_reservations_isolation ON atlas.stock_reservations
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- What each order took from a promotion, for usage limits and flash sale caps
CREATE TABLE atlas.promotion_redemptions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    promotion_id UUID NOT NULL REFERENCES atlas.promotions(id) ON DELETE CASCADE,
    order_id UUID NOT NULL REFERENCES atlas.shop_orders(id) ON DELETE CASCADE,
    shopper_id UUID REFERENCES atlas.shoppers(id),
    email TEXT NOT NULL,
    discount_minor BIGINT NOT NULL,
    units NUMERIC NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX promotion_redemptions_promotion_idx ON atlas.promotion_redemptions (promotion_id);

ALTER TABLE atlas.promotion_redemptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.promotion_redemptions FORCE ROW LEVEL SECURITY;
CREATE POLICY promotion_redemptions_isolation ON atlas.promotion_redemptions
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.shop_number_sequences (
    store_id UUID NOT NULL REFERENCES atlas.stores(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    last_value BIGINT NOT NULL DEFAULT 0,

    PRIMARY KEY (store_id, kind)
);

ALTER TABLE atlas.shop_number_sequences ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shop_number_sequences FORCE ROW LEVEL SECURITY;
CREATE POLICY shop_number_sequences_isolation ON atlas.shop_number_sequences
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Loyalty

-- Signed points; a balance only counts entries whose available_at has passed
CREATE TABLE atlas.loyalty_entries (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    shopper_id UUID NOT NULL REFERENCES atlas.shoppers(id) ON DELETE CASCADE,
    order_id UUID REFERENCES atlas.shop_orders(id) ON DELETE SET NULL,
    kind atlas.loyalty_entry_kind NOT NULL,
    points INT NOT NULL CHECK (points <> 0),
    available_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX loyalty_entries_shopper_idx ON atlas.loyalty_entries (shopper_id, available_at);

ALTER TABLE atlas.loyalty_entries ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.loyalty_entries FORCE ROW LEVEL SECURITY;
CREATE POLICY loyalty_entries_isolation ON atlas.loyalty_entries
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

-- === Shipments

CREATE TABLE atlas.shipments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    order_id UUID NOT NULL REFERENCES atlas.shop_orders(id) ON DELETE CASCADE,
    courier TEXT NOT NULL,
    tracking_number TEXT NOT NULL UNIQUE,
    label_url TEXT NOT NULL,
    status atlas.shipment_status NOT NULL DEFAULT 'created',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE atlas.shipments ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shipments FORCE ROW LEVEL SECURITY;
CREATE POLICY shipments_isolation ON atlas.shipments
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

CREATE TABLE atlas.shipment_events (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES atlas.tenants(id) ON DELETE CASCADE,
    shipment_id UUID NOT NULL REFERENCES atlas.shipments(id) ON DELETE CASCADE,
    status atlas.shipment_status NOT NULL,
    note TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE atlas.shipment_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE atlas.shipment_events FORCE ROW LEVEL SECURITY;
CREATE POLICY shipment_events_isolation ON atlas.shipment_events
    USING (
        coalesce(nullif(current_setting('app.bypass_tenant', true), ''), 'off') = 'on'
        OR tenant_id = nullif(current_setting('app.current_tenant_id', true), '')::uuid
    );

UPDATE atlas.roles
SET permissions = permissions || ARRAY['storefront.manage', 'promotion.manage', 'order.manage']::atlas.permission[]
WHERE is_system AND name IN ('owner', 'admin', 'platform_admin');

UPDATE atlas.roles
SET permissions = permissions || ARRAY['promotion.manage', 'order.manage']::atlas.permission[]
WHERE is_system AND name = 'store_manager';
