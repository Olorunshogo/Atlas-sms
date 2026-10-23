import { onMounted, ref, type Ref } from 'vue';
import { usePaymentAdapter } from './payment-adapter';
import type { PayoutAccount, RolePaymentConfig } from '@atlas/types';

/*
 * Reads and sets the bank account payouts go to - ported from the reference source's
 * React use-payout-account.ts. `account` is null when nothing has been set
 * yet, which the caller shows as an empty state rather than an error.
 */

export interface UsePayoutAccountOptions {
  config: RolePaymentConfig;
}

export interface UsePayoutAccountResult {
  account: Ref<PayoutAccount | null>;
  isLoading: Ref<boolean>;
  error: Ref<string | null>;
  refetch: () => Promise<void>;
  save: (
    bankCode: string,
    accountNumber: string,
  ) => Promise<PayoutAccount | null>;
  isSaving: Ref<boolean>;
  saveError: Ref<string | null>;
  clearSaveError: () => void;
}

export function usePayoutAccount(
  options: UsePayoutAccountOptions,
): UsePayoutAccountResult {
  const { config } = options;
  const adapter = usePaymentAdapter();

  const account = ref<PayoutAccount | null>(null);
  const isLoading = ref<boolean>(false);
  const error = ref<string | null>(null);
  const isSaving = ref<boolean>(false);
  const saveError = ref<string | null>(null);

  function clearSaveError(): void {
    saveError.value = null;
  }

  async function fetchAccount(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      const raw = await adapter.getPayoutAccount(
        config.endpoints.payoutAccount,
      );
      account.value = config.mapPayoutAccount(raw);
    } catch (err) {
      error.value =
        err instanceof Error
          ? err.message
          : 'Could not load your payout account. Please try again.';
    } finally {
      isLoading.value = false;
    }
  }

  onMounted(fetchAccount);

  async function save(
    bankCode: string,
    accountNumber: string,
  ): Promise<PayoutAccount | null> {
    isSaving.value = true;
    saveError.value = null;
    try {
      const raw = await adapter.setPayoutAccount(
        config.endpoints.payoutAccount,
        bankCode,
        accountNumber,
      );
      const data = config.mapPayoutAccount(raw);
      account.value = data;
      return data;
    } catch (err) {
      saveError.value =
        err instanceof Error
          ? err.message
          : 'Could not save your payout account. Please try again.';
      return null;
    } finally {
      isSaving.value = false;
    }
  }

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
