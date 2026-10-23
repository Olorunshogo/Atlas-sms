export const ATLAS_SITE_NAME = 'AtlasSMS';
export const ATLAS_AUTHOR = 'AtlasSMS Team';
export const ATLAS_CATCHPHRASE = 'One catalogue across every store.';
export const ATLAS_BASE_KEYWORDS = [
  'AtlasSMS',
  'store management',
  'inventory',
  'point of sale',
] as const;

export interface PageSeoInput {
  title: string;
  description: string;
  path: string;
  keywords?: readonly string[];
  noIndex?: boolean;
  siteUrl: string;
}

export interface PageSeoResult {
  title: string;
  description: string;
  author: string;
  robots: 'index, follow' | 'noindex, nofollow';
  keywords?: string;
  canonical?: string;
  image?: string;
  imageAlt: string;
  siteName: string;
}

function brandedTitle(title: string): string {
  if (title === ATLAS_SITE_NAME) return title;
  return `${title} - ${ATLAS_SITE_NAME}`;
}

/**
 * Shared page metadata. noIndex stops before keywords, canonical, and image
 * so private screens do not advertise a social card.
 */
export function buildPageSeo(input: PageSeoInput): PageSeoResult {
  const title = brandedTitle(input.title);
  const base = {
    title,
    description: input.description,
    author: ATLAS_AUTHOR,
    imageAlt: `${ATLAS_SITE_NAME} logo`,
    siteName: ATLAS_SITE_NAME,
  };

  if (input.noIndex) {
    return {
      ...base,
      robots: 'noindex, nofollow',
    };
  }

  const origin = input.siteUrl.replace(/\/$/, '');
  const path = input.path.startsWith('/') ? input.path : `/${input.path}`;
  const keywords = [...ATLAS_BASE_KEYWORDS, ...(input.keywords ?? [])].join(
    ', ',
  );

  return {
    ...base,
    robots: 'index, follow',
    keywords,
    canonical: `${origin}${path}`,
    image: `${origin}/logo.png`,
  };
}
