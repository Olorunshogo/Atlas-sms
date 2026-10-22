import { inject, provide, type InjectionKey } from 'vue';

/*
 * Dependency injection boundary for the shared payment composables, mirroring
 * ../auth/auth-adapter.ts - ported from the reference source's React payment-adapter.tsx.
 *
 * NOTE: this whole payment/ subtree (wallet, deposit, withdrawal, payout
 * account, banks) is that source's buyer/agent marketplace-wallet domain. Atlas
 * SMS has no equivalent concept yet - verify with the actual Atlas FRs
 * before wiring this into either app; it may turn out to be dead weight.
 */

export interface PaymentAdapter {
  getWallet: (
    endpoint: string,
    page: number,
    limit: number,
  ) => Promise<unknown>;
  initiateDeposit: (endpoint: string, amountKobo: number) => Promise<unknown>;
  confirmDeposit: (endpoint: string, reference: string) => Promise<unknown>;
  submitWithdrawal: (
    endpoint: string,
    amountKobo: number,
    reason?: string,
  ) => Promise<unknown>;
  getBanks: (endpoint: string) => Promise<unknown>;
  getPayoutAccount: (endpoint: string) => Promise<unknown>;
  setPayoutAccount: (
    endpoint: string,
    bankCode: string,
    accountNumber: string,
  ) => Promise<unknown>;
}

const PAYMENT_ADAPTER_KEY: InjectionKey<PaymentAdapter> = Symbol(
  'AtlasPaymentAdapter',
);

export function providePaymentAdapter(adapter: PaymentAdapter): void {
  provide(PAYMENT_ADAPTER_KEY, adapter);
}

export function usePaymentAdapter(): PaymentAdapter {
  const adapter = inject(PAYMENT_ADAPTER_KEY);
  if (!adapter) {
    throw new Error(
      'Payment composables require providePaymentAdapter() to run higher up the component tree.',
    );
  }
  return adapter;
}
