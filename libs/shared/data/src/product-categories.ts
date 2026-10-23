import type { SelectOption } from '@atlas/types';

/*
 * Reference-source values ("Grains", "Beans", "Oil", "Tubers", "Processed" -
 * a grocery marketplace's top-level categories). The TYPE and function shape
 * are generically useful (a fixed, root-level category list for filter
 * chips, matched server-side), but Atlas's actual categories are per-tenant
 * retail data, not a fixed global list - replace the productCategories array
 * before use, or move this to a tenant-scoped API call instead of a static
 * export.
 */
export const productCategories = [
  'Grains',
  'Beans',
  'Oil',
  'Tubers',
  'Processed',
] as const;

export type ProductCategory = (typeof productCategories)[number];

export function productCategoryOptions(): SelectOption[] {
  return productCategories.map((category) => ({
    value: category,
    label: category,
  }));
}

/** Chip list for a shop filter, with the "show everything" entry first. */
export const ALL_CATEGORIES = 'All';

export function categoryFilterChips(
  present: readonly (string | null)[],
): string[] {
  const available = productCategories.filter((c) => present.includes(c));
  return [ALL_CATEGORIES, ...available];
}
