import { z } from 'zod';
import { createRequiredString } from '../generics';

export const inviteVerificationSchema = z.object({
  inviteCode: createRequiredString('Invite code'),
});

export type InviteVerificationValues = z.infer<typeof inviteVerificationSchema>;
