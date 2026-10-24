/**
 * Shapes the shop API returns, mirroring `apps/atlas-backend/src/shop`. Quantities are decimal strings,
 * money is integer minor units.
 */

export interface ShopZone {
  id: string;
  name: string;
  regions: string[];
  base_fee_minor: number;
  per_kg_fee_minor: number;
  allows_pay_on_delivery: boolean;
}

export interface ShopInfo {
  handle: string;
  store_name: string;
  currency: string;
  prices_include_tax: boolean;
  shipping_zones: ShopZone[];
  pay_on_delivery_max_minor: number | null;
  loyalty_enabled: boolean;
  loyalty_point_value_minor: number;
}

export interface ShopCategory {
  id: string;
  name: string;
}

export interface CatalogProduct {
  id: string;
  name: string;
  sku: string | null;
  category_id: string | null;
  attributes: Record<string, unknown>;
  price_minor: number;
  sale_price_minor: number | null;
  available: string;
  in_stock: boolean;
}

export interface Recommendations {
  related: CatalogProduct[];
  frequently_bought_together: CatalogProduct[];
  recently_viewed: CatalogProduct[];
}

export interface CartItem {
  product_id: string;
  name: string;
  sku: string | null;
  quantity: string;
  unit_price_minor: number;
  added_unit_price_minor: number;
  price_changed: boolean;
  available: string;
  in_stock: boolean;
  line_total_minor: number;
}

export interface CartView {
  items: CartItem[];
  subtotal_minor: number;
}

export interface Shopper {
  id: string;
  customer_id: string;
  email: string;
  full_name: string;
  phone: string | null;
}

export interface ShopperAccount extends Shopper {
  loyalty_points: number;
  pending_loyalty_points: number;
}

export interface AppliedPromotion {
  promotion_id: string;
  name: string;
  kind: string;
  discount_minor: number;
}

export interface CheckoutQuote {
  currency: string;
  subtotal_minor: number;
  discount_minor: number;
  shipping_minor: number;
  tax_minor: number;
  total_minor: number;
  promotions: AppliedPromotion[];
  coupon_error: string | null;
  points_redeemed: number;
  points_value_minor: number;
}

export interface OrderLine {
  product_id: string;
  product_name: string;
  sku: string | null;
  quantity: string;
  unit_price_minor: number;
  discount_minor: number;
  tax_minor: number;
  total_minor: number;
}

export interface OrderEvent {
  status: string;
  note: string | null;
  created_at: string;
}

export interface OrderPayment {
  id: string;
  method: string;
  status: string;
  amount_minor: number;
  gateway_ref: string | null;
  redirect_url: string | null;
  failure_reason: string | null;
}

export interface Shipment {
  courier: string;
  tracking_number: string;
  label_url: string;
  status: string;
}

export interface Order {
  id: string;
  order_number: string;
  invoice_number: string;
  receipt_number: string | null;
  email: string;
  full_name: string;
  status: string;
  payment_method: string;
  payment_expires_at: string | null;
  currency: string;
  subtotal_minor: number;
  discount_minor: number;
  shipping_minor: number;
  tax_minor: number;
  total_minor: number;
  created_at?: string;
}

export interface OrderView extends Order {
  lines: OrderLine[];
  events: OrderEvent[];
  payments: OrderPayment[];
  shipment: Shipment | null;
}

export interface PaymentStart {
  id: string;
  method: string;
  status: string;
  amount_minor: number;
  redirect_url: string | null;
  instructions: string | null;
  expires_at: string | null;
}

export interface PlacedOrder {
  order: OrderView;
  payment: PaymentStart;
  access_token?: string;
}

export interface LoyaltyAccount {
  enabled: boolean;
  available_points: number;
  pending_points: number;
  point_value_minor: number;
}

export interface WishlistItem {
  product_id: string;
  name: string;
  added_at: string;
}
