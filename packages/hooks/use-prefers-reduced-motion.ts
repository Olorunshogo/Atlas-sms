import { useMediaQuery } from './use-media-query';

/**
 * True when the viewer has asked the OS to reduce motion.
 *
 * Server has no matchMedia, so it always reports false and the client swaps on hydration - animations opt out on the client only, never mismatch on render.
 */
export function usePrefersReducedMotion(): boolean {
  return useMediaQuery('(prefers-reduced-motion: reduce)', false);
}
