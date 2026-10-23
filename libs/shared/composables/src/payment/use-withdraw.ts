import { ref, type Ref } from 'vue';
import { usePaymentAdapter } from './payment-adapter';
import type { RolePaymentConfig, WithdrawalResult } from '@atlas/types';

/*
 * Submits a payout request - ported from the reference source's React use-withdraw.ts.
 * The caller supplies the max allowed amount at the form layer; this
 * composable only owns the submit call and its result.
 */

export interface UseWithdrawOptions {
  config: RolePaymentConfig;
}

export interface UseWithdrawResult {
  submit: (
    amountKobo: number,
    reason?: string,
  ) => Promise<WithdrawalResult | null>;
  result: Ref<WithdrawalResult | null>;
  isSubmitting: Ref<boolean>;
  apiError: Ref<string | null>;
  clearApiError: () => void;
}

export function useWithdraw(options: UseWithdrawOptions): UseWithdrawResult {
  const { config } = options;
  const adapter = usePaymentAdapter();

  const result = ref<WithdrawalResult | null>(null);
  const isSubmitting = ref<boolean>(false);
  const apiError = ref<string | null>(null);

  function clearApiError(): void {
    apiError.value = null;
  }

  async function submit(
    amountKobo: number,
    reason?: string,
  ): Promise<WithdrawalResult | null> {
    isSubmitting.value = true;
    apiError.value = null;
    try {
      const raw = await adapter.submitWithdrawal(
        config.endpoints.withdraw,
        amountKobo,
        reason,
      );
      const data = config.mapWithdrawal(raw);
      result.value = data;
      return data;
    } catch (err) {
      apiError.value =
        err instanceof Error
          ? err.message
          : 'Could not submit your withdrawal. Please try again.';
      return null;
    } finally {
      isSubmitting.value = false;
    }
  }

  return { submit, result, isSubmitting, apiError, clearApiError };
}
