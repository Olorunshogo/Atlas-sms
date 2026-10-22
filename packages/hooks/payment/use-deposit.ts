import { useState, useCallback } from 'react';
import { usePaymentAdapter } from './payment-adapter';
import type {
  DepositConfirmation,
  DepositInitiation,
  RolePaymentConfig,
} from '../../types/payment-config';

/*
 * Two-step deposit flow: initiate returns the Paystack authorization URL to redirect the buyer to, and confirm reconciles the wallet once they return.
 * Both steps share one submitting/error state since a caller only ever runs one of them at a time.
 */

export interface UseDepositOptions {
  config: RolePaymentConfig;
}

export interface UseDepositResult {
  initiate: (amountKobo: number) => Promise<DepositInitiation | null>;
  confirm: (reference: string) => Promise<DepositConfirmation | null>;
  initiation: DepositInitiation | null;
  confirmation: DepositConfirmation | null;
  isSubmitting: boolean;
  apiError: string | null;
  clearApiError: () => void;
}

export function useDeposit(options: UseDepositOptions): UseDepositResult {
  const { config } = options;
  const adapter = usePaymentAdapter();

  const [initiation, setInitiation] = useState<DepositInitiation | null>(null);
  const [confirmation, setConfirmation] = useState<DepositConfirmation | null>(
    null,
  );
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [apiError, setApiError] = useState<string | null>(null);

  const clearApiError = useCallback((): void => {
    setApiError(null);
  }, []);

  const initiate = useCallback(
    async (amountKobo: number): Promise<DepositInitiation | null> => {
      setIsSubmitting(true);
      setApiError(null);
      try {
        const raw = await adapter.initiateDeposit(
          config.endpoints.deposit,
          amountKobo,
        );
        const data = config.mapDepositInitiation(raw);
        setInitiation(data);
        return data;
      } catch (err) {
        setApiError(
          err instanceof Error
            ? err.message
            : 'Could not start your deposit. Please try again.',
        );
        return null;
      } finally {
        setIsSubmitting(false);
      }
    },
    [adapter, config],
  );

  const confirm = useCallback(
    async (reference: string): Promise<DepositConfirmation | null> => {
      setIsSubmitting(true);
      setApiError(null);
      try {
        const raw = await adapter.confirmDeposit(
          config.endpoints.depositConfirm,
          reference,
        );
        const data = config.mapDepositConfirmation(raw);
        setConfirmation(data);
        return data;
      } catch (err) {
        setApiError(
          err instanceof Error
            ? err.message
            : 'Could not confirm your deposit. Please try again.',
        );
        return null;
      } finally {
        setIsSubmitting(false);
      }
    },
    [adapter, config],
  );

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
