import tailwindcss from '@tailwindcss/vite';

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2024-11-01',
  devtools: { enabled: true },
  css: ['./app/assets/css/main.css'],
  modules: ['@nuxt/eslint'],

  app: {
    head: {
      title: 'Shop',
      charset: 'utf-8',
      viewport: 'width=device-width, initial-scale=1',
      htmlAttrs: { lang: 'en' },
    },
  },

  runtimeConfig: {
    apiBase: process.env.ATLAS_API_BASE || 'http://localhost:5755',
    // Which store's storefront this deployment serves
    shopHandle: process.env.ATLAS_SHOP_HANDLE || '',
    // Signs the dev-only mock payment and courier callbacks; same value as the backend's
    webhookSecret: process.env.ATLAS_WEBHOOK_SECRET || '',
    public: {
      mockIntegrations: process.env.NUXT_PUBLIC_MOCK_INTEGRATIONS !== 'false',
    },
  },

  components: [{ path: '~/components', pathPrefix: false }],

  vite: {
    plugins: [tailwindcss()],
    optimizeDeps: {
      // axios is @atlas/api's dependency, not this app's, so it resolves through that package
      include: ['@atlas/api > axios'],
    },
  },
});
