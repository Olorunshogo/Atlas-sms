import { z } from 'zod';
import { createNameSchema } from '../field-factories';

export const updateEmployeeNameSchema = z.object({
  firstName: createNameSchema('First name'),
  lastName: createNameSchema('Last name'),
});

export type UpdateEmployeeNameValues = z.infer<typeof updateEmployeeNameSchema>;
