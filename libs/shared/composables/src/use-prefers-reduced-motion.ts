import type { Ref } from 'vue';
import { useMediaQuery } from './use-media-query';

/**
 * True when the viewer has asked the OS to reduce motion - ported from
 * the reference source's React use-prefers-reduced-motion.ts. Server reports false and
 * the client swaps after mount - animations opt out on the client only,
 * never mismatch on render.
 */
export function usePrefersReducedMotion(): Ref<boolean> {
  return useMediaQuery('(prefers-reduced-motion: reduce)', false);
}
