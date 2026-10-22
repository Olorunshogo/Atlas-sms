import type { LegalDocument } from '../../types/legal-document';
import { agentTerms } from './agent-terms';
import { buyerTerms } from './buyer-terms';

/*
 * Every published legal document, keyed by the slug in its own definition.
 *
 * Adding the agent agreement or the privacy policy is one content file and one
 * entry here. No route and no component changes, which is the same bargain
 * ROLE_SIGNUP_CONFIG makes for roles.
 *
 * The key is derived from each document rather than typed a second time, so a
 * slug and its URL cannot disagree.
 */
const DOCUMENTS: readonly LegalDocument[] = [buyerTerms, agentTerms];

export const LEGAL_DOCUMENTS: Readonly<Record<string, LegalDocument>> =
  Object.fromEntries(DOCUMENTS.map((document) => [document.slug, document]));

export function getLegalDocument(slug: string): LegalDocument | undefined {
  return LEGAL_DOCUMENTS[slug];
}

export function listLegalDocuments(): readonly LegalDocument[] {
  return DOCUMENTS;
}

/*
 * Where a role's terms live. Signup reads this to link the right document and
 * to record which one was accepted, so a role that has no published terms is
 * absent rather than pointed at somebody else's.
 */
export const ROLE_TERMS_SLUG: Readonly<Record<string, string>> = {
  buyer: buyerTerms.slug,
  agent: agentTerms.slug,
};

export { buyerTerms, agentTerms };
