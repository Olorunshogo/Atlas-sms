import { z } from 'zod';

/*
 * Agent payout request.
 *
 * The amount is entered in naira because that is what an agent thinks in, and
 * converted to kobo at the boundary. The upper bound is the available balance,
 * which the caller supplies - the server re-checks it, since a client-side max
 * is a convenience, never the authority.
 */
export function createWithdrawalSchema(availableNaira: number) {
  return z.object({
    amount: z
      .number({ message: 'Enter an amount' })
      .positive('Amount must be greater than zero')
      .max(
        availableNaira,
        `You can request at most ${availableNaira.toLocaleString('en-NG')}`,
      ),
  });
}

export type WithdrawalValues = z.infer<
  ReturnType<typeof createWithdrawalSchema>
>;
