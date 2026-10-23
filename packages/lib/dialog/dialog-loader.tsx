import { Loader2 } from 'lucide-react';

/*
 * `inline` renders inside a panel (for Suspense while a lazy dialog loads).
 * Otherwise it covers the viewport, blocking interaction during a submission.
 */
export interface DialogLoaderOverlayProps {
  inline?: boolean;
  label?: string;
}

export function DialogLoaderOverlay({
  inline = false,
  label = 'Loading',
}: DialogLoaderOverlayProps) {
  if (inline) {
    return (
      <div className="flex items-center justify-center py-10">
        <Loader2
          className="text-primary h-6 w-6 animate-spin"
          aria-label={label}
        />
      </div>
    );
  }

  return (
    <div
      className="fixed inset-0 z-90 flex items-center justify-center bg-black/30 backdrop-blur-sm"
      role="status"
      aria-live="polite"
    >
      <Loader2 className="h-8 w-8 animate-spin text-white" aria-label={label} />
    </div>
  );
}
