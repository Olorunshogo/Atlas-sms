<script setup lang="ts">
import type { CatalogProduct } from '../utils/types';

interface ProductCardProps {
  product: CatalogProduct;
  currency: string;
}

defineProps<ProductCardProps>();
</script>

<template>
  <NuxtLink
    :to="`/products/${product.id}`"
    class="border-border-soft bg-surface-raised hover:border-border-inked flex flex-col gap-3 rounded-3xl border p-5 transition-colors"
    data-test="product-card"
  >
    <div
      class="bg-surface-sunken text-ink-muted flex aspect-square items-center justify-center rounded-2xl text-sm"
    >
      <img
        v-if="typeof product.attributes.image_url === 'string'"
        :src="product.attributes.image_url"
        :alt="product.name"
        class="h-full w-full rounded-2xl object-cover"
      />
      <span v-else>{{ product.name.slice(0, 1) }}</span>
    </div>
    <div class="flex flex-col gap-1">
      <span class="font-semibold">{{ product.name }}</span>
      <span class="flex items-baseline gap-2">
        <span class="font-semibold">{{
          formatMoney(shownPrice(product), currency)
        }}</span>
        <span
          v-if="product.sale_price_minor !== null"
          class="text-ink-muted text-sm line-through"
        >
          {{ formatMoney(product.price_minor, currency) }}
        </span>
      </span>
      <span v-if="!product.in_stock" class="text-danger text-sm"
        >Out of stock</span
      >
    </div>
  </NuxtLink>
</template>
