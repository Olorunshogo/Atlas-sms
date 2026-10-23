import { z } from 'zod';
import { createRequiredStringWithOptions } from '../field-factories';
import { techStackKeys } from '@atlas/data';

/*
 * Admin job-posting form for the reference source's own careers page.
 * Prune candidate.
 */
export const employmentTypeValues = [
  'full_time',
  'part_time',
  'contract',
  'agent',
] as const;

export type EmploymentTypeValue = (typeof employmentTypeValues)[number];

export const employmentTypeOptions: {
  value: EmploymentTypeValue;
  label: string;
}[] = [
  { value: 'full_time', label: 'Full time' },
  { value: 'part_time', label: 'Part time' },
  { value: 'contract', label: 'Contract' },
  { value: 'agent', label: 'Agent' },
];

const optionalYearsString = z.string().refine((value) => {
  if (value.trim() === '') return true;
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed >= 0 && parsed <= 50;
}, 'Enter a whole number of years between 0 and 50');

export const createJobFormSchema = z
  .object({
    title: createRequiredStringWithOptions('Title', { min: 2, max: 200 }),
    department: createRequiredStringWithOptions('Department', {
      min: 1,
      max: 120,
    }),
    location: createRequiredStringWithOptions('Location', { min: 1, max: 200 }),
    description: createRequiredStringWithOptions('Description', { min: 10 }),
    requirements: createRequiredStringWithOptions('Requirements', { min: 10 }),
    employment_type: z.enum(employmentTypeValues),
    experience_min_years: optionalYearsString,
    experience_max_years: optionalYearsString,
    tech_stack: z.array(z.enum(techStackKeys)).max(12),
  })
  .superRefine((values, ctx) => {
    const minRaw = values.experience_min_years.trim();
    const maxRaw = values.experience_max_years.trim();
    if (minRaw === '' || maxRaw === '') return;
    if (Number(minRaw) > Number(maxRaw)) {
      ctx.addIssue({
        code: 'custom',
        path: ['experience_max_years'],
        message: 'Max years must be at least min years',
      });
    }
  });

export type CreateJobFormValues = z.infer<typeof createJobFormSchema>;

export function parseOptionalYears(raw: string): number | null {
  const trimmed = raw.trim();
  if (trimmed === '') return null;
  return Number(trimmed);
}
