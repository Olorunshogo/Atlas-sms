<script setup lang="ts">
import type { CartView, WishlistItem } from '../utils/types';

const shop = useShop();
const items = ref<WishlistItem[]>([]);
const error = ref<string>('');
const signedOut = ref<boolean>(false);

async function load(): Promise<void> {
  const result = await shop.call<WishlistItem[]>('/wishlist');
  signedOut.value = !result.success;
  items.value = result.success ? result.data : [];
}

async function moveToCart(productId: string): Promise<void> {
  const result = await shop.call<CartView>(
    `/wishlist/${productId}/move-to-cart`,
    { method: 'POST' },
  );
  if (!result.success) {
    error.value = result.message;
    return;
  }
  shop.cartCount.value = result.data.items.length;
  await load();
}

async function remove(productId: string): Promise<void> {
  const result = await shop.call<WishlistItem[]>(`/wishlist/${productId}`, {
    method: 'DELETE',
  });
  if (result.success) {
    items.value = result.data;
  }
}

await load();
useHead({ title: 'Wishlist' });
</script>

<template>
  <h1 class="text-3xl font-medium">Wishlist</h1>
  <p v-if="signedOut" class="text-ink-secondary">
    <NuxtLink to="/account" class="underline">Sign in</NuxtLink> to save things
    for later.
  </p>
  <p v-else-if="items.length === 0" class="text-ink-secondary">
    Nothing saved yet.
  </p>
  <p v-if="error" class="text-danger" role="alert">{{ error }}</p>
  <ul class="flex flex-col gap-3">
    <li
      v-for="item in items"
      :key="item.product_id"
      class="border-border-soft flex items-center justify-between gap-4 rounded-3xl border p-5"
    >
      <NuxtLink :to="`/products/${item.product_id}`" class="font-semibold">{{
        item.name
      }}</NuxtLink>
      <div class="flex gap-3 text-sm">
        <button
          type="button"
          class="bg-brand text-ink-inverse rounded-full px-4 py-2 font-semibold"
          @click="moveToCart(item.product_id)"
        >
          Move to cart
        </button>
        <button
          type="button"
          class="text-danger underline"
          @click="remove(item.product_id)"
        >
          Remove
        </button>
      </div>
    </li>
  </ul>
</template>
