export interface AtlasLoaderContext {
  showLoader: () => void;
  hideLoader: () => void;
  withLoader: <T>(action: () => Promise<T>) => Promise<T>;
}
