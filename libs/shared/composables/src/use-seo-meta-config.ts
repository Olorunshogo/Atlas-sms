import { useHead, useRuntimeConfig, useSeoMeta } from 'nuxt/app';
import { buildPageSeo } from '@atlas/utils';

export interface SeoMetaPageConfig {
  title: string;
  description: string;
  path: string;
  keywords?: readonly string[];
  noIndex?: boolean;
}

interface SeoRuntimeConfig {
  siteUrl: string;
}

/**
 * Page SEO for Atlas Nuxt apps. Title, description, extra keywords, and
 * noIndex are per page. Author, base keywords, and the social-card shape
 * come from the shared public constants. noIndex pages skip the Open Graph
 * and Twitter tags.
 */
export function useSeoMetaConfig(page: SeoMetaPageConfig): void {
  const seo = useRuntimeConfig().public.seo as SeoRuntimeConfig;
  const meta = buildPageSeo({
    title: page.title,
    description: page.description,
    path: page.path,
    keywords: page.keywords,
    noIndex: page.noIndex,
    siteUrl: seo.siteUrl,
  });

  useHead({
    title: meta.title,
    titleTemplate: '%s',
    htmlAttrs: { lang: 'en' },
    meta: [{ name: 'author', content: meta.author }],
    link: [
      { rel: 'icon', type: 'image/x-icon', href: '/favicon.ico' },
      ...(meta.canonical
        ? [{ rel: 'canonical' as const, href: meta.canonical }]
        : []),
    ],
  });

  useSeoMeta({
    description: meta.description,
    robots: meta.robots,
    ...(meta.canonical
      ? {
          keywords: meta.keywords,
          ogTitle: meta.title,
          ogDescription: meta.description,
          ogImageAlt: meta.imageAlt,
          ogType: 'website' as const,
          ogUrl: meta.canonical,
          ogSiteName: meta.siteName,
          twitterCard: 'summary_large_image' as const,
          twitterTitle: meta.title,
          twitterDescription: meta.description,
        }
      : {}),
  });
}
