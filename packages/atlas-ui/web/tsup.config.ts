import { defineConfig } from 'tsup';
import vuePlugin from 'unplugin-vue/esbuild';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['cjs', 'esm'],
  dts: false,
  minify: true,
  external: ['vue', '@atlas/ui-core'],
  esbuildPlugins: [vuePlugin()],
  tsconfig: 'tsconfig.lib.json',
});
