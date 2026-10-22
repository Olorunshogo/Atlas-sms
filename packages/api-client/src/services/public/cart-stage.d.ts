export interface StagedCartItem {
  id: string;
  name: string;
  price: number;
  unit: string;
  image_url: string | null;
  qty: number;
}
export declare function stageCart(items: StagedCartItem[]): Promise<{
  token: string;
}>;
/**
 * Single-use: the backend deletes the record on read, so a stale or replayed link cannot resurrect an old cart.
 * Resolves to an empty array (not an error) when the token has already expired or been consumed, so a caller can fall back to a normal sign-in without a broken cart blocking it.
 */
export declare function getStagedCart(token: string): Promise<StagedCartItem[]>;
