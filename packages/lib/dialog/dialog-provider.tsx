import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  lazy,
  Suspense,
  type ReactNode,
} from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { useLocation } from 'react-router';
import { DialogLoaderOverlay } from './dialog-loader';
import {
  backdropVariants,
  dialogPanelVariants,
  springPanel,
  transitionBase,
} from '../motion/variants';
import type {
  DialogComponent,
  DialogContextValue,
  DialogProps,
  DialogRegistry,
  OpenDialog,
} from './dialog-types';

/*
 * The dialog engine.
 *
 * Solves, once, what every hand-rolled modal in this codebase currently solves badly or not at all: responsive bottom-sheet on mobile and centered panel on desktop, backdrop and Escape dismissal, stacking, focus trapping and restoration, a blocking loading overlay, and auto-close on route change.
 *
 * Never build a bespoke useState-driven modal in a page again.
 * Register the dialog and call triggerDialog.
 */

const DialogContext = createContext<DialogContextValue | null>(null);

export interface DialogProviderProps {
  registry: DialogRegistry;
  children: ReactNode;
}

export function DialogProvider({ registry, children }: DialogProviderProps) {
  const [openDialogs, setOpenDialogs] = useState<OpenDialog[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  /* A dialog that gates the dashboard behind it opts out of dismissal. */
  const [isDismissible, setIsDismissible] = useState<boolean>(true);
  const nextId = useRef<number>(0);
  const location = useLocation();

  /*
   * Lazy components are cached per key so reopening a dialog does not re-trigger Suspense, and so the same component identity is reused across renders.
   */
  const componentCache = useRef<Map<string, DialogComponent>>(new Map());

  const getComponent = useCallback(
    (key: string): DialogComponent | null => {
      const cached = componentCache.current.get(key);
      if (cached) return cached;

      const loader = registry[key];
      if (!loader) return null;

      /* Narrowed here, once - see the note on DialogLoader. */
      const Component = lazy(
        loader as () => Promise<{ default: DialogComponent }>,
      );
      componentCache.current.set(key, Component);
      return Component;
    },
    [registry],
  );

  const triggerDialog = useCallback(
    (key: string, props: DialogProps = {}): void => {
      if (!registry[key]) {
        /*
         * A missing key is a wiring mistake, not a user-facing error.
         * Fail loudly in development rather than silently rendering nothing.
         */
        console.error(
          `[DialogProvider] No dialog registered for key "${key}". Add it to the registry.`,
        );
        return;
      }
      nextId.current += 1;
      setOpenDialogs((current) => [
        ...current,
        { key, props, id: nextId.current },
      ]);
    },
    [registry],
  );

  const closeDialog = useCallback((): void => {
    setIsLoading(false);
    setIsDismissible(true);
    setOpenDialogs((current) => current.slice(0, -1));
  }, []);

  const closeAllDialogs = useCallback((): void => {
    setIsLoading(false);
    setIsDismissible(true);
    setOpenDialogs([]);
  }, []);

  // Route changes should never leave a dialog stranded over the new page
  useEffect(() => {
    closeAllDialogs();
  }, [location.pathname, closeAllDialogs]);

  /* Escape closes the topmost dialog, unless a blocking action is running or the dialog has opted out of dismissal. */
  useEffect(() => {
    if (openDialogs.length === 0) return;

    function handleKeyDown(event: KeyboardEvent): void {
      if (event.key === 'Escape' && !isLoading && isDismissible) closeDialog();
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [openDialogs.length, isLoading, isDismissible, closeDialog]);

  // Lock body scroll while anything is open
  useEffect(() => {
    if (openDialogs.length === 0) return;
    const previous = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = previous;
    };
  }, [openDialogs.length]);

  const value = useMemo<DialogContextValue>(
    () => ({
      triggerDialog,
      closeDialog,
      closeAllDialogs,
      setDialogLoading: setIsLoading,
      setDialogDismissible: setIsDismissible,
      openDialogs,
    }),
    [triggerDialog, closeDialog, closeAllDialogs, openDialogs],
  );

  return (
    <DialogContext.Provider value={value}>
      {children}

      <AnimatePresence>
        {openDialogs.map((dialog, index) => {
          const Component = getComponent(dialog.key);
          if (!Component) return null;

          return (
            <DialogPanel
              key={dialog.id}
              isTop={index === openDialogs.length - 1}
              stackIndex={index}
              onDismiss={isLoading || !isDismissible ? undefined : closeDialog}
            >
              <Suspense fallback={<DialogLoaderOverlay inline />}>
                {/* Props are app-defined; the engine passes them through untouched. */}
                <Component {...dialog.props} />
              </Suspense>
            </DialogPanel>
          );
        })}
      </AnimatePresence>

      {isLoading && <DialogLoaderOverlay />}
    </DialogContext.Provider>
  );
}

/*
 * `onDismiss` is undefined while a blocking action runs, or while the dialog is a gate, either of which disables dismissal.
 */
interface DialogPanelProps {
  isTop: boolean;
  stackIndex: number;
  onDismiss?: () => void;
  children: ReactNode;
}

/*
 * Centered panel at every breakpoint, vertically and horizontally.
 *
 * Was a bottom sheet below md.
 * Centering everywhere is what the admin password dialog needed to stop reading as clipped, and the wrapper's py-6 is the other half of it: max-height alone still let a tall panel sit flush against the viewport edge with nothing to show it was scrollable.
 *
 * The wrapper is pointer-events-none so a click lands on the backdrop underneath it rather than the empty space beside the panel.
 */
function DialogPanel({
  isTop,
  stackIndex,
  onDismiss,
  children,
}: DialogPanelProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const previouslyFocused = useRef<Element | null>(null);

  useEffect(() => {
    previouslyFocused.current = document.activeElement;
    panelRef.current?.focus();

    return () => {
      const target = previouslyFocused.current;
      if (target instanceof HTMLElement) target.focus();
    };
  }, []);

  return (
    <>
      <motion.div
        className={`fixed inset-0 bg-black/50 backdrop-blur-sm ${
          onDismiss ? 'cursor-pointer' : ''
        }`}
        style={{ zIndex: 40 + stackIndex * 10 }}
        variants={backdropVariants}
        initial="initial"
        animate="animate"
        exit="exit"
        transition={transitionBase}
        onClick={onDismiss}
        aria-hidden="true"
      />

      <div
        className="px-section-px sm:px-section-px-sm lg:px-section-px-lg pointer-events-none fixed inset-0 flex items-center justify-center py-6"
        style={{ zIndex: 50 + stackIndex * 10 }}
      >
        <motion.div
          ref={panelRef}
          role="dialog"
          aria-modal={isTop}
          tabIndex={-1}
          /*
           * max-w-150 (37.5rem), not max-w-md.
           *
           * The app theme defines a named spacing scale including --spacing-md: 0.75rem, and in Tailwind v4 max-w-* reads the spacing namespace - so max-w-md compiled to 12px and the panel collapsed to the width of its padding.
           * Numeric widths cannot be shadowed.
           *
           * The admin branch independently reached for max-w-160; this keeps the narrower value already chosen here.
           * Widen if an admin dialog needs the extra room.
           */
          className="border-line pointer-events-auto max-h-[calc(100dvh-3rem)] w-full max-w-150 overflow-y-auto rounded-2xl border bg-white p-6 outline-none"
          variants={dialogPanelVariants}
          initial="initial"
          animate="animate"
          exit="exit"
          transition={springPanel}
        >
          {children}
        </motion.div>
      </div>
    </>
  );
}

/*
 * The context without the throw, for engines that can work without a provider.
 * The table engine uses this: a DataTable with no confirm action must render fine outside a DialogProvider, and only a confirm action needs one.
 */
export function useOptionalDialog(): DialogContextValue | null {
  return useContext(DialogContext);
}

export function useDialog(): DialogContextValue {
  const context = useContext(DialogContext);
  if (!context) {
    throw new Error('useDialog must be used inside <DialogProvider>');
  }
  return context;
}
