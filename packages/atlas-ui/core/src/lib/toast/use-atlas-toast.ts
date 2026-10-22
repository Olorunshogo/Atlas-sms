import { inject, type InjectionKey } from 'vue';
import type { AtlasToastContext } from './toast-types';

export const ATLAS_TOAST_KEY: InjectionKey<AtlasToastContext> =
  Symbol('AtlasToast');

// Same contract as useAtlasDialog: throws outside an AtlasToastProvider.
export function useAtlasToast(): AtlasToastContext {
  const context = inject(ATLAS_TOAST_KEY);
  if (!context) {
    throw new Error(
      'useAtlasToast() must be called within an AtlasToastProvider',
    );
  }
  return context;
}
