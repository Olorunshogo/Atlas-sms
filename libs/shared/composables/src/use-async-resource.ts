import { ref, watch, type Ref, type WatchSource } from 'vue';

// === Types

export interface UseAsyncResourceOptions {
  enabled?: boolean;
}

export interface UseAsyncResourceResult<T> {
  data: Ref<T | null>;
  error: Ref<Error | null>;
  loading: Ref<boolean>;
  refetching: Ref<boolean>;
  refetch: () => void;
}

// === Composable

/**
 * Runs `fetcher` whenever `deps` change, with a fresh AbortController per
 * run - ported from the reference source's React use-async-resource.ts. Previous `data`
 * stays visible while a re-run is in flight (`refetching`), so a refresh
 * never flashes a skeleton over content that is already good. `loading` is
 * reserved for the very first run with nothing to show yet. An aborted run
 * is a normal outcome of a superseding run and never becomes an error.
 */
export function useAsyncResource<T>(
  fetcher: (signal: AbortSignal) => Promise<T>,
  deps: WatchSource[],
  opts?: UseAsyncResourceOptions,
): UseAsyncResourceResult<T> {
  const enabled: boolean = opts?.enabled ?? true;

  const data = ref<T | null>(null) as Ref<T | null>;
  const error = ref<Error | null>(null);
  const loading = ref<boolean>(false);
  const refetching = ref<boolean>(false);
  const nonce = ref<number>(0);

  let activeController: AbortController | null = null;

  function run(): void {
    if (!enabled) return;

    activeController?.abort();
    const controller = new AbortController();
    activeController = controller;

    const hasData = data.value !== null;
    if (hasData) refetching.value = true;
    else loading.value = true;
    error.value = null;

    fetcher(controller.signal)
      .then((result: T) => {
        if (controller.signal.aborted) return;
        data.value = result;
      })
      .catch((err: unknown) => {
        if (controller.signal.aborted) return;
        if (err instanceof DOMException && err.name === 'AbortError') return;
        if (err instanceof Error && err.name === 'AbortError') return;
        error.value = err instanceof Error ? err : new Error(String(err));
      })
      .finally(() => {
        if (controller.signal.aborted) return;
        loading.value = false;
        refetching.value = false;
      });
  }

  watch([...deps, () => enabled, nonce], run, { immediate: true });

  function refetch(): void {
    nonce.value += 1;
  }

  return { data, error, loading, refetching, refetch };
}
