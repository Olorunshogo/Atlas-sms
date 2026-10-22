import { cn } from '../utils';
import { Skeleton } from '../../components/skeleton';
import { densityTokens } from './table-density';
import type { TableColumn, TableDensity } from './table-types';

/*
 * Skeletons derived from the column definitions, so the placeholder has the same column count, the same widths and the same padding as the real thing.
 * A skeleton that does not match is worse than none: the page settles, then jumps.
 *
 * Widths cycle through a fixed list rather than being random, because a random width differs between server and client render and React calls that a hydration mismatch.
 */
const barWidths = ['w-24', 'w-32', 'w-20', 'w-28', 'w-16'] as const;

function barWidth(index: number): string {
  return barWidths[index % barWidths.length];
}

export interface TableSkeletonBodyProps<TRow> {
  columns: readonly TableColumn<TRow>[];
  rowCount: number;
  density: TableDensity;
  /** Adds a leading checkbox cell so the columns still line up. */
  selectable?: boolean;
}

export function TableSkeletonBody<TRow>({
  columns,
  rowCount,
  density,
  selectable = false,
}: TableSkeletonBodyProps<TRow>) {
  const tokens = densityTokens[density];

  return (
    <tbody>
      {Array.from({ length: rowCount }, (_, rowIndex) => (
        <tr key={rowIndex} className="border-line border-b last:border-0">
          {selectable && (
            <td className={tokens.bodyCell}>
              <Skeleton className="h-4 w-4" />
            </td>
          )}
          {columns.map((column, columnIndex) => (
            <td key={column.id} className={tokens.bodyCell}>
              <Skeleton
                className={cn(
                  tokens.skeletonRowHeight,
                  barWidth(columnIndex + rowIndex),
                )}
              />
            </td>
          ))}
        </tr>
      ))}
    </tbody>
  );
}

export interface CardSkeletonListProps {
  rowCount: number;
  density: TableDensity;
}

export function CardSkeletonList({ rowCount, density }: CardSkeletonListProps) {
  const tokens = densityTokens[density];

  return (
    <div className={cn('flex flex-col', tokens.cardGap)}>
      {Array.from({ length: rowCount }, (_, index) => (
        <div
          key={index}
          className={cn(
            'border-line flex flex-col gap-3 rounded-xl border',
            tokens.cardPadding,
          )}
        >
          <div className="flex items-start justify-between gap-3">
            <div className="flex flex-col gap-2">
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-3 w-24" />
            </div>
            <Skeleton className="h-5 w-20 rounded-full" />
          </div>
          <Skeleton className="h-3 w-full" />
        </div>
      ))}
    </div>
  );
}
