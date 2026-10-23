import { z } from 'zod';
import {
  createCvSchema,
  createNameSchema,
  createPhoneSchema,
  createWordCountSchema,
} from '../generics';

/*
 * Mirrors applyJobSchema on the reference backend (application.dto.ts).
 * Email is not here: applying needs a signed-in account, and the backend takes it from that.
 * The name is, because an applicant may want a different name on an application than on their account.
 */
export const COVER_LETTER_MIN_WORDS = 50;
export const COVER_LETTER_MAX_WORDS = 1000;

export const applyJobSchema = z.object({
  fullName: createNameSchema(),
  phone: createPhoneSchema(),
  coverLetter: createWordCountSchema('Cover letter', {
    min: COVER_LETTER_MIN_WORDS,
    max: COVER_LETTER_MAX_WORDS,
  }),
  cv: createCvSchema(),
});

export type ApplyJobValues = z.infer<typeof applyJobSchema>;
