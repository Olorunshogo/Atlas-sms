import type { LegalDocument } from '@atlas/types';

/*
 * The original source (packages/data/legal/) held the reference source's actual Terms of
 * Service text: agent-terms.ts (247 lines) and buyer-terms.ts (1448 lines)
 * of real contract prose for that project's marketplace. That content was
 * deliberately NOT ported - it is legally binding text for a different
 * company and would be actively wrong sitting in Atlas's codebase, not
 * merely irrelevant. Only the registry pattern is kept here, empty, so
 * Atlas's own terms/privacy-policy documents (written in @atlas/types's
 * LegalDocument shape) can be dropped in the same way later.
 */
const DOCUMENTS: readonly LegalDocument[] = [];

export const LEGAL_DOCUMENTS: Readonly<Record<string, LegalDocument>> =
  Object.fromEntries(DOCUMENTS.map((document) => [document.slug, document]));

export function getLegalDocument(slug: string): LegalDocument | undefined {
  return LEGAL_DOCUMENTS[slug];
}

export function listLegalDocuments(): readonly LegalDocument[] {
  return DOCUMENTS;
}

export const ROLE_TERMS_SLUG: Readonly<Record<string, string>> = {};
