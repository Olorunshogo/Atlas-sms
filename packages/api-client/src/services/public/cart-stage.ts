import { publicRequest } from '../../transport/public-request';

/*
 * The cross-subdomain checkout handoff: the marketing app has no buyer session to attach a cart to, so it stages the guest cart here before redirecting to buyer.example.com for login/signup, and hands the returned token along as a query param.
 * No auth required on either call.
 */

export interface StagedCartItem {
  id: string;
  name: string;
  price: number;
  unit: string;
  image_url: string | null;
  qty: number;
}

export function stageCart(items: StagedCartItem[]): Promise<{ token: string }> {
  return publicRequest<{ token: string }>('/cart/stage', {
    method: 'POST',
    body: JSON.stringify({ items }),
  });
}

/**
 * Single-use: the backend deletes the record on read, so a stale or replayed link cannot resurrect an old cart.
 * Resolves to an empty array (not an error) when the token has already expired or been consumed, so a caller can fall back to a normal sign-in without a broken cart blocking it.
 */
export async function getStagedCart(token: string): Promise<StagedCartItem[]> {
  try {
    return await publicRequest<StagedCartItem[]>(
      `/cart/stage/${encodeURIComponent(token)}`,
    );
  } catch {
    return [];
  }
}
