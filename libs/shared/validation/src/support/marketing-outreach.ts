// Field-outreach lead capture from the reference source's own marketing site, not Atlas's domain. Prune candidate.

import { z } from 'zod';
import {
  createNameSchema,
  createPhoneSchemaWithOptions,
} from '../field-factories';

/*
 * Mirrors webLeadSchema on the reference backend (public.controller.ts).
 * quantity stays a string in the form (NumberInputField) and is coerced to a number at submit.
 * Optional blanks are trimmed to undefined so the payload matches the DTO's optional fields.
 */
export const marketingOutreachSchema = z.object({
  ownerName: createNameSchema(),
  phone: createPhoneSchemaWithOptions(),
  shopName: z.string(),
  lga: z.string(),
  area: z.string(),
  productInterest: z.string(),
  quantity: z.string(),
  howHeard: z.string(),
  notes: z.string(),
});

export type MarketingOutreachValues = z.infer<typeof marketingOutreachSchema>;
