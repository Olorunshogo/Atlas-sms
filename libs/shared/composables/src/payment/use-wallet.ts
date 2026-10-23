import { onMounted, ref, watch, type Ref } from 'vue';
import { usePaymentAdapter } from './payment-adapter';
import type {
  RolePaymentConfig,
  WalletPagination,
  WalletSummary,
  WalletTransaction,
} from '@atlas/types';

/*
 * Balance and paginated transaction history for any role - ported from
 * the reference source's React use-wallet.ts (useState/useEffect -> ref/watch).
 */

export interface UseWalletOptions {
  config: RolePaymentConfig;
  initialPage?: number;
  limit?: number;
}

export interface UseWalletResult {
  wallet: Ref<WalletSummary | null>;
  transactions: Ref<WalletTransaction[]>;
  pagination: Ref<WalletPagination | null>;
  page: Ref<number>;
  isLoading: Ref<boolean>;
  error: Ref<string | null>;
  refetch: () => Promise<void>;
}

export function useWallet(options: UseWalletOptions): UseWalletResult {
  const { config, initialPage = 1, limit = 20 } = options;
  const adapter = usePaymentAdapter();

  const wallet = ref<WalletSummary | null>(null);
  const transactions = ref<WalletTransaction[]>([]);
  const pagination = ref<WalletPagination | null>(null);
  const page = ref<number>(initialPage);
  const isLoading = ref<boolean>(false);
  const error = ref<string | null>(null);

  async function fetchWallet(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      const raw = await adapter.getWallet(
        config.endpoints.wallet,
        page.value,
        limit,
      );
      const data = config.mapWallet(raw);
      wallet.value = data.wallet;
      transactions.value = data.transactions;
      pagination.value = data.pagination;
    } catch (err) {
      error.value =
        err instanceof Error
          ? err.message
          : 'Could not load your wallet. Please try again.';
    } finally {
      isLoading.value = false;
    }
  }

  watch(page, fetchWallet);
  onMounted(fetchWallet);

  return {
    wallet,
    transactions,
    pagination,
    page,
    isLoading,
    error,
    refetch: fetchWallet,
  };
}
