/*
 * Shape of a role's payment configuration, plus the domain types the shared
 * wallet/deposit/withdraw hooks and the role configs both consume.
 *
 * The TYPES live here because the shared hooks and each role's config both
 * need them. The buyer TABLE (BUYER_PAYMENT_CONFIG) lives with the payment
 * hooks in ./hooks/payment, not here - see that file for why.
 */

// === Wallet

export interface WalletSummary {
  id: number;
  availableBalanceKobo: number;
  pendingBalanceKobo: number;
  totalDepositedKobo: number;
}

export type WalletTransactionType = 'deposit' | 'withdraw' | 'refund';
export type WalletTransactionStatus = 'pending' | 'completed' | 'failed';

export interface WalletTransaction {
  id: number;
  type: WalletTransactionType;
  amountKobo: number;
  status: WalletTransactionStatus;
  reference: string | null;
  description: string | null;
  createdAt: string;
}

export interface WalletPagination {
  page: number;
  limit: number;
  total: number;
}

export interface WalletPage {
  wallet: WalletSummary;
  transactions: WalletTransaction[];
  pagination: WalletPagination;
}

// === Deposit

export interface DepositInitiation {
  transactionId: number;
  amountKobo: number;
  authorizationUrl: string;
  reference: string;
}

export interface DepositConfirmation {
  walletId: number;
  availableBalanceKobo: number;
  amountAddedKobo: number;
}

// === Withdrawal

export interface WithdrawalResult {
  withdrawalId: number;
  amountKobo: number;
  reference: string;
  status: string;
}

// === Payout account

export interface Bank {
  name: string;
  code: string;
  slug: string;
}

export interface PayoutAccount {
  bankCode: string;
  bankName: string;
  accountNumber: string;
  accountName: string;
}

// === Role configuration

export interface RolePaymentConfigEndpoints {
  wallet: string;
  deposit: string;
  depositConfirm: string;
  withdraw: string;
  banks: string;
  payoutAccount: string;
}

/*
 * The `map*` fields below are shape adapters. Each role's raw API response can
 * use its own field names (the agent endpoints do), so mapping from the raw
 * response to the canonical domain types above is config, not hook logic -
 * the same reason RoleSignupConfig carries the signup schema instead of the
 * hook branching on role.
 */
export interface RolePaymentConfig {
  role: string;
  endpoints: RolePaymentConfigEndpoints;
  mapWallet: (raw: unknown) => WalletPage;
  mapDepositInitiation: (raw: unknown) => DepositInitiation;
  mapDepositConfirmation: (raw: unknown) => DepositConfirmation;
  mapWithdrawal: (raw: unknown) => WithdrawalResult;
  mapBanks: (raw: unknown) => Bank[];
  mapPayoutAccount: (raw: unknown) => PayoutAccount | null;
}
