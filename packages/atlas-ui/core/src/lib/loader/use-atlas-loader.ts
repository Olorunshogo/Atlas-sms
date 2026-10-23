import { inject, type InjectionKey } from 'vue';
import type { AtlasLoaderContext } from './loader-types';

export const ATLAS_LOADER_KEY: InjectionKey<AtlasLoaderContext> =
  Symbol('AtlasLoader');

// Same contract as useAtlasDialog: throws outside an AtlasLoaderProvider.
export function useAtlasLoader(): AtlasLoaderContext {
  const context = inject(ATLAS_LOADER_KEY);
  if (!context) {
    throw new Error(
      'useAtlasLoader() must be used within an AtlasLoaderProvider',
    );
  }
  return context;
}
