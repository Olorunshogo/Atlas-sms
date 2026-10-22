import { z } from 'zod';

/* Mirrors DepositDto on the reference backend (wallet.controller.ts). */
export function createFundWalletSchema(minNaira: number) {
  return z.object({
    amount: z
      .number({ message: 'Enter an amount' })
      .min(minNaira, `Minimum amount is ₦${minNaira}`),
  });
}

export type FundWalletValues = z.infer<
  ReturnType<typeof createFundWalletSchema>
>;

/* Mirrors PayoutAccountDto on the backend. */
export const payoutAccountSchema = z.object({
  bankCode: z.string().min(3, 'Select a bank'),
  accountNumber: z
    .string()
    .length(10, 'Account number must be 10 digits')
    .regex(/^\d+$/, 'Account number must be 10 digits'),
});

export type PayoutAccountValues = z.infer<typeof payoutAccountSchema>;

/* Mirrors WithdrawalDto on the backend. */
export function createWithdrawWalletSchema(availableBalanceNaira: number) {
  return z.object({
    amount: z
      .number({ message: 'Enter an amount' })
      .positive('Enter a valid amount')
      .max(availableBalanceNaira, 'Amount exceeds your available balance'),
    reason: z.string(),
  });
}

export type WithdrawWalletValues = z.infer<
  ReturnType<typeof createWithdrawWalletSchema>
>;
