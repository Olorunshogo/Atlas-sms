// === Ratings
// Not wired into Atlas yet - kept intact, to be built out and modified later.
export * from './ratings';

/*
 * Everything else here (auth, payment, the generic utility hooks, dashboard
 * nav, notifications) had its content cleared: auth and payment are already
 * superseded by the real Vue composables in libs/shared/composables, and
 * the generic hooks (debounce, media query, reduced motion) are now covered
 * by @vueuse/core via @atlas/ui-core instead of an Atlas-owned copy. Files
 * stay as placeholders. See docs/atlas/CONTEXT.md.
 */
