/*
 * From the reference source - a marketplace wallet/deposit/withdraw/payout domain.
 * Atlas SMS has no wallet or payout feature in its requirements (see
 * docs/PDF/Store-Management-System-SRS-v1.0.pdf FR-01 through FR-06) - this
 * is a prune candidate, kept only as reference for the shape if Atlas ever
 * needs a till-reconciliation or supplier-payout ledger.
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
