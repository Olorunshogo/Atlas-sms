import tailwindcss from '@tailwindcss/vite';

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2024-11-01',
  devtools: { enabled: true },
  css: ['./app/assets/css/main.css', 'lenis/dist/lenis.css'],
  modules: ['@nuxt/eslint', '@pinia/nuxt', 'nuxt-og-image'],

  app: {
    head: {
      title: 'AtlasSMS',
      charset: 'utf-8',
      viewport: 'width=device-width, initial-scale=1',
      htmlAttrs: { lang: 'en' },
      meta: [{ name: 'author', content: 'AtlasSMS Team' }],
      link: [
        { rel: 'icon', type: 'image/x-icon', href: '/favicon.ico' },
        {
          rel: 'icon',
          type: 'image/png',
          sizes: '32x32',
          href: '/favicon-32x32.png',
        },
        { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' },
      ],
    },
  },

  runtimeConfig: {
    apiBase: process.env.ATLAS_API_BASE || 'http://localhost:5755',
    public: {
      seo: {
        siteUrl: process.env.NUXT_PUBLIC_SITE_URL || 'https://app.atlas.com',
        siteName: 'AtlasSMS',
      },
    },
  },

  components: [{ path: '~/components', pathPrefix: false }],

  nitro: {
    preset: 'cloudflare-pages',
  },

  site: {
    url: process.env.NUXT_PUBLIC_SITE_URL || 'https://app.atlas.com',
  },

  ogImage: {
    defaults: {
      width: 1200,
      height: 630,
    },
  },

  vite: {
    plugins: [tailwindcss()],
    optimizeDeps: {
      include: ['vee-validate', 'zod'],
    },
  },
});
