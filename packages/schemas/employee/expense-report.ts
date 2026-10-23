import { z } from 'zod';

export const expenseCategoryOptions = [
  { value: 'travel', label: 'Travel' },
  { value: 'meals', label: 'Meals' },
  { value: 'materials', label: 'Materials' },
  { value: 'client_entertainment', label: 'Client entertainment' },
  { value: 'other', label: 'Other' },
] as const;

/* Mirrors createExpenseSchema on the reference backend (careers-ops.dto.ts); amountNaira is the input mask, converted to amount_kobo at the submission boundary - money is kobo everywhere past the form. */
export const expenseReportSchema = z.object({
  description: z.string().min(3, 'Describe the expense').max(5000),
  amountNaira: z
    .number({ message: 'Enter the amount' })
    .positive('Enter an amount greater than 0'),
  expenseDate: z.string().min(1, 'Pick the expense date'),
  category: z.enum(
    ['travel', 'meals', 'materials', 'client_entertainment', 'other'],
    { message: 'Select a category' },
  ),
});

export type ExpenseReportValues = z.infer<typeof expenseReportSchema>;
