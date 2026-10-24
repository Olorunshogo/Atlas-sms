<script setup lang="ts">
import type { CatalogProduct } from '../utils/types';

interface ProductRowProps {
  title: string;
  products: CatalogProduct[];
  currency: string;
}

defineProps<ProductRowProps>();
</script>

<template>
  <section v-if="products.length > 0" class="flex flex-col gap-4">
    <h2 class="text-xl font-semibold">{{ title }}</h2>
    <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
      <ProductCard
        v-for="product in products"
        :key="product.id"
        :product="product"
        :currency="currency"
      />
    </div>
  </section>
</template>
