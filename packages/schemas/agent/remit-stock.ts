import { z } from 'zod';
import { formatFromKobo } from '../../utils/format-currency';

export function createRemitStockSchema(outstandingKobo: number) {
  return z.object({
    amount: z
      .number({ message: 'Enter an amount' })
      .positive('Amount must be greater than zero')
      .max(
        outstandingKobo / 100,
        `That is more than the ${formatFromKobo(outstandingKobo)} still owed.`,
      ),
  });
}

export type RemitStockValues = z.infer<
  ReturnType<typeof createRemitStockSchema>
>;
