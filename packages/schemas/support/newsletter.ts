import { z } from 'zod';
import { createEmailSchema } from '../generics';

/* Mirrors subscribeNewsletterSchema on the reference backend (subscribe-newsletter.dto.ts). */
export const newsletterSubscribeSchema = z.object({
  email: createEmailSchema(),
});

export type NewsletterSubscribeValues = z.infer<
  typeof newsletterSubscribeSchema
>;
