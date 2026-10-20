<script setup lang="ts">
import { useSeoMetaConfig } from '@atlas/composables';

useSeoMetaConfig({
  title: 'Atlas',
  description: 'One system for every store: inventory, staff, prices, and sales.',
  ogUrl: 'https://app.atlas.com',
});
</script>

<template>
  <main class="mx-auto flex w-full max-w-5xl flex-col gap-16 px-4 py-16 lg:px-8">
    <section class="flex flex-col gap-6">
      <p class="text-sm font-semibold tracking-widest text-ink-muted uppercase">Store management</p>
      <h1 class="max-w-3xl text-5xl leading-tight font-medium lg:text-6xl">
        Run every branch from one catalogue.
      </h1>
      <p class="max-w-2xl text-lg text-ink-secondary">
        Atlas is for a business with one to twenty stores. Each store keeps its own stock, staff, and prices. Reporting stays shared.
      </p>
      <div class="flex flex-col gap-3 sm:flex-row">
        <NuxtLink to="/signup" class="rounded-full bg-brand px-5 py-3 text-center text-sm font-semibold text-ink-inverse">
          Open your first store
        </NuxtLink>
        <NuxtLink
          to="/login"
          class="rounded-full border border-border-inked px-5 py-3 text-center text-sm font-semibold text-ink-primary"
        >
          Log in
        </NuxtLink>
      </div>
    </section>

    <section id="product" class="grid grid-cols-1 gap-4 lg:grid-cols-3">
      <article class="flex flex-col gap-2 rounded-3xl border border-border-soft bg-surface-raised p-6">
        <h2 class="text-xl font-semibold">Inventory per store</h2>
        <p class="text-ink-secondary">A shared catalogue, with stock and cost that belong to the branch that bought them.</p>
      </article>
      <article class="flex flex-col gap-2 rounded-3xl border border-border-soft bg-surface-raised p-6">
        <h2 class="text-xl font-semibold">Staff by role</h2>
        <p class="text-ink-secondary">Owners invite admins, managers, cashiers, and clerks. Nobody can grant a permission they do not hold.</p>
      </article>
      <article class="flex flex-col gap-2 rounded-3xl border border-border-soft bg-surface-raised p-6">
        <h2 class="text-xl font-semibold">A till that works offline</h2>
        <p class="text-ink-secondary">The point of sale is a separate app. The web app is the office: stores, staff, and the books.</p>
      </article>
    </section>

    <section id="pricing" class="flex flex-col gap-4 rounded-3xl bg-surface-inked p-8 text-ink-inverse">
      <h2 class="text-3xl font-medium">Priced by location</h2>
      <p class="max-w-2xl text-ink-inverse/80">
        Starter, growth, and multi-location plans follow how many stores you run. Sign up creates the business. Billing is attached to that tenant.
      </p>
      <NuxtLink to="/signup" class="w-fit rounded-full bg-surface-canvas px-5 py-3 text-sm font-semibold text-ink-primary">
        Create a tenant
      </NuxtLink>
    </section>
  </main>
</template>
