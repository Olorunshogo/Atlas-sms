/*
 * @atlas/ui-core is the one package both atlas-web and atlas-pos depend
 * on, so this is where @vueuse/core is wired in - not an Atlas-owned
 * reimplementation of debounce/media-query/reduced-motion (those used to
 * be ported hooks under packages/hooks, now cleared in favour of this).
 */
export {
  useDebounce,
  useMediaQuery,
  usePreferredReducedMotion,
} from '@vueuse/core';
