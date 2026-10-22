import { z } from 'zod';

export const bankDetailsSchema = z.object({
  bankCode: z.string().min(1, 'Select your bank'),
  accountNumber: z
    .string()
    .length(10, 'Account number must be 10 digits')
    .regex(/^\d+$/, 'Account number must be 10 digits'),
});

export type BankDetailsValues = z.infer<typeof bankDetailsSchema>;
