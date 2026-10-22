import { z } from 'zod';

/*
 * `dateOfBirth` is YYYY-MM-DD from DateTimeInputField; the submit boundary
 * turns it into an ISO datetime with offset, which is what the backend's
 * `date_of_birth` field requires. Home address and next of kin are optional,
 * same as the backend DTO.
 */
export const personalDetailsSchema = z.object({
  dateOfBirth: z.string().min(1, 'Date of birth is required'),
  homeAddress: z.string().max(500, 'Keep it under 500 characters'),
  nextOfKin: z.string().max(500, 'Keep it under 500 characters'),
});

export type PersonalDetailsValues = z.infer<typeof personalDetailsSchema>;
