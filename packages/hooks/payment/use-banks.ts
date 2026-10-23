import { useState, useCallback } from 'react';
import { usePaymentAdapter } from './payment-adapter';
import type { Bank, RolePaymentConfig } from '../../types/payment-config';

/*
 * The bank list is a large, slow upstream call, so it only fetches when a caller actually calls `load` - never on mount, and never more than once unless the caller forces it.
 */

export interface UseBanksOptions {
  config: RolePaymentConfig;
}

export interface UseBanksResult {
  banks: Bank[];
  isLoading: boolean;
  error: string | null;
  hasLoaded: boolean;
  load: (force?: boolean) => Promise<void>;
}

export function useBanks(options: UseBanksOptions): UseBanksResult {
  const { config } = options;
  const adapter = usePaymentAdapter();

  const [banks, setBanks] = useState<Bank[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [hasLoaded, setHasLoaded] = useState<boolean>(false);

  const load = useCallback(
    async (force = false): Promise<void> => {
      if ((hasLoaded && !force) || isLoading) return;
      setIsLoading(true);
      setError(null);
      try {
        const raw = await adapter.getBanks(config.endpoints.banks);
        setBanks(config.mapBanks(raw));
        setHasLoaded(true);
      } catch (err) {
        setError(
          err instanceof Error
            ? err.message
            : 'Could not load the bank list. Please try again.',
        );
      } finally {
        setIsLoading(false);
      }
    },
    [adapter, config, hasLoaded, isLoading],
  );

  return { banks, isLoading, error, hasLoaded, load };
}
