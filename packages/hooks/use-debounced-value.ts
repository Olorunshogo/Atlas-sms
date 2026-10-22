import { useEffect, useState } from 'react';

/**
 * Trails `value` by `delayMs`, so a keystroke does not become a request.
 *
 * The first value is returned immediately rather than after a delay - a table should render its initial state, not an empty one, while a timer runs.
 */
export function useDebouncedValue<T>(value: T, delayMs = 300): T {
  const [debounced, setDebounced] = useState<T>(value);

  useEffect(() => {
    if (delayMs <= 0) {
      setDebounced(value);
      return;
    }

    const timer = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(timer);
  }, [value, delayMs]);

  return debounced;
}
