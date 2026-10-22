import type { MetaDescriptor } from 'react-router';

/*
 * One factory per app, not one shared instance: each of the marketing app, the buyer app, the agent app and the admin app has its own siteUrl (a different subdomain), so each calls createPageMetaBuilder once with its own defaults and every route in that app calls the buildPageMeta it gets back.
 * Change the og-image (or any other default) in one place and every route using it picks it up immediately.
 *
 * OG/Twitter/canonical fields are all optional on SeoDefaults: only the marketing app's public pages need them.
 * The dashboard apps supply just siteName/siteUrl/author, and the builder skips the whole social-card block for any noIndex page regardless.
 */

/*
 * `baseKeywords` are base keywords merged into every route's own, e.g. ["Store", "Shop"].
 * `defaultImage` is an absolute https URL; omit on an app with no public/indexable pages.
 * `twitterHandle` is e.g. "@example"; omit on an app with no public/indexable pages.
 * `locale` is e.g. "en_NG".
 */
export interface SeoDefaults {
  siteName: string;
  siteUrl: string;
  author: string;
  baseKeywords: readonly string[];
  defaultImage?: string;
  imageWidth?: string;
  imageHeight?: string;
  imageAlt?: string;
  twitterHandle?: string;
  locale?: string;
}

/*
 * `path` is the route path from the site root, e.g. "/shop" - builds og:url/canonical.
 * `keywords` are extra keywords for this route, merged with (not replacing) baseKeywords.
 * `image` overrides defaults.defaultImage for a route that wants its own card.
 * Pass `false` (not just omitting this) to explicitly suppress the card image for a route that still wants og:title/og:description - a legal document, for instance - rather than falling back to defaults.defaultImage.
 * `noIndex` is true for every auth/dashboard route: it skips the whole OG/Twitter/canonical block (a private page has no business declaring a social card) and sets robots to noindex, nofollow.
 */
export interface PageMetaInput {
  title: string;
  description: string;
  path: string;
  keywords?: readonly string[];
  image?: string | false;
  imageAlt?: string;
  type?: 'website' | 'article';
  noIndex?: boolean;
}

export type BuildPageMeta = (input: PageMetaInput) => MetaDescriptor[];

export function createPageMetaBuilder(defaults: SeoDefaults): BuildPageMeta {
  return function buildPageMeta(input: PageMetaInput): MetaDescriptor[] {
    const keywords = [...defaults.baseKeywords, ...(input.keywords ?? [])].join(
      ', ',
    );

    const tags: MetaDescriptor[] = [
      { title: input.title },
      { name: 'description', content: input.description },
      { name: 'keywords', content: keywords },
      { name: 'author', content: defaults.author },
      {
        name: 'robots',
        content: input.noIndex ? 'noindex, nofollow' : 'index, follow',
      },
    ];

    if (input.noIndex) return tags;

    const url = `${defaults.siteUrl}${input.path}`;
    const image =
      input.image === false
        ? undefined
        : (input.image ?? defaults.defaultImage);
    const imageAlt = input.imageAlt ?? defaults.imageAlt;

    tags.push({ tagName: 'link', rel: 'canonical', href: url });

    tags.push(
      { property: 'og:type', content: input.type ?? 'website' },
      { property: 'og:url', content: url },
      { property: 'og:site_name', content: defaults.siteName },
      { property: 'og:title', content: input.title },
      { property: 'og:description', content: input.description },
      { property: 'og:locale', content: defaults.locale ?? 'en_NG' },
      {
        name: 'twitter:card',
        content: image ? 'summary_large_image' : 'summary',
      },
      { name: 'twitter:url', content: url },
      { name: 'twitter:title', content: input.title },
      { name: 'twitter:description', content: input.description },
    );

    if (defaults.twitterHandle) {
      tags.push({ name: 'twitter:site', content: defaults.twitterHandle });
    }

    if (image) {
      /*
       * secure_url + type help WhatsApp / Facebook scrapers that ignore a bare
       * og:image, or keep an old preview after the asset changes. Keep width /
       * height declared so the card layout is known before the image downloads.
       */
      tags.push(
        { property: 'og:image', content: image },
        { property: 'og:image:secure_url', content: image },
        { property: 'og:image:type', content: 'image/png' },
        { property: 'og:image:alt', content: imageAlt ?? input.title },
        { name: 'twitter:image', content: image },
        { name: 'twitter:image:alt', content: imageAlt ?? input.title },
      );
      if (defaults.imageWidth) {
        tags.push({ property: 'og:image:width', content: defaults.imageWidth });
      }
      if (defaults.imageHeight) {
        tags.push({
          property: 'og:image:height',
          content: defaults.imageHeight,
        });
      }
    }

    return tags;
  };
}
