<script setup lang="ts">
import { usePageSeo } from '../composables/use-page-seo';
import { useScrollReveal } from '@atlas/ui';

usePageSeo({
  title: 'AtlasSMS',
  description:
    'One system for every store: inventory, staff, prices, and sales.',
  path: '/',
  keywords: ['branches', 'catalogue'],
});

const pageRoot = ref<HTMLElement | null>(null);
useScrollReveal(pageRoot);
</script>

<template>
  <main ref="pageRoot" class="flex w-full flex-col">
    <section data-gsap="reveal" class="w-full">
      <div
        class="section-max-width px-section-px py-section-py sm:px-section-px-sm sm:py-section-py-sm lg:px-section-px-lg lg:py-section-py-lg flex flex-col gap-6"
      >
        <p
          class="text-ink-muted text-sm font-semibold tracking-widest uppercase"
        >
          Store management
        </p>
        <h1 class="max-w-3xl text-5xl leading-tight font-medium lg:text-6xl">
          Run every branch from one catalogue.
        </h1>
        <p class="text-ink-secondary max-w-2xl text-lg">
          Atlas is for a business with one to twenty stores. Each store keeps
          its own stock, staff, and prices. Reporting stays shared.
        </p>
        <div class="flex flex-col gap-3 sm:flex-row">
          <NuxtLink
            to="/auth/signup"
            class="bg-brand text-ink-inverse px-5 py-3 text-center text-sm font-semibold"
          >
            Open your first store
          </NuxtLink>
          <NuxtLink
            to="/auth/login"
            class="border-border-inked text-ink-primary border px-5 py-3 text-center text-sm font-semibold"
          >
            Log in
          </NuxtLink>
        </div>
      </div>
    </section>

    <section
      id="product"
      data-gsap="reveal"
      class="border-border-soft bg-surface-raised w-full border-t"
    >
      <div
        class="section-max-width px-section-px py-section-py sm:px-section-px-sm sm:py-section-py-sm lg:px-section-px-lg lg:py-section-py-lg grid grid-cols-1 gap-4 lg:grid-cols-3"
      >
        <article
          class="border-border-soft bg-surface-canvas flex flex-col gap-2 border p-6"
        >
          <h2 class="text-xl font-semibold">Inventory per store</h2>
          <p class="text-ink-secondary">
            A shared catalogue, with stock and cost that belong to the branch
            that bought them.
          </p>
        </article>
        <article
          class="border-border-soft bg-surface-canvas flex flex-col gap-2 border p-6"
        >
          <h2 class="text-xl font-semibold">Staff by role</h2>
          <p class="text-ink-secondary">
            Owners invite admins, managers, cashiers, and clerks. Nobody can
            grant a permission they do not hold.
          </p>
        </article>
        <article
          class="border-border-soft bg-surface-canvas flex flex-col gap-2 border p-6"
        >
          <h2 class="text-xl font-semibold">A till that works offline</h2>
          <p class="text-ink-secondary">
            The point of sale is a separate app. The web app is the office:
            stores, staff, and the books.
          </p>
        </article>
      </div>
    </section>

    <section
      id="pricing"
      data-gsap="reveal"
      class="bg-surface-inked text-ink-inverse w-full"
    >
      <div
        class="section-max-width px-section-px py-section-py sm:px-section-px-sm sm:py-section-py-sm lg:px-section-px-lg lg:py-section-py-lg flex flex-col gap-4"
      >
        <h2 class="text-3xl font-medium">Priced by location</h2>
        <p class="text-ink-inverse/80 max-w-2xl">
          Starter, growth, and multi-location plans follow how many stores you
          run. Sign up creates the business. Billing is attached to that tenant.
        </p>
        <NuxtLink
          to="/auth/signup"
          class="bg-surface-canvas text-ink-primary w-fit px-5 py-3 text-sm font-semibold"
        >
          Create a tenant
        </NuxtLink>
      </div>
    </section>
  </main>
</template>
