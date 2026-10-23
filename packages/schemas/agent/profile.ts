import { z } from 'zod';
import { createNameSchema, createPhoneSchema } from '../generics';

/*
 * Mirrors updateAgentProfileSchema on the reference backend (update-agent-profile.dto.ts).
 * Phone may be cleared to an empty string; anything non-empty must pass the shared Nigerian rule.
 */
export const agentProfileSchema = z.object({
  firstName: createNameSchema('First name'),
  lastName: createNameSchema('Last name'),
  phone: z.union([createPhoneSchema(), z.literal('')]),
  address: z.string().max(500),
  state: z.string().max(100),
  lga: z.string().max(100),
});

export type AgentProfileValues = z.infer<typeof agentProfileSchema>;
