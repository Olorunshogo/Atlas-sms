import { z } from 'zod';
import {
  createPhoneSchemaWithOptions,
  createRequiredStringWithOptions,
} from '../field-factories';

export const outreachLeadTypes = ['buyer', 'farmer', 'supplier'] as const;
export type OutreachLeadType = (typeof outreachLeadTypes)[number];

export const MAX_OUTREACH_ADDITIONAL_PHONES = 2;

const additionalPhoneEntry = z.union([
  z.literal(''),
  createPhoneSchemaWithOptions({
    invalidMessage: 'Enter a valid phone number',
  }),
]);

/* Mirrors createOutreachSchema on the reference backend (admin.controller.ts). */
export const createOutreachRecordSchema = z.object({
  shopName: createRequiredStringWithOptions('Shop (owner) name'),
  leadType: z.enum(outreachLeadTypes),
  phone: createPhoneSchemaWithOptions({
    invalidMessage: 'Enter a valid phone number',
  }),
  additionalPhones: z
    .array(additionalPhoneEntry)
    .max(MAX_OUTREACH_ADDITIONAL_PHONES),
  lga: z.string(),
  area: z.string(),
  address: z.string(),
  productInterest: z.string(),
  quantity: z.string(),
  notes: z.string(),
  collectedBy: z.string(),
  visitDate: z.string(),
});

export type CreateOutreachRecordValues = z.infer<
  typeof createOutreachRecordSchema
>;

/*
 * One freeform note (plus photos) per lead per week.
 * Note is required; pictures are optional and validated at the upload boundary.
 */
export const outreachNoteSchema = z.object({
  note: createRequiredStringWithOptions('Note'),
});

export type OutreachNoteValues = z.infer<typeof outreachNoteSchema>;

// Row shape returned by GET /admin/outreach, shared so every outreach table renders the same record.
export interface OutreachRecord {
  id: number;
  shop_name: string;
  owner_name: string | null;
  phone: string | null;
  additional_phones?: string[] | null;
  lga: string | null;
  area: string | null;
  address: string | null;
  product_interest: string | null;
  quantity: number | null;
  notes: string | null;
  collected_by: string | null;
  visit_date: string;
  lead_type: OutreachLeadType;
  picture_urls: string[] | null;
  created_at: string;
}

/* Primary first, then extras. Used for search, tel: links, and preview. */
export function outreachPhones(record: OutreachRecord): string[] {
  const extras = (record.additional_phones ?? []).map((p) => p.trim());
  const primary = record.phone?.trim();
  return [primary, ...extras].filter((p): p is string => Boolean(p));
}
