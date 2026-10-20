import { useHead, useRuntimeConfig, useSeoMeta } from 'nuxt/app';

// === Types

export interface SeoMetaPageConfig {
  title: string;
  titleTemplate?: string;
  description: string;
  ogTitle?: string;
  ogDescription?: string;
  ogUrl: string;
  twitterTitle?: string;
  twitterDescription?: string;
  keywords?: string[];
}

/**
 * Site-wide defaults every Atlas Nuxt app supplies via its own
 * `nuxt.config.ts` (`runtimeConfig.public.seo`) - never hardcoded here,
 * since `atlas-web` and `landing-page` are different domains/products
 * and shouldn't share a site URL, OG image, or Twitter handle.
 */
export interface SeoRuntimeConfig {
  siteUrl: string;
  siteName: string;
  defaultOgImage: string;
  ogImageWidth?: number;
  ogImageHeight?: number;
  ogImageAlt: string;
  twitterHandle?: string;
  baseKeywords?: string[];
}

// === Composable

/**
 * One SEO composable for every Atlas Nuxt app. Combines `useHead`
 * (canonical link, favicon, lang) and `useSeoMeta` (OG/Twitter tags) so a
 * page only has to supply what's page-specific. Site-wide defaults come
 * from `useRuntimeConfig().public.seo`, set once per app.
 */
export function useSeoMetaConfig(page: SeoMetaPageConfig): void {
  const { public: publicConfig } = useRuntimeConfig();
  const seo = publicConfig.seo as SeoRuntimeConfig;

  const allKeywords = [
    ...(seo.baseKeywords ?? []),
    ...(page.keywords ?? []),
  ].join(', ');
  const absoluteOgImage = `${seo.siteUrl}${seo.defaultOgImage}`;

  useHead({
    title: page.title,
    titleTemplate: page.titleTemplate ?? `%s - ${seo.siteName}`,
    htmlAttrs: { lang: 'en' },
    link: [
      { rel: 'canonical', href: page.ogUrl },
      { rel: 'icon', type: 'image/x-icon', href: '/favicon.ico' },
    ],
  });

  useSeoMeta({
    title: page.title,
    titleTemplate: page.titleTemplate ?? `%s - ${seo.siteName}`,
    description: page.description,
    ogTitle: page.ogTitle ?? page.title,
    ogDescription: page.ogDescription ?? page.description,
    ogImage: absoluteOgImage,
    ogImageWidth: seo.ogImageWidth ?? 1200,
    ogImageHeight: seo.ogImageHeight ?? 630,
    ogImageAlt: seo.ogImageAlt,
    ogType: 'website',
    ogUrl: page.ogUrl,
    ogSiteName: seo.siteName,
    twitterCard: 'summary_large_image',
    twitterTitle: page.twitterTitle ?? page.title,
    twitterDescription: page.twitterDescription ?? page.description,
    twitterImage: absoluteOgImage,
    ...(seo.twitterHandle ? { twitterSite: seo.twitterHandle } : {}),
    keywords: allKeywords,
    robots: 'index, follow',
  });
}
