// Newsletter signup from the reference source's own marketing site, not Atlas's domain. Prune candidate.

import { z } from 'zod';
import { createEmailSchemaWithOptions } from '../field-factories';

/* Mirrors subscribeNewsletterSchema on the reference backend (subscribe-newsletter.dto.ts). */
export const newsletterSubscribeSchema = z.object({
  email: createEmailSchemaWithOptions(),
});

export type NewsletterSubscribeValues = z.infer<
  typeof newsletterSubscribeSchema
>;
