import { z } from 'zod';
import type { PhotoUpload } from '../../components/upload-field';

export const deliveryVerificationSchema = z.object({
  photos: z.custom<PhotoUpload[]>(
    (value) => Array.isArray(value) && value.length > 0,
    {
      message: 'Please upload at least one photo',
    },
  ),
  recipientName: z.string(),
  notes: z.string(),
});

export type DeliveryVerificationValues = z.infer<
  typeof deliveryVerificationSchema
>;
