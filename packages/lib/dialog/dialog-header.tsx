import { X } from 'lucide-react';
import type { ReactNode } from 'react';

/** `showCloser` is hidden while a submission is in flight or a success panel is showing. */
export interface DialogHeaderProps {
  title: string;
  description?: ReactNode;
  showCloser?: boolean;
  onClose?: () => void;
}

export function DialogHeader({
  title,
  description,
  showCloser = true,
  onClose,
}: DialogHeaderProps) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="flex flex-col gap-1">
        <h2 className="font-syne text-heading text-lg font-bold">{title}</h2>
        {description && <p className="text-body text-sm">{description}</p>}
      </div>

      {showCloser && onClose && (
        <button
          type="button"
          onClick={onClose}
          aria-label="Close dialog"
          className="text-icon-secondary hover:text-heading cursor-pointer transition-colors"
        >
          <X className="h-5 w-5" />
        </button>
      )}
    </div>
  );
}
