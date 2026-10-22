import type { TableDensity } from './table-types';

/*
 * One padding scale for every table in the codebase.
 *
 * Before this existed, tables ran px-5 py-4 in one place and px-6 py-4 in another, with header text sometimes uppercase-xs and sometimes sm.
 * Density is a prop now, not a per-page decision.
 */
export interface DensityTokens {
  headerCell: string;
  bodyCell: string;
  cardPadding: string;
  cardGap: string;
  skeletonRowHeight: string;
}

export const densityTokens: Record<TableDensity, DensityTokens> = {
  comfortable: {
    headerCell: 'px-5 py-3',
    bodyCell: 'px-5 py-4',
    cardPadding: 'p-4',
    cardGap: 'gap-3',
    skeletonRowHeight: 'h-5',
  },
  compact: {
    headerCell: 'px-4 py-2',
    bodyCell: 'px-4 py-2.5',
    cardPadding: 'p-3',
    cardGap: 'gap-2',
    skeletonRowHeight: 'h-4',
  },
};

export const alignClass: Record<'left' | 'center' | 'right', string> = {
  left: 'text-left',
  center: 'text-center',
  right: 'text-right',
};
