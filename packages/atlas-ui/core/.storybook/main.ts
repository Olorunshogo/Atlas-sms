import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

import type { StorybookConfig } from '@storybook/vue3-vite';

const config: StorybookConfig = {
  stories: [
    '../src/**/*.@(mdx|stories.@(js|ts))',
    '../../web/src/**/*.@(mdx|stories.@(js|ts))',
    '../../app/src/**/*.@(mdx|stories.@(js|ts))',
  ],
  addons: [],
  framework: {
    name: getAbsolutePath('@storybook/vue3-vite'),
    options: {
      builder: {
        viteConfigPath: 'vite.config.mts',
      },
    },
  },
};

function getAbsolutePath(value: string): string {
  return dirname(fileURLToPath(import.meta.resolve(`${value}/package.json`)));
}

export default config;
