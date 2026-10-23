import { z } from 'zod';
import { createRequiredString } from '../generics';

export const promotionScopes = ['global', 'zone', 'first_order'] as const;
export type PromotionScope = (typeof promotionScopes)[number];

/*
 * Admin delivery-zone form values.
 * `distanceKm` stays a string (what NumberInputField yields); parse to an integer at submit.
 * `areas` is a comma-separated string until the submit boundary splits it.
 */
export const zoneFormSchema = z.object({
  name: createRequiredString('Zone name', { min: 2, max: 100 }),
  description: z.string(),
  areas: z.string(),
  distanceKm: z
    .string()
    .min(1, 'Distance is required')
    .refine((value) => {
      const parsed = Number(value);
      return Number.isInteger(parsed) && parsed >= 0;
    }, 'Enter a whole number of kilometres'),
  freeDelivery: z.boolean(),
  isActive: z.boolean(),
});

export type ZoneFormValues = z.infer<typeof zoneFormSchema>;

/*
 * Admin free-delivery campaign form.
 * Dates are YYYY-MM-DD; the submit boundary turns them into local-midnight ISO bounds.
 * `zoneId` is required only when scope is "zone".
 */
export const promotionFormSchema = z
  .object({
    name: createRequiredString('Campaign name', { min: 2, max: 100 }),
    scope: z.enum(promotionScopes),
    zoneId: z.string(),
    startsOn: z.string().min(1, 'Start date is required'),
    endsOn: z.string().min(1, 'End date is required'),
  })
  .superRefine((values, ctx) => {
    if (values.scope === 'zone' && !values.zoneId) {
      ctx.addIssue({
        code: 'custom',
        path: ['zoneId'],
        message: 'A zone-scoped campaign needs a zone.',
      });
    }

    if (values.startsOn && values.endsOn) {
      const startsAt = new Date(`${values.startsOn}T00:00:00`);
      const endsBoundary = new Date(`${values.endsOn}T00:00:00`);
      endsBoundary.setDate(endsBoundary.getDate() + 1);
      if (endsBoundary.getTime() <= startsAt.getTime()) {
        ctx.addIssue({
          code: 'custom',
          path: ['endsOn'],
          message: 'The end date must be on or after the start date.',
        });
      }
    }
  });

export type PromotionFormValues = z.infer<typeof promotionFormSchema>;

export const pauseScopes = ['global', 'zone'] as const;
export type PauseScope = (typeof pauseScopes)[number];

/*
 * Admin delivery-pause form. Same shape as the promotion form minus
 * "first_order", which doesn't apply to pausing - a pause blocks an LGA or
 * the whole platform, not one buyer.
 */
export const pauseFormSchema = z
  .object({
    reason: createRequiredString('Reason', { min: 2, max: 200 }),
    scope: z.enum(pauseScopes),
    zoneId: z.string(),
    startsOn: z.string().min(1, 'Start date is required'),
    endsOn: z.string().min(1, 'End date is required'),
  })
  .superRefine((values, ctx) => {
    if (values.scope === 'zone' && !values.zoneId) {
      ctx.addIssue({
        code: 'custom',
        path: ['zoneId'],
        message: 'A zone-scoped pause needs a zone.',
      });
    }

    if (values.startsOn && values.endsOn) {
      const startsAt = new Date(`${values.startsOn}T00:00:00`);
      const endsBoundary = new Date(`${values.endsOn}T00:00:00`);
      endsBoundary.setDate(endsBoundary.getDate() + 1);
      if (endsBoundary.getTime() <= startsAt.getTime()) {
        ctx.addIssue({
          code: 'custom',
          path: ['endsOn'],
          message: 'The end date must be on or after the start date.',
        });
      }
    }
  });

export type PauseFormValues = z.infer<typeof pauseFormSchema>;

/* Admin-editable defaults for the two settings-backed distance fee figures. Naira in, kobo out at the submit boundary. */
export const deliveryRateFormSchema = z.object({
  baseFeeNaira: z
    .number({ message: 'Base fee must be a valid amount.' })
    .min(0, 'Base fee must be a valid amount.'),
  ratePerKmNaira: z
    .number({ message: 'Rate per km must be a valid amount.' })
    .min(0, 'Rate per km must be a valid amount.'),
});

export type DeliveryRateFormValues = z.infer<typeof deliveryRateFormSchema>;
