import { productCategories } from '../data/product-categories';

/*
 * Sort is separate from the category filter chips: the chips decide which
 * products show, this decides what order the ones that pass appear in.
 *
 * "Catalogue" keeps the API's sort_order (admin shelf order). "Category"
 * reuses productCategories so the grid groups the same way the filter chips
 * read left to right.
 */

export type ProductSortKey =
  | 'catalogue'
  | 'category'
  | 'name-asc'
  | 'name-desc';

export interface SortableProduct {
  name: string;
  category: string | null;
}

export const PRODUCT_SORT_OPTIONS: { value: ProductSortKey; label: string }[] =
  [
    { value: 'catalogue', label: 'Featured' },
    { value: 'category', label: 'Category' },
    { value: 'name-asc', label: 'Name (A to Z)' },
    { value: 'name-desc', label: 'Name (Z to A)' },
  ];

/** Root ancestors not in the canonical list sort after every named one. */
function categoryRank(category: string | null): number {
  if (!category) return productCategories.length;
  const index = productCategories.indexOf(
    category as (typeof productCategories)[number],
  );
  return index === -1 ? productCategories.length : index;
}

export function sortProducts<T extends SortableProduct>(
  products: readonly T[],
  sortBy: ProductSortKey,
): T[] {
  const sorted = [...products];

  if (sortBy === 'catalogue') {
    return sorted;
  }

  if (sortBy === 'name-asc') {
    sorted.sort((a, b) => a.name.localeCompare(b.name));
  } else if (sortBy === 'name-desc') {
    sorted.sort((a, b) => b.name.localeCompare(a.name));
  } else {
    sorted.sort((a, b) => {
      const rankDiff = categoryRank(a.category) - categoryRank(b.category);
      return rankDiff !== 0 ? rankDiff : a.name.localeCompare(b.name);
    });
  }

  return sorted;
}
