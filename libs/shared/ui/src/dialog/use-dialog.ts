import { inject, type InjectionKey } from 'vue';
import type { DialogContextValue } from './dialog-types';

export const DIALOG_KEY: InjectionKey<DialogContextValue> =
  Symbol('atlas-dialog');

export function useOptionalDialog(): DialogContextValue | null {
  return inject(DIALOG_KEY, null);
}

export function useDialog(): DialogContextValue {
  const context = inject(DIALOG_KEY, null);
  if (!context) {
    throw new Error('useDialog must be used inside DialogProvider');
  }
  return context;
}
