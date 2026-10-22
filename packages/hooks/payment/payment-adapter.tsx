import { createContext, useContext, type ReactNode } from 'react';

/*
 * Dependency injection boundary for the shared payment hooks, mirroring ../auth/auth-adapter.tsx.
 *
 * This package must never import the API client - doing so would drag transport into a UI package.
 * So the app supplies these once via PaymentAdapterProvider, and the hooks stay pure: form/loading state and calling the adapter with the endpoint each role's config provides.
 *
 * Every method returns Promise<unknown> rather than a typed shape, because the response shape differs per role.
 * The role's RolePaymentConfig maps the raw response to the canonical domain type - see ../../types/payment-config.
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

const PaymentAdapterContext = createContext<PaymentAdapter | null>(null);

export interface PaymentAdapterProviderProps {
  adapter: PaymentAdapter;
  children: ReactNode;
}

export function PaymentAdapterProvider({
  adapter,
  children,
}: PaymentAdapterProviderProps) {
  return (
    <PaymentAdapterContext.Provider value={adapter}>
      {children}
    </PaymentAdapterContext.Provider>
  );
}

export function usePaymentAdapter(): PaymentAdapter {
  const adapter = useContext(PaymentAdapterContext);
  if (!adapter) {
    throw new Error(
      'Payment hooks require <PaymentAdapterProvider>. Wrap your app in it and supply an adapter.',
    );
  }
  return adapter;
}
