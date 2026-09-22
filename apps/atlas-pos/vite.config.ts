import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    // Ionic's full component library is genuinely >500KB minified on its
    // own - isolated into its own vendor-ionic chunk below rather than
    // mixed into app code.
    chunkSizeWarningLimit: 1100,
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-ionic': ['@ionic/vue', '@ionic/vue-router'],
          'vendor-capacitor': ['@capacitor/core'],
          'vendor-vue': ['vue', 'vue-router'],
        },
      },
    },
  },
});
