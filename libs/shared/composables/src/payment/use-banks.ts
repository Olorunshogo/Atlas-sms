import { ref, type Ref } from 'vue';
import { usePaymentAdapter } from './payment-adapter';
import type { Bank, RolePaymentConfig } from '@atlas/types';

/*
 * The bank list is a large, slow upstream call - ported from the reference source's React
 * use-banks.ts. It only fetches when a caller actually calls `load`, never
 * on mount, and never more than once unless the caller forces it.
 */

export interface UseBanksOptions {
  config: RolePaymentConfig;
}

export interface UseBanksResult {
  banks: Ref<Bank[]>;
  isLoading: Ref<boolean>;
  error: Ref<string | null>;
  hasLoaded: Ref<boolean>;
  load: (force?: boolean) => Promise<void>;
}

export function useBanks(options: UseBanksOptions): UseBanksResult {
  const { config } = options;
  const adapter = usePaymentAdapter();

  const banks = ref<Bank[]>([]);
  const isLoading = ref<boolean>(false);
  const error = ref<string | null>(null);
  const hasLoaded = ref<boolean>(false);

  async function load(force = false): Promise<void> {
    if ((hasLoaded.value && !force) || isLoading.value) return;
    isLoading.value = true;
    error.value = null;
    try {
      const raw = await adapter.getBanks(config.endpoints.banks);
      banks.value = config.mapBanks(raw);
      hasLoaded.value = true;
    } catch (err) {
      error.value =
        err instanceof Error
          ? err.message
          : 'Could not load the bank list. Please try again.';
    } finally {
      isLoading.value = false;
    }
  }

  return { banks, isLoading, error, hasLoaded, load };
}
