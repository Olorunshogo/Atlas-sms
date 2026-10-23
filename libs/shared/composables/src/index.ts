export * from './use-seo-meta-config';
export * from './use-sidebar';

// === Utility composables (ported from the reference source's React hooks package)
export * from './use-debounced-value';
export * from './use-media-query';
export * from './use-prefers-reduced-motion';
export * from './use-async-resource';
export * from './use-dashboard-nav';

// === Notifications
export * from './use-notifications-service';
export * from './notifications-provider';

// === Auth
export * from './auth';

// === Payment
export * from './payment';

/*
 * Ratings deliberately not exported from the barrel: it's the reference source's
 * marketplace review feature (see ratings/use-rating-submission-service.ts),
 * not wired to anything in Atlas, and its only import is from @atlas/ui -
 * a package that needs vue-tsc, not plain tsc, to build its .vue exports,
 * which this package's project-reference build can't cross.
 * The source stays on disk for the prune-or-keep decision; it's just not
 * part of this package's public surface or build graph until that's resolved.
 */
