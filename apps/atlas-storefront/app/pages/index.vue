<script setup lang="ts">
import type { CatalogProduct, ShopCategory } from '../utils/types';

const shop = useShop();
const route = useRoute();
const router = useRouter();

const q = ref<string>(typeof route.query.q === 'string' ? route.query.q : '');
const categoryId = ref<string>(
  typeof route.query.category_id === 'string' ? route.query.category_id : '',
);
const sort = ref<string>(
  typeof route.query.sort === 'string' ? route.query.sort : 'name',
);
const inStock = ref<boolean>(route.query.in_stock === 'true');
const products = ref<CatalogProduct[]>([]);
const categories = ref<ShopCategory[]>([]);
const error = ref<string>('');

const currency = computed((): string => shop.info.value?.currency ?? 'NGN');

async function load(): Promise<void> {
  const result = await shop.call<CatalogProduct[]>('/products', {
    query: {
      q: q.value || undefined,
      category_id: categoryId.value || undefined,
      sort: sort.value,
      in_stock: inStock.value || undefined,
    },
  });
  products.value = result.success ? result.data : [];
  error.value = result.success ? '' : result.message;
}

async function apply(): Promise<void> {
  await router.replace({
    query: {
      ...(q.value ? { q: q.value } : {}),
      ...(categoryId.value ? { category_id: categoryId.value } : {}),
      ...(sort.value !== 'name' ? { sort: sort.value } : {}),
      ...(inStock.value ? { in_stock: 'true' } : {}),
    },
  });
  await load();
}

const categoryResult = await shop.call<ShopCategory[]>('/categories');
categories.value = categoryResult.success ? categoryResult.data : [];
await load();
</script>

<template>
  <section class="flex flex-col gap-2">
    <h1 class="text-3xl font-medium">{{ shop.info.value?.store_name }}</h1>
    <p class="text-ink-secondary">
      Order online, pay how you like, and track it to your door.
    </p>
  </section>

  <form
    class="flex flex-col gap-3 md:flex-row md:items-end"
    @submit.prevent="apply"
  >
    <label class="flex flex-1 flex-col gap-1 text-sm">
      <span class="text-ink-secondary font-medium">Search</span>
      <input
        v-model="q"
        type="search"
        placeholder="Name or SKU"
        class="border-border-soft bg-surface-canvas rounded-full border px-4 py-3"
      />
    </label>
    <label class="flex flex-col gap-1 text-sm">
      <span class="text-ink-secondary font-medium">Category</span>
      <select
        v-model="categoryId"
        class="border-border-soft bg-surface-canvas rounded-full border px-4 py-3"
      >
        <option value="">All</option>
        <option
          v-for="category in categories"
          :key="category.id"
          :value="category.id"
        >
          {{ category.name }}
        </option>
      </select>
    </label>
    <label class="flex flex-col gap-1 text-sm">
      <span class="text-ink-secondary font-medium">Sort</span>
      <select
        v-model="sort"
        class="border-border-soft bg-surface-canvas rounded-full border px-4 py-3"
      >
        <option value="name">Name</option>
        <option value="price_asc">Price: low to high</option>
        <option value="price_desc">Price: high to low</option>
        <option value="newest">Newest</option>
      </select>
    </label>
    <label class="flex items-center gap-2 py-3 text-sm">
      <input v-model="inStock" type="checkbox" />
      In stock only
    </label>
    <button
      type="submit"
      class="bg-brand text-ink-inverse rounded-full px-5 py-3 text-sm font-semibold"
    >
      Apply
    </button>
  </form>

  <p v-if="error" class="text-danger">{{ error }}</p>
  <p v-else-if="products.length === 0" class="text-ink-secondary">
    Nothing matches that yet.
  </p>
  <div v-else class="grid grid-cols-2 gap-4 md:grid-cols-3 lg:grid-cols-4">
    <ProductCard
      v-for="product in products"
      :key="product.id"
      :product="product"
      :currency="currency"
    />
  </div>
</template>
