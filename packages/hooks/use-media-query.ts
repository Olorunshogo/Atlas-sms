import { useCallback, useSyncExternalStore } from 'react';

/**
 * Subscribes to a media query.
 *
 * useSyncExternalStore rather than useEffect + useState, because it gives a stable server snapshot: SSR renders `serverSnapshot` and the client swaps on hydration without React complaining about a mismatch.
 * Callers that switch layouts on this should pass the layout the server should assume.
 */
export function useMediaQuery(query: string, serverSnapshot = false): boolean {
  const subscribe = useCallback(
    (onChange: () => void): (() => void) => {
      /* Guarded because a non-browser environment has no matchMedia at all. */
      if (typeof window === 'undefined' || !window.matchMedia) return () => {};

      const list = window.matchMedia(query);
      list.addEventListener('change', onChange);
      return () => list.removeEventListener('change', onChange);
    },
    [query],
  );

  const getSnapshot = useCallback((): boolean => {
    if (typeof window === 'undefined' || !window.matchMedia) {
      return serverSnapshot;
    }
    return window.matchMedia(query).matches;
  }, [query, serverSnapshot]);

  const getServerSnapshot = useCallback(
    (): boolean => serverSnapshot,
    [serverSnapshot],
  );

  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}

/** Tailwind's `md` breakpoint, the point the table engine swaps card for table. */
export const MD_BREAKPOINT_QUERY = '(min-width: 48rem)';
