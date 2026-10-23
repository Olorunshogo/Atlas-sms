import { useSeoMetaConfig, type SeoMetaPageConfig } from '@atlas/composables';

/**
 * Web pages call this. noIndex routes stop after title, description, author,
 * and robots inside useSeoMetaConfig.
 */
export function usePageSeo(page: SeoMetaPageConfig): void {
  useSeoMetaConfig(page);
}
