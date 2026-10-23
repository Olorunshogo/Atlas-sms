import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { PageLoaderOverlay } from './page-loader-overlay';

/*
 * A ref-counted `showLoader`/`hideLoader` pair, not a boolean.
 * A boolean breaks the moment two callers overlap: the first one's `hideLoader` would hide the overlay while the second caller's operation is still running.
 * Counting concurrent callers is what makes `showLoader`/`hideLoader` safe to call from independent places without either one needing to know about the other.
 *
 * The overlay itself waits `SHOW_DELAY_MS` before appearing and then stays for at least `MIN_VISIBLE_MS`, so a fast operation never flashes it in and out.
 */
const SHOW_DELAY_MS = 150;
const MIN_VISIBLE_MS = 300;

export interface PageLoaderContextValue {
  showLoader: () => void;
  hideLoader: () => void;
  /** Wraps a promise in show/hide, guaranteeing hideLoader runs even if the promise rejects. */
  withLoader: <T>(action: () => Promise<T>) => Promise<T>;
}

const PageLoaderContext = createContext<PageLoaderContextValue | null>(null);

export function PageLoaderProvider({ children }: { children: ReactNode }) {
  const [visible, setVisible] = useState<boolean>(false);
  const countRef = useRef<number>(0);
  const showTimerRef = useRef<number | null>(null);
  const hideTimerRef = useRef<number | null>(null);
  const shownAtRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (showTimerRef.current) window.clearTimeout(showTimerRef.current);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    };
  }, []);

  const showLoader = useCallback((): void => {
    countRef.current += 1;
    if (countRef.current > 1) return;

    if (hideTimerRef.current) {
      window.clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }

    showTimerRef.current = window.setTimeout(() => {
      showTimerRef.current = null;
      if (countRef.current > 0) {
        shownAtRef.current = Date.now();
        setVisible(true);
      }
    }, SHOW_DELAY_MS);
  }, []);

  const hideLoader = useCallback((): void => {
    countRef.current = Math.max(0, countRef.current - 1);
    if (countRef.current > 0) return;

    if (showTimerRef.current) {
      window.clearTimeout(showTimerRef.current);
      showTimerRef.current = null;
      return;
    }

    const shownAt = shownAtRef.current;
    if (shownAt === null) return;

    const remaining = MIN_VISIBLE_MS - (Date.now() - shownAt);
    if (remaining <= 0) {
      setVisible(false);
      shownAtRef.current = null;
      return;
    }

    hideTimerRef.current = window.setTimeout(() => {
      hideTimerRef.current = null;
      if (countRef.current === 0) {
        setVisible(false);
        shownAtRef.current = null;
      }
    }, remaining);
  }, []);

  const withLoader = useCallback(
    async <T,>(action: () => Promise<T>): Promise<T> => {
      showLoader();
      try {
        return await action();
      } finally {
        hideLoader();
      }
    },
    [showLoader, hideLoader],
  );

  return (
    <PageLoaderContext.Provider value={{ showLoader, hideLoader, withLoader }}>
      {children}
      {visible && <PageLoaderOverlay />}
    </PageLoaderContext.Provider>
  );
}

export function usePageLoader(): PageLoaderContextValue {
  const ctx = useContext(PageLoaderContext);
  if (!ctx) {
    throw new Error('usePageLoader must be used within a PageLoaderProvider');
  }
  return ctx;
}
