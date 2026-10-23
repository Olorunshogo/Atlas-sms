import type { SelectOption } from '../types/location';

/*
 * Top-level merchandising categories for the shop's filter chips.
 *
 * These are the ROOTS of the product_categories tree. `products.category` is
 * populated server-side from a product's root ancestor, so these values and the
 * database agree without anyone maintaining both by hand.
 *
 * The previous list mixed levels - Rice, Beans and Garri are types under roots,
 * not categories alongside Oil and Tubers - which is why a "Rice" chip and a
 * "Grains" heading could never coexist. Deeper levels are not chips: the shop
 * filters by category, and drilling to a variety is the agent stock flow's job.
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

/** Chip list for the shop filter, with the "show everything" entry first. */
export const ALL_CATEGORIES = 'All';

export function categoryFilterChips(
  present: readonly (string | null)[],
): string[] {
  /*
   * Only offer a category that actually has products behind it - an empty
   * filter chip is worse than no chip.
   */
  const available = productCategories.filter((c) => present.includes(c));
  return [ALL_CATEGORIES, ...available];
}
