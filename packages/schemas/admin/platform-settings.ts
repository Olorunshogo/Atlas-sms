import { z } from 'zod';

export const platformSettingsSchema = z.object({
  commissionRate: z
    .number({ message: 'Commission rate must be between 1 and 100.' })
    .min(1, 'Commission rate must be between 1 and 100.')
    .max(100, 'Commission rate must be between 1 and 100.'),
  discountNaira: z
    .number({ message: 'Referral discount must be a valid amount.' })
    .min(0, 'Referral discount must be a valid amount.'),
});

export type PlatformSettingsValues = z.infer<typeof platformSettingsSchema>;
