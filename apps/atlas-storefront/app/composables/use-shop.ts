import { normalizeBody, type ApiBody } from '@atlas/api';
import type { CartView, ShopInfo, ShopperAccount } from '../utils/types';

interface ShopRequestInit {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  body?: unknown;
  query?: Record<string, string | number | boolean | undefined>;
}

/**
 * Every page talks to the shop through the cookie proxy at `/api/shop`. The fetcher is taken during setup,
 * so a server render forwards the visitor's cookies.
 */
export function useShop() {
  const requestFetch = useRequestFetch();
  const info = useState<ShopInfo | null>('shop-info', () => null);
  const account = useState<ShopperAccount | null>('shop-account', () => null);
  const accountLoaded = useState<boolean>('shop-account-loaded', () => false);
  const cartCount = useState<number>('shop-cart-count', () => 0);

  async function call<T>(
    path: string,
    init: ShopRequestInit = {},
  ): Promise<ApiBody<T>> {
    try {
      const payload: unknown = await requestFetch(`/api/shop${path}`, {
        method: init.method ?? 'GET',
        body: init.body as Record<string, unknown> | undefined,
        query: init.query,
        ignoreResponseError: true,
      });
      return normalizeBody<T>(payload);
    } catch {
      return { success: false, message: 'The shop is not reachable.' };
    }
  }

  async function loadInfo(): Promise<ShopInfo | null> {
    if (info.value) {
      return info.value;
    }
    const result = await call<ShopInfo>('');
    info.value = result.success ? result.data : null;
    return info.value;
  }

  async function loadAccount(): Promise<void> {
    const result = await call<ShopperAccount>('/account');
    account.value = result.success ? result.data : null;
    accountLoaded.value = true;
  }

  async function refreshCartCount(): Promise<void> {
    const result = await call<CartView>('/cart');
    cartCount.value = result.success ? result.data.items.length : 0;
  }

  async function signOut(): Promise<void> {
    await requestFetch('/api/session/logout', { method: 'POST' });
    account.value = null;
    await refreshCartCount();
  }

  return {
    call,
    info,
    account,
    accountLoaded,
    cartCount,
    loadInfo,
    loadAccount,
    refreshCartCount,
    signOut,
  };
}
