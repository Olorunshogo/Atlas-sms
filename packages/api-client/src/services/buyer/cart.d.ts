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
export declare function getServerCart(): Promise<ServerCartLine[]>;
/** Full replace - idempotent, so a retried debounce is harmless. */
export declare function syncServerCart(items: CartSyncLine[]): Promise<unknown>;
export declare function clearServerCart(): Promise<unknown>;
export declare function mergeServerCart(
  items: CartSyncLine[],
): Promise<ServerCartLine[]>;
