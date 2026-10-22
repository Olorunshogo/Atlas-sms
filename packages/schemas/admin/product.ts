import { z } from 'zod';
import { createRequiredString } from '../generics';

export const measureUnits = ['kg', 'litre', 'piece', 'measure'] as const;
export type MeasureUnit = (typeof measureUnits)[number];

/*
 * Admin catalogue form values.
 * `price` is naira as a string (what NumberInputField yields); conversion to
 * price_kobo happens at the submit boundary.
 * Optional numeric fields stay strings so blank means "not set" rather than 0.
 */
export const createProductFormSchema = z.object({
  name: createRequiredString('Product name', { max: 100 }),
  unit: createRequiredString('Unit', { max: 50 }),
  price: z
    .string()
    .min(1, 'Price is required')
    .refine((value) => {
      const parsed = parseFloat(value);
      return !Number.isNaN(parsed) && parsed > 0;
    }, 'Enter a valid price'),
  description: z
    .string()
    .max(500, 'Description must be at most 500 characters'),
  categoryId: z.string(),
  measureValue: z.string(),
  measureUnit: z.enum(measureUnits),
  weightGrams: z.string(),
});

export type CreateProductFormValues = z.infer<typeof createProductFormSchema>;
