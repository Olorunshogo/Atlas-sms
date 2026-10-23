import { defineConfig } from 'vitest/config';
import { nxViteTsPaths } from '@nx/vite/plugins/nx-tsconfig-paths.plugin';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  plugins: [vue(), nxViteTsPaths()],
  test: {
    environment: 'jsdom',
    globals: true,
    watch: false,
  },
});
