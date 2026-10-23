import { z } from 'zod';
import { createRequiredStringWithOptions } from '../field-factories';

export const inviteVerificationSchema = z.object({
  inviteCode: createRequiredStringWithOptions('Invite code'),
});

export type InviteVerificationValues = z.infer<typeof inviteVerificationSchema>;
