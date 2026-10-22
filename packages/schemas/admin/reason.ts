import { z } from 'zod';

export const reasonFormSchema = z.object({
  reason: z.string(),
});

export type ReasonFormValues = z.infer<typeof reasonFormSchema>;
