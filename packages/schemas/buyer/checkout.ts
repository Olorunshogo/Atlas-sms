import { z } from 'zod';

/* Mirrors create-order.dto.ts's delivery_address rule (the stricter of the two order-creation endpoints checkout calls) on the backend. */
export const checkoutDeliverySchema = z.object({
  stateName: z.string().min(1, 'Select a state'),
  lga: z.string().min(1, 'Select your LGA'),
  zoneId: z.string().min(1, 'Choose your delivery area'),
  deliveryAddress: z
    .string()
    .min(10, 'Enter your full delivery address (at least 10 characters)'),
});

export type CheckoutDeliveryValues = z.infer<typeof checkoutDeliverySchema>;
