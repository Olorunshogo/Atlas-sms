import { useState, useCallback, useEffect } from 'react';
import { usePaymentAdapter } from './payment-adapter';
import type {
  RolePaymentConfig,
  WalletPagination,
  WalletSummary,
  WalletTransaction,
} from '../../types/payment-config';

/*
 * Balance and paginated transaction history for any role.
 * The endpoint and response mapping both come from config, so this hook never branches on role - see RolePaymentConfig.
 */

export interface UseWalletOptions {
  config: RolePaymentConfig;
  initialPage?: number;
  limit?: number;
}

export interface UseWalletResult {
  wallet: WalletSummary | null;
  transactions: WalletTransaction[];
  pagination: WalletPagination | null;
  page: number;
  setPage: (page: number) => void;
  isLoading: boolean;
  error: string | null;
  refetch: () => Promise<void>;
}

export function useWallet(options: UseWalletOptions): UseWalletResult {
  const { config, initialPage = 1, limit = 20 } = options;
  const adapter = usePaymentAdapter();

  const [wallet, setWallet] = useState<WalletSummary | null>(null);
  const [transactions, setTransactions] = useState<WalletTransaction[]>([]);
  const [pagination, setPagination] = useState<WalletPagination | null>(null);
  const [page, setPage] = useState<number>(initialPage);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const fetchWallet = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);
    try {
      const raw = await adapter.getWallet(config.endpoints.wallet, page, limit);
      const data = config.mapWallet(raw);
      setWallet(data.wallet);
      setTransactions(data.transactions);
      setPagination(data.pagination);
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : 'Could not load your wallet. Please try again.',
      );
    } finally {
      setIsLoading(false);
    }
  }, [adapter, config, page, limit]);

  useEffect(() => {
    fetchWallet();
  }, [fetchWallet]);

  return {
    wallet,
    transactions,
    pagination,
    page,
    setPage,
    isLoading,
    error,
    refetch: fetchWallet,
  };
}
