import { z } from 'zod';

/* Mirrors closePip on the backend; outcome notes are optional. */
export const closePipSchema = z.object({
  status: z.enum(['passed', 'failed'], { message: 'Pick an outcome' }),
  outcomeNotes: z.string(),
});

export type ClosePipValues = z.infer<typeof closePipSchema>;
