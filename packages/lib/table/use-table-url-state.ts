import { useCallback, useMemo, useRef } from 'react';
import { useSearchParams } from 'react-router';
import type {
  SortDirection,
  TableSort,
  TableStateSnapshot,
} from './table-types';

/*
 * Keeps a table's page, sort and search in the query string.
 *
 * Opt-in, because not every table wants it.
 * Worth it for admin lists: a refresh keeps your place, a link to page 4 is a link to page 4, and coming back from a detail route lands where you left rather than at the top.
 *
 * `prefix` namespaces the params so two tables on one page do not fight over `?page`.
 */

export interface UseTableUrlStateOptions {
  prefix?: string;
  defaultPageSize?: number;
  defaultSort?: TableSort | null;
}

export interface TableUrlState {
  initialPage: number;
  initialSearch: string;
  initialSort: TableSort | null;
  initialPageSize: number;
  /** Pass to DataTable's onStateChange, or call it from your own handler. */
  writeState: (snapshot: TableStateSnapshot) => void;
}

export function useTableUrlState(
  options: UseTableUrlStateOptions = {},
): TableUrlState {
  const { prefix, defaultPageSize = 10, defaultSort = null } = options;
  const [searchParams, setSearchParams] = useSearchParams();

  const key = useCallback(
    (name: string): string => (prefix ? `${prefix}_${name}` : name),
    [prefix],
  );

  /*
   * Read once.
   * After mount the engine owns the state, and re-reading on every params change would fight the writes this same hook makes.
   */
  const initial = useRef<Omit<TableUrlState, 'writeState'>>({
    initialPage: 1,
    initialSearch: '',
    initialSort: defaultSort,
    initialPageSize: defaultPageSize,
  });
  const hasRead = useRef<boolean>(false);
  if (!hasRead.current) {
    hasRead.current = true;
    const rawPage = Number(searchParams.get(key('page')));
    const rawSize = Number(searchParams.get(key('size')));
    const rawSort = searchParams.get(key('sort'));
    const rawOrder = searchParams.get(key('order'));

    initial.current = {
      initialPage: Number.isInteger(rawPage) && rawPage > 0 ? rawPage : 1,
      initialSearch: searchParams.get(key('q')) ?? '',
      initialSort: rawSort
        ? {
            key: rawSort,
            direction: rawOrder === 'asc' ? 'asc' : ('desc' as SortDirection),
          }
        : defaultSort,
      initialPageSize:
        Number.isInteger(rawSize) && rawSize > 0 ? rawSize : defaultPageSize,
    };
  }

  const writeState = useCallback(
    (snapshot: TableStateSnapshot): void => {
      setSearchParams(
        (current) => {
          const next = new URLSearchParams(current);

          const set = (name: string, value: string, isDefault: boolean) => {
            if (isDefault) next.delete(key(name));
            else next.set(key(name), value);
          };

          set('page', String(snapshot.page), snapshot.page === 1);
          set('q', snapshot.search, snapshot.search === '');
          set(
            'size',
            String(snapshot.pageSize),
            snapshot.pageSize === defaultPageSize,
          );
          set('sort', snapshot.sort?.key ?? '', !snapshot.sort);
          set('order', snapshot.sort?.direction ?? '', !snapshot.sort);

          return next;
        },
        /* Replace, so paging does not fill the back button with every page. */
        { replace: true, preventScrollReset: true },
      );
    },
    [setSearchParams, key, defaultPageSize],
  );

  return useMemo<TableUrlState>(
    () => ({ ...initial.current, writeState }),
    [writeState],
  );
}
