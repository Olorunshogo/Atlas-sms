export type ToastVariant = 'default' | 'success' | 'warning' | 'destructive';

export interface ToastOptions {
  title: string;
  description?: string;
  variant?: ToastVariant;
  duration?: number;
}

export interface ToastEntry extends ToastOptions {
  id: string;
}

export interface AtlasToastContext {
  toast: (options: ToastOptions) => string;
  dismiss: (id: string) => void;
}
