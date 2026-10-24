/** Minor units to a display price in the shop's currency, e.g. 150000 NGN -> "₦1,500.00". */
export function formatMoney(minor: number, currency: string): string {
  return new Intl.NumberFormat('en-NG', {
    style: 'currency',
    currency: currency || 'NGN',
  }).format(minor / 100);
}

export function shownPrice(product: {
  price_minor: number;
  sale_price_minor: number | null;
}): number {
  return product.sale_price_minor ?? product.price_minor;
}
