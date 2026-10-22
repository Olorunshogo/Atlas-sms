<script setup lang="ts">
import { ShoppingCart } from 'lucide-vue-next';
import { Badge, EmptyState, PrimaryButton, SearchField } from '@atlas/ui-core';

interface SaleSessionProps {
  storeName?: string;
  sessionDetail?: string;
  online?: boolean;
}

const props = withDefaults(defineProps<SaleSessionProps>(), {
  storeName: 'Main store',
  sessionDetail: 'Register session',
  online: true,
});

const query = defineModel<string>('query', { default: '' });
</script>

<template>
  <section class="bg-navy text-ink-inverse rounded-card flex flex-col gap-5 p-5">
    <header class="flex items-start justify-between gap-3">
      <div class="flex flex-col gap-1">
        <h2 class="text-lg font-semibold">New sale</h2>
        <p class="text-ink-inverse/70 text-sm">
          {{ props.storeName }} · {{ props.sessionDetail }}
        </p>
      </div>
      <Badge v-if="online" variant="success">Online</Badge>
    </header>

    <SearchField
      v-model="query"
      tone="inverse"
      label="Find a product"
      placeholder="Scan barcode or search by name / SKU"
    />

    <div class="flex items-center justify-between gap-3">
      <h3 class="font-semibold">Cart</h3>
      <p class="text-ink-inverse/70 text-sm">0 items</p>
    </div>

    <EmptyState
      title="Your cart is empty"
      description="Search or scan a product to begin the sale."
      :icon="ShoppingCart"
    />

    <div class="flex items-end justify-between gap-4">
      <div class="flex flex-col gap-1">
        <p class="text-ink-inverse/70 text-sm">Total due</p>
        <p class="tabular-figures text-3xl font-semibold">&#8358;0.00</p>
      </div>
      <PrimaryButton label="Charge customer" />
    </div>
  </section>
</template>
