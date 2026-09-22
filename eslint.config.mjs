import nx from '@nx/eslint-plugin';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';
import vuePlugin from 'eslint-plugin-vue';
import vueParser from 'vue-eslint-parser';
import prettierPlugin from 'eslint-plugin-prettier';
import prettierConfig from 'eslint-config-prettier';

export default [
  ...nx.configs['flat/base'],
  ...nx.configs['flat/typescript'],
  ...nx.configs['flat/javascript'],
  {
    ignores: [
      '**/dist',
      '**/vite.config.*.timestamp*',
      '**/vitest.config.*.timestamp*',
      '**/.nx',
      '**/.nuxt',
      '**/.output',
    ],
  },
  // Base TypeScript configuration
  {
    files: ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.jsx'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    plugins: {
      '@typescript-eslint': tseslint,
      prettier: prettierPlugin,
    },
    rules: {
      // Prettier integration
      'prettier/prettier': 'error',

      /**
       * React/react-hooks rules deliberately not enabled: the frontend is
       * Vue/Nuxt (see docs/repo/ARCHITECTURE.md's Remix -> Nuxt migration
       * decision).
       */

      // TypeScript rules
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-module-boundary-types': 'off',
      '@typescript-eslint/no-non-null-assertion': 'warn',

      // General rules
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'prefer-const': 'error',
      'no-var': 'error',

      // Nx rules
      '@nx/enforce-module-boundaries': [
        'error',
        {
          enforceBuildableLibDependency: true,
          allow: ['^.*/eslint(\\\\.base)?\\\\.config\\\\.[cm]?[jt]s$'],
          /**
           * These constraints are the mechanism behind CONTEXT.md decision
           * 9: "if a second app could plausibly need it, it does not live
           * in an app." Convention alone does not hold that line, so an app
           * declaring its own input component or reaching into another app
           * is a lint failure rather than a code-review catch.
           *
           * The layering runs app -> ui-platform -> ui-core -> theme/util,
           * and nothing depends upward. type:app is absent from every
           * onlyDependOnLibsWithTags list, which is what stops one app from
           * importing another.
           */
          depConstraints: [
            {
              sourceTag: 'type:app',
              onlyDependOnLibsWithTags: [
                'type:ui-platform',
                'type:ui-core',
                'type:util',
                'type:theme',
              ],
            },
            {
              sourceTag: 'type:ui-platform',
              onlyDependOnLibsWithTags: [
                'type:ui-core',
                'type:util',
                'type:theme',
              ],
            },
            {
              sourceTag: 'type:ui-core',
              onlyDependOnLibsWithTags: ['type:util', 'type:theme'],
            },
            {
              sourceTag: 'type:util',
              onlyDependOnLibsWithTags: ['type:util'],
            },
            {
              sourceTag: 'type:theme',
              onlyDependOnLibsWithTags: [],
            },
          ],
        },
      ],
    },
  },
  // Vue SFCs (atlas-web, landing-page, @atlasui-web, @atlasui-app)
  ...vuePlugin.configs['flat/recommended'].map((config) => ({
    ...config,
    files: ['**/*.vue'],
  })),
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tsparser,
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    plugins: {
      '@typescript-eslint': tseslint,
      prettier: prettierPlugin,
    },
    rules: {
      'prettier/prettier': 'error',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      /**
       * Single-word component names are the norm here (e.g.
       * KlassivoxDialogProvider is fine, but so is a plain `web-button.vue`
       * default-exported as `WebButton` per the existing @atlasui-web
       * convention) - don't force multi-word.
       */
      'vue/multi-word-component-names': 'off',
      /**
       * Props are typed via `defineProps<Interface>()` (TS optionality via
       * `?:`), not the Options-API prop-object style this rule assumes - a
       * runtime default isn't meaningful for every optional prop declared
       * this way.
       */
      'vue/require-default-prop': 'off',
    },
  },
  // Prettier config must be last to override other configs
  prettierConfig,
];
