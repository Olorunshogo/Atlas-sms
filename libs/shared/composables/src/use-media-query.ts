import { onBeforeUnmount, onMounted, ref, type Ref } from 'vue';

/**
 * Subscribes to a media query - ported from the reference source's React use-media-query.ts
 * (useSyncExternalStore -> ref + mount/unmount listener). `serverSnapshot` is
 * what the value reports before the client can read window.matchMedia -
 * matches on the server (Nuxt SSR) and before onMounted runs client-side.
 */
export function useMediaQuery(
  query: string,
  serverSnapshot = false,
): Ref<boolean> {
  const matches = ref<boolean>(serverSnapshot);
  let list: MediaQueryList | null = null;

  function onChange(): void {
    if (list) matches.value = list.matches;
  }

  onMounted(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return;
    list = window.matchMedia(query);
    matches.value = list.matches;
    list.addEventListener('change', onChange);
  });

  onBeforeUnmount(() => {
    list?.removeEventListener('change', onChange);
  });

  return matches;
}

/** Tailwind's `md` breakpoint, the point the table engine swaps card for table. */
export const MD_BREAKPOINT_QUERY = '(min-width: 48rem)';
