import type {
  Bank,
  DepositConfirmation,
  DepositInitiation,
  PayoutAccount,
  RolePaymentConfig,
  WalletPage,
  WithdrawalResult,
} from '../../types/payment-config';

/*
 * Buyer's RolePaymentConfig.
 * This is the TABLE half of the payment config seam - it lives here, not in a consuming app, because the buyer endpoints and their raw field names are already known and stable.
 * Agent's config (different paths: /agent/wallet, /agent/banks, /agent/bank-details, /agent/withdrawals, and its own field names) drops in beside this one as AGENT_PAYMENT_CONFIG without any change to the hooks that consume it.
 */

// === Raw response shapes
//
// The backend responds in snake_case; these interfaces exist only to narrow the adapter's `unknown` before mapping to the camelCase domain types.

interface RawWalletSummary {
  id: number;
  available_balance: number;
  pending_balance: number;
  total_deposited: number;
}

interface RawWalletTransaction {
  id: number;
  type: 'deposit' | 'withdraw' | 'refund';
  amount: number;
  status: 'pending' | 'completed' | 'failed';
  reference: string | null;
  description: string | null;
  created_at: string;
}

interface RawWalletPage {
  wallet: RawWalletSummary;
  transactions: RawWalletTransaction[];
  pagination: { page: number; limit: number; total: number };
}

interface RawDepositInitiation {
  transaction_id: number;
  amount_kobo: number;
  authorization_url: string;
  reference: string;
}

interface RawDepositConfirmation {
  wallet_id: number;
  available_balance: number;
  amount_added: number;
}

interface RawWithdrawalResult {
  withdrawal_id: number;
  amount_kobo: number;
  reference: string;
  status: string;
}

interface RawPayoutAccount {
  bank_code: string;
  bank_name: string;
  account_number: string;
  account_name: string;
}

// === Mappers

function mapWallet(raw: unknown): WalletPage {
  const data = raw as RawWalletPage;
  return {
    wallet: {
      id: data.wallet.id,
      availableBalanceKobo: data.wallet.available_balance,
      pendingBalanceKobo: data.wallet.pending_balance,
      totalDepositedKobo: data.wallet.total_deposited,
    },
    transactions: data.transactions.map((transaction) => ({
      id: transaction.id,
      type: transaction.type,
      amountKobo: transaction.amount,
      status: transaction.status,
      reference: transaction.reference,
      description: transaction.description,
      createdAt: transaction.created_at,
    })),
    pagination: data.pagination,
  };
}

function mapDepositInitiation(raw: unknown): DepositInitiation {
  const data = raw as RawDepositInitiation;
  return {
    transactionId: data.transaction_id,
    amountKobo: data.amount_kobo,
    authorizationUrl: data.authorization_url,
    reference: data.reference,
  };
}

function mapDepositConfirmation(raw: unknown): DepositConfirmation {
  const data = raw as RawDepositConfirmation;
  return {
    walletId: data.wallet_id,
    availableBalanceKobo: data.available_balance,
    amountAddedKobo: data.amount_added,
  };
}

function mapWithdrawal(raw: unknown): WithdrawalResult {
  const data = raw as RawWithdrawalResult;
  return {
    withdrawalId: data.withdrawal_id,
    amountKobo: data.amount_kobo,
    reference: data.reference,
    status: data.status,
  };
}

function mapBanks(raw: unknown): Bank[] {
  return raw as Bank[];
}

function mapPayoutAccount(raw: unknown): PayoutAccount | null {
  if (!raw) return null;
  const data = raw as RawPayoutAccount;
  return {
    bankCode: data.bank_code,
    bankName: data.bank_name,
    accountNumber: data.account_number,
    accountName: data.account_name,
  };
}

export const BUYER_PAYMENT_CONFIG: RolePaymentConfig = {
  role: 'buyer',
  endpoints: {
    wallet: '/buyer/wallet',
    deposit: '/buyer/wallet/deposit',
    depositConfirm: '/buyer/wallet/deposit/confirm',
    withdraw: '/buyer/wallet/withdraw',
    banks: '/buyer/wallet/banks',
    payoutAccount: '/buyer/wallet/payout-account',
  },
  mapWallet,
  mapDepositInitiation,
  mapDepositConfirmation,
  mapWithdrawal,
  mapBanks,
  mapPayoutAccount,
};
