import { useCallback, useEffect, useRef, useState } from 'react';

// === Types

export interface UseAsyncResourceOptions {
  enabled?: boolean;
}

export interface UseAsyncResourceResult<T> {
  data: T | null;
  error: Error | null;
  loading: boolean;
  refetching: boolean;
  refetch: () => void;
}

// === Hook

/**
 * Runs `fetcher` whenever `deps` change, with a fresh AbortController per run.
 *
 * Previous `data` stays visible while a re-run is in flight (`refetching`), so a refresh never flashes a skeleton over content that is already good.
 * `loading` is reserved for the very first run with nothing to show yet.
 * An aborted run is a normal outcome of a superseding run and never becomes an error.
 */
export function useAsyncResource<T>(
  fetcher: (signal: AbortSignal) => Promise<T>,
  deps: unknown[],
  opts?: UseAsyncResourceOptions,
): UseAsyncResourceResult<T> {
  const enabled: boolean = opts?.enabled ?? true;

  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [refetching, setRefetching] = useState<boolean>(false);

  // Bumped by refetch() to force a re-run without changing caller deps.
  const [nonce, setNonce] = useState<number>(0);

  const dataRef = useRef<T | null>(null);
  dataRef.current = data;

  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;

  useEffect(() => {
    if (!enabled) return;

    const controller = new AbortController();
    const hasData: boolean = dataRef.current !== null;

    if (hasData) setRefetching(true);
    else setLoading(true);
    setError(null);

    fetcherRef
      .current(controller.signal)
      .then((result: T) => {
        if (controller.signal.aborted) return;
        setData(result);
      })
      .catch((err: unknown) => {
        if (controller.signal.aborted) return;
        if (err instanceof DOMException && err.name === 'AbortError') return;
        if (err instanceof Error && err.name === 'AbortError') return;
        setError(err instanceof Error ? err : new Error(String(err)));
      })
      .finally(() => {
        if (controller.signal.aborted) return;
        setLoading(false);
        setRefetching(false);
      });

    return () => controller.abort();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, enabled, nonce]);

  const refetch = useCallback((): void => {
    setNonce((n: number): number => n + 1);
  }, []);

  return { data, error, loading, refetching, refetch };
}
