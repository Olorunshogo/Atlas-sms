import { z } from 'zod';

export const KYC_ID_TYPES = ['NIN', 'Passport', 'Drivers License'] as const;

/* Mirrors submitKycSchema on the reference backend (submit-kyc.dto.ts); bankCode is required here (not on the backend, kept optional there for older multipart clients) because without it every later payout request is rejected. */
export const submitKycSchema = z.object({
  idType: z.enum(KYC_ID_TYPES),
  bankName: z.string().min(2, 'Bank name is required'),
  bankCode: z.string().regex(/^\d{3,10}$/, 'Please select your bank'),
  bankAccountNumber: z
    .string()
    .length(10, 'Account number must be 10 digits')
    .regex(/^\d+$/, 'Account number must be 10 digits'),
  bankAccountName: z.string().min(2, 'Account name is required'),
  idFront: z.instanceof(File, {
    message: 'Please upload a photo of your ID (front side).',
  }),
  idSelfie: z.instanceof(File, {
    message: 'Please upload a selfie holding your ID.',
  }),
});

export type SubmitKycValues = z.infer<typeof submitKycSchema>;
