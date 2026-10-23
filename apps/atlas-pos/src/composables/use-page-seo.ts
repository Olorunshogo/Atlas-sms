import { buildPageSeo, type PageSeoInput } from '@atlas/utils';

function setNamedMeta(name: string, content: string): void {
  let tag = document.querySelector(`meta[name="${name}"]`);
  if (!tag) {
    tag = document.createElement('meta');
    tag.setAttribute('name', name);
    document.head.appendChild(tag);
  }
  tag.setAttribute('content', content);
}

function removeNamedMeta(name: string): void {
  document.querySelector(`meta[name="${name}"]`)?.remove();
}

function removePropertyMeta(property: string): void {
  document
    .querySelectorAll(`meta[property="${property}"], meta[name="${property}"]`)
    .forEach((tag) => tag.remove());
}

/**
 * POS has no Nuxt head helpers. This writes the same metadata shape in the
 * browser, and drops the social card on noIndex screens.
 */
export function usePageSeo(page: Omit<PageSeoInput, 'siteUrl'>): void {
  if (typeof document === 'undefined') return;

  const meta = buildPageSeo({
    ...page,
    siteUrl: window.location.origin,
  });

  document.title = meta.title;
  setNamedMeta('description', meta.description);
  setNamedMeta('author', meta.author);
  setNamedMeta('robots', meta.robots);

  const canonical = document.querySelector('link[rel="canonical"]');
  if (!meta.canonical) {
    removeNamedMeta('keywords');
    removePropertyMeta('og:title');
    removePropertyMeta('og:description');
    removePropertyMeta('og:image');
    removePropertyMeta('og:url');
    removePropertyMeta('twitter:card');
    removePropertyMeta('twitter:title');
    removePropertyMeta('twitter:description');
    removePropertyMeta('twitter:image');
    canonical?.remove();
    return;
  }

  if (meta.keywords) setNamedMeta('keywords', meta.keywords);
}
