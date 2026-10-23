import { z } from 'zod';
import { createNameSchema } from '../field-factories';

/* Mirrors updateProfileSchema on the reference backend (update-profile.dto.ts). */
export const buyerSettingsSchema = z.object({
  userName: createNameSchema('Name'),
  currency: z.string().min(1, 'Select a currency'),
  country: z.string().min(1, 'Select a country'),
  deliveryAddress: z.string(),
  emailNotification: z.boolean(),
  smsNotification: z.boolean(),
  twoFactor: z.boolean(),
});

export type BuyerSettingsValues = z.infer<typeof buyerSettingsSchema>;
