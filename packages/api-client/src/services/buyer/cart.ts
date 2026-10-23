import { apiFetch } from '../../apiFetch';

/*
 * Server-side cart.
 * localStorage stays the source of truth while browsing; these keep the account copy in step so the cart follows the buyer to another device.
 */

export interface ServerCartLine {
  product_id: number;
  quantity: number;
  name: string;
  unit: string;
  price_kobo: number;
  image_url: string | null;
}

export interface CartSyncLine {
  product_id: number;
  quantity: number;
}

export function getServerCart(): Promise<ServerCartLine[]> {
  return apiFetch<ServerCartLine[]>('/buyer/cart');
}

/** Full replace - idempotent, so a retried debounce is harmless. */
export function syncServerCart(items: CartSyncLine[]): Promise<unknown> {
  return apiFetch('/buyer/cart', {
    method: 'PUT',
    body: JSON.stringify({ items }),
  });
}

export function clearServerCart(): Promise<unknown> {
  return apiFetch('/buyer/cart', { method: 'DELETE' });
}

/*
 * Merge on login.
 * Takes the higher quantity per product rather than summing, so adding the same item on two devices does not double the order.
 */
export function mergeServerCart(
  items: CartSyncLine[],
): Promise<ServerCartLine[]> {
  return apiFetch<ServerCartLine[]>('/buyer/cart/merge', {
    method: 'POST',
    body: JSON.stringify({ items }),
  });
}
