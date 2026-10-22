import { cva, type VariantProps } from 'class-variance-authority';

export const toastVariants = cva(
  'pointer-events-auto flex w-full items-start gap-3 rounded-xl border p-4 shadow-lg',
  {
    variants: {
      variant: {
        default: 'border-border bg-surface-canvas text-ink-primary',
        success: 'border-transparent bg-success-soft text-success',
        warning: 'border-transparent bg-warning-soft text-warning',
        destructive: 'border-transparent bg-danger-soft text-danger',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  },
);

export type ToastVariants = VariantProps<typeof toastVariants>;
