import { inject, type InjectionKey } from 'vue';
import type { AtlasDialogContext } from './dialog-types';

export const ATLAS_DIALOG_KEY: InjectionKey<AtlasDialogContext> =
  Symbol('AtlasDialog');

// Throws outside an AtlasDialogProvider, same contract as the reference source's dialog hook.
export function useAtlasDialog(): AtlasDialogContext {
  const context = inject(ATLAS_DIALOG_KEY);
  if (!context) {
    throw new Error(
      'useAtlasDialog() must be called within an AtlasDialogProvider',
    );
  }
  return context;
}
