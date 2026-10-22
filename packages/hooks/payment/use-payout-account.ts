import { useState, useCallback, useEffect } from 'react';
import { usePaymentAdapter } from './payment-adapter';
import type {
  PayoutAccount,
  RolePaymentConfig,
} from '../../types/payment-config';

/*
 * Reads and sets the bank account payouts go to.
 * `account` is null when nothing has been set yet, which the caller shows as an empty state rather than an error.
 */

export interface UsePayoutAccountOptions {
  config: RolePaymentConfig;
}

export interface UsePayoutAccountResult {
  account: PayoutAccount | null;
  isLoading: boolean;
  error: string | null;
  refetch: () => Promise<void>;
  save: (
    bankCode: string,
    accountNumber: string,
  ) => Promise<PayoutAccount | null>;
  isSaving: boolean;
  saveError: string | null;
  clearSaveError: () => void;
}

export function usePayoutAccount(
  options: UsePayoutAccountOptions,
): UsePayoutAccountResult {
  const { config } = options;
  const adapter = usePaymentAdapter();

  const [account, setAccount] = useState<PayoutAccount | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  const clearSaveError = useCallback((): void => {
    setSaveError(null);
  }, []);

  const fetchAccount = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);
    try {
      const raw = await adapter.getPayoutAccount(
        config.endpoints.payoutAccount,
      );
      setAccount(config.mapPayoutAccount(raw));
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : 'Could not load your payout account. Please try again.',
      );
    } finally {
      setIsLoading(false);
    }
  }, [adapter, config]);

  useEffect(() => {
    fetchAccount();
  }, [fetchAccount]);

  const save = useCallback(
    async (
      bankCode: string,
      accountNumber: string,
    ): Promise<PayoutAccount | null> => {
      setIsSaving(true);
      setSaveError(null);
      try {
        const raw = await adapter.setPayoutAccount(
          config.endpoints.payoutAccount,
          bankCode,
          accountNumber,
        );
        const data = config.mapPayoutAccount(raw);
        setAccount(data);
        return data;
      } catch (err) {
        setSaveError(
          err instanceof Error
            ? err.message
            : 'Could not save your payout account. Please try again.',
        );
        return null;
      } finally {
        setIsSaving(false);
      }
    },
    [adapter, config],
  );

  return {
    account,
    isLoading,
    error,
    refetch: fetchAccount,
    save,
    isSaving,
    saveError,
    clearSaveError,
  };
}
