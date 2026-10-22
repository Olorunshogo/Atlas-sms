import { useState, useCallback } from 'react';
import { usePaymentAdapter } from './payment-adapter';
import type {
  RolePaymentConfig,
  WithdrawalResult,
} from '../../types/payment-config';

/*
 * Submits a payout request.
 * The caller supplies the max allowed amount at the form layer (see AgentRequestPayoutDialog's use of createWithdrawalSchema) - this hook only owns the submit call and its result.
 */

export interface UseWithdrawOptions {
  config: RolePaymentConfig;
}

export interface UseWithdrawResult {
  submit: (
    amountKobo: number,
    reason?: string,
  ) => Promise<WithdrawalResult | null>;
  result: WithdrawalResult | null;
  isSubmitting: boolean;
  apiError: string | null;
  clearApiError: () => void;
}

export function useWithdraw(options: UseWithdrawOptions): UseWithdrawResult {
  const { config } = options;
  const adapter = usePaymentAdapter();

  const [result, setResult] = useState<WithdrawalResult | null>(null);
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [apiError, setApiError] = useState<string | null>(null);

  const clearApiError = useCallback((): void => {
    setApiError(null);
  }, []);

  const submit = useCallback(
    async (
      amountKobo: number,
      reason?: string,
    ): Promise<WithdrawalResult | null> => {
      setIsSubmitting(true);
      setApiError(null);
      try {
        const raw = await adapter.submitWithdrawal(
          config.endpoints.withdraw,
          amountKobo,
          reason,
        );
        const data = config.mapWithdrawal(raw);
        setResult(data);
        return data;
      } catch (err) {
        setApiError(
          err instanceof Error
            ? err.message
            : 'Could not submit your withdrawal. Please try again.',
        );
        return null;
      } finally {
        setIsSubmitting(false);
      }
    },
    [adapter, config],
  );

  return { submit, result, isSubmitting, apiError, clearApiError };
}
