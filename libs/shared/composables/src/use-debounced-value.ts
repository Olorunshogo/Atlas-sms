import { ref, watch, type Ref } from 'vue';

/**
 * Trails `value` by `delayMs`, so a keystroke does not become a request.
 * Ported from the reference source's React use-debounced-value.ts. The first value is
 * returned immediately rather than after a delay - a table should render
 * its initial state, not an empty one, while a timer runs.
 */
export function useDebouncedValue<T>(value: Ref<T>, delayMs = 300): Ref<T> {
  const debounced = ref(value.value) as Ref<T>;
  let timer: ReturnType<typeof setTimeout> | null = null;

  watch(value, (next: T) => {
    if (timer) clearTimeout(timer);

    if (delayMs <= 0) {
      debounced.value = next;
      return;
    }

    timer = setTimeout(() => {
      debounced.value = next;
    }, delayMs);
  });

  return debounced;
}
