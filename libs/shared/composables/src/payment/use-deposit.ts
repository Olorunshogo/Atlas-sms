import { ref, type Ref } from 'vue';
import { usePaymentAdapter } from './payment-adapter';
import type {
  DepositConfirmation,
  DepositInitiation,
  RolePaymentConfig,
} from '@atlas/types';

/*
 * Two-step deposit flow - ported from the reference source's React use-deposit.ts.
 * Initiate returns the payment-provider authorization URL to redirect to;
 * confirm reconciles the wallet once the payer returns.
 */

export interface UseDepositOptions {
  config: RolePaymentConfig;
}

export interface UseDepositResult {
  initiate: (amountKobo: number) => Promise<DepositInitiation | null>;
  confirm: (reference: string) => Promise<DepositConfirmation | null>;
  initiation: Ref<DepositInitiation | null>;
  confirmation: Ref<DepositConfirmation | null>;
  isSubmitting: Ref<boolean>;
  apiError: Ref<string | null>;
  clearApiError: () => void;
}

export function useDeposit(options: UseDepositOptions): UseDepositResult {
  const { config } = options;
  const adapter = usePaymentAdapter();

  const initiation = ref<DepositInitiation | null>(null);
  const confirmation = ref<DepositConfirmation | null>(null);
  const isSubmitting = ref<boolean>(false);
  const apiError = ref<string | null>(null);

  function clearApiError(): void {
    apiError.value = null;
  }

  async function initiate(
    amountKobo: number,
  ): Promise<DepositInitiation | null> {
    isSubmitting.value = true;
    apiError.value = null;
    try {
      const raw = await adapter.initiateDeposit(
        config.endpoints.deposit,
        amountKobo,
      );
      const data = config.mapDepositInitiation(raw);
      initiation.value = data;
      return data;
    } catch (err) {
      apiError.value =
        err instanceof Error
          ? err.message
          : 'Could not start your deposit. Please try again.';
      return null;
    } finally {
      isSubmitting.value = false;
    }
  }

  async function confirm(
    reference: string,
  ): Promise<DepositConfirmation | null> {
    isSubmitting.value = true;
    apiError.value = null;
    try {
      const raw = await adapter.confirmDeposit(
        config.endpoints.depositConfirm,
        reference,
      );
      const data = config.mapDepositConfirmation(raw);
      confirmation.value = data;
      return data;
    } catch (err) {
      apiError.value =
        err instanceof Error
          ? err.message
          : 'Could not confirm your deposit. Please try again.';
      return null;
    } finally {
      isSubmitting.value = false;
    }
  }

  return {
    initiate,
    confirm,
    initiation,
    confirmation,
    isSubmitting,
    apiError,
    clearApiError,
  };
}
