import { z } from 'zod';
import {
  createCvSchema,
  createNameSchema,
  createPhoneSchemaWithOptions,
  createWordCountSchema,
} from '../field-factories';

/*
 * A careers/job-application form from the reference source. Atlas has no careers
 * feature. Prune candidate.
 */
export const COVER_LETTER_MIN_WORDS = 50;
export const COVER_LETTER_MAX_WORDS = 1000;

export const applyJobSchema = z.object({
  fullName: createNameSchema(),
  phone: createPhoneSchemaWithOptions(),
  coverLetter: createWordCountSchema('Cover letter', {
    min: COVER_LETTER_MIN_WORDS,
    max: COVER_LETTER_MAX_WORDS,
  }),
  cv: createCvSchema(),
});

export type ApplyJobValues = z.infer<typeof applyJobSchema>;
