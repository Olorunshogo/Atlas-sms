/*
 * The shape of a legal document: terms, a privacy policy, a staff agreement.
 * Generic content model, applies to Atlas directly even though the original
 * comment referenced the source site - the shape has no source-specific
 * content baked in.
 *
 * The TYPE lives here because the shared renderer consumes it. The DOCUMENTS
 * themselves stay in the consuming app or @atlas/data - what a role has
 * agreed to is company policy, not shared UI.
 *
 * Content is data rather than markup so a document can be rendered, linked to
 * by clause, and version-compared without a component knowing what a role is.
 */

// === Inline content

/*
 * A run of text inside a block. A link is data rather than an anchor element
 * so a content file never imports a UI framework - the same object can be
 * rendered to a page, and later to an email or a PDF, with no rewrite.
 */
export type LegalInline =
  | string
  | { type: 'link'; text: string; href: string }
  | { type: 'strong'; text: string };

// === Blocks

export interface LegalParagraphBlock {
  type: 'paragraph';
  content: readonly LegalInline[];
}

/*
 * One entry in a Definitions section: the defined term and its meaning. Kept
 * distinct from a paragraph because the term is quoted and bound throughout
 * the document, so it is styled and anchored as a definition rather than as
 * prose.
 */
export interface LegalDefinitionBlock {
  type: 'definition';
  term: string;
  content: readonly LegalInline[];
}

/** A clause quoted verbatim, set apart from the surrounding explanation. */
export interface LegalQuoteBlock {
  type: 'quote';
  content: readonly LegalInline[];
  attribution?: string;
}

export interface LegalListBlock {
  type: 'list';
  ordered?: boolean;
  items: readonly (readonly LegalInline[])[];
}

/** A labelled contact line: "Email", "support@example.com". */
export interface LegalContactBlock {
  type: 'contact';
  entries: readonly { label: string; value: string; href?: string }[];
}

/** An aside that is not itself a term: an effective date, a pointer, a caveat. */
export interface LegalNoteBlock {
  type: 'note';
  content: readonly LegalInline[];
}

export type LegalBlock =
  | LegalParagraphBlock
  | LegalDefinitionBlock
  | LegalQuoteBlock
  | LegalListBlock
  | LegalContactBlock
  | LegalNoteBlock;

// === Sections

/*
 * A numbered section, which may contain numbered subsections.
 *
 * `children` is what lets 7.1 and 27.6 nest rather than being flattened into
 * headings that only look nested. The table of contents and the anchor ids
 * are both derived from this tree, so a renumbering cannot leave them
 * disagreeing.
 *
 * id is the anchor fragment, stable across revisions so an old link keeps
 * working. number is the clause number as displayed, "7" or "10.8". Omitted
 * for an unnumbered section.
 */
export interface LegalSection {
  id: string;
  number?: string;
  heading: string;
  blocks?: readonly LegalBlock[];
  children?: readonly LegalSection[];
}

// === Document

export interface LegalRevision {
  version: string;
  date: string;
  summary: string;
}

/*
 * slug is the registry key and URL segment: /legal/<slug>.
 * version is what a consent is recorded against. A signup stores this
 * string, so a revision that changes obligations must raise it or the record
 * will claim the user agreed to text they never saw.
 * intro is the preamble before the first numbered section.
 * closing is the closing line under the last section.
 */
export interface LegalDocument {
  slug: string;
  title: string;
  subtitle?: string;
  version: string;
  effectiveDate: string;
  lastRevised: string;
  revisions?: readonly LegalRevision[];
  intro?: readonly LegalBlock[];
  sections: readonly LegalSection[];
  closing?: string;
}
