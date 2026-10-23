// === Field-level factories
export * from './rules';
export * from './field-factories';

// === vee-validate + Zod v4 bridge (see file comment for why this exists)
export * from './vee-validate-zod4';

// === Auth forms (reusable for Atlas as-is)
export * from './auth';

/*
 * Everything below is ported from a prior React project's reference schemas - mostly a business domain (admin/platform, field-agent, buyer, employee, careers, ratings) that isn't Atlas SMS's.
 * See each subfolder's index.ts for a one-line flag. support/ticket and support/marketing-contact are the one plausibly-reusable exception within support/.
 * admin and agent are left out of this barrel: both have a dangling import to a component that was never ported, so they're excluded from the build graph until pruned or fixed. The source stays on disk under src/admin and src/agent.
 */

export * from './buyer';
export * from './careers';
export * from './employee';
export * from './ratings';
export * from './support';
