import { z } from 'zod';

/*
 * Mirrors submitReportSchema on the reference backend (submit-report.dto.ts).
 * Per-product items (product, quantity, amount collected) are managed as
 * plain component state, the same way request-stock.tsx's line-item picker
 * already does it - not through this schema, since react-hook-form has
 * nothing to register against a picker-built list.
 */
export const dailyReportSchema = z.object({
  areaCovered: z.string().min(1, 'Select the area you covered today'),
  feedback: z.string(),
  unsoldReason: z.string(),
});

export type DailyReportValues = z.infer<typeof dailyReportSchema>;
