import tailwindcss from '@tailwindcss/vite';

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2024-11-01',
  devtools: { enabled: true },
  css: ['./app/assets/css/main.css'],
  modules: ['@nuxt/eslint', '@pinia/nuxt'],

  app: {
    head: {
      title: 'Atlas',
      charset: 'utf-8',
      viewport: 'width=device-width, initial-scale=1',
      htmlAttrs: { lang: 'en' },
    },
  },

  runtimeConfig: {
    // server-only: base URL of atlas-backend (Rust/Axum). Never exposed to the client.
    apiBase: process.env.ATLAS_API_BASE || 'http://localhost:5755',
    public: {
      seo: {
        siteUrl: process.env.NUXT_PUBLIC_SITE_URL || 'https://app.atlas.com',
        siteName: 'Atlas',
        defaultOgImage: '/og-image.png',
        ogImageAlt: 'Atlas - multi-store management platform',
        baseKeywords: ['atlas', 'store management dashboard'],
      },
    },
  },

  nitro: {
    preset: 'cloudflare-pages',
  },

  vite: {
    plugins: [tailwindcss()],
  },
});
