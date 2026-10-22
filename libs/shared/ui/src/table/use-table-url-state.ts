import { useRoute, useRouter } from 'vue-router';
import type {
  SortDirection,
  TableSort,
  TableStateSnapshot,
} from './table-types';

/*
 * Opt-in query string for page, sort, and search.
 * prefix keeps two tables on one route from sharing ?page.
 */

export interface TableUrlState {
  initialPage: number;
  initialSearch: string;
  initialSort: TableSort | null;
  writeState: (snapshot: TableStateSnapshot) => void;
}

function readQuery(value: unknown): string {
  if (Array.isArray(value)) {
    const first = value[0];
    return typeof first === 'string' ? first : '';
  }
  if (typeof value === 'string') {
    return value;
  }
  return '';
}

export function useTableUrlState(prefix = ''): TableUrlState {
  const route = useRoute();
  const router = useRouter();
  const pageKey = prefix ? prefix + '_page' : 'page';
  const searchKey = prefix ? prefix + '_q' : 'q';
  const sortKey = prefix ? prefix + '_sort' : 'sort';
  const dirKey = prefix ? prefix + '_dir' : 'dir';

  const pageRaw = Number(readQuery(route.query[pageKey]) || '1');
  const initialPage = Number.isFinite(pageRaw) && pageRaw > 0 ? pageRaw : 1;
  const initialSearch = readQuery(route.query[searchKey]);
  const sortName = readQuery(route.query[sortKey]);
  const direction = readQuery(route.query[dirKey]);
  let initialSort: TableSort | null = null;
  if (sortName && (direction === 'asc' || direction === 'desc')) {
    const sortDirection: SortDirection = direction;
    initialSort = { key: sortName, direction: sortDirection };
  }

  function writeState(snapshot: TableStateSnapshot): void {
    const next: Record<string, string> = {};
    for (const [key, value] of Object.entries(route.query)) {
      const text = readQuery(value);
      if (text) {
        next[key] = text;
      }
    }
    next[pageKey] = String(snapshot.page);
    if (snapshot.search) {
      next[searchKey] = snapshot.search;
    } else {
      delete next[searchKey];
    }
    if (snapshot.sort) {
      next[sortKey] = snapshot.sort.key;
      next[dirKey] = snapshot.sort.direction;
    } else {
      delete next[sortKey];
      delete next[dirKey];
    }
    void router.replace({ query: next });
  }

  return {
    initialPage,
    initialSearch,
    initialSort,
    writeState,
  };
}
