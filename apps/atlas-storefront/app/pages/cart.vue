<script setup lang="ts">
import type { CartView, WishlistItem } from '../utils/types';

const shop = useShop();
const cart = ref<CartView | null>(null);
const error = ref<string>('');

const currency = computed((): string => shop.info.value?.currency ?? 'NGN');
const changed = computed(
  (): boolean =>
    cart.value?.items.some((item) => item.price_changed || !item.in_stock) ??
    false,
);

function keep(result: {
  success: boolean;
  message: string;
  data?: CartView;
}): void {
  if (!result.success || !result.data) {
    error.value = result.message;
    return;
  }
  error.value = '';
  cart.value = result.data;
  shop.cartCount.value = result.data.items.length;
}

async function load(): Promise<void> {
  const result = await shop.call<CartView>('/cart');
  keep(result.success ? result : { success: false, message: result.message });
}

async function setQuantity(productId: string, quantity: number): Promise<void> {
  if (quantity <= 0) {
    await remove(productId);
    return;
  }
  const result = await shop.call<CartView>(`/cart/items/${productId}`, {
    method: 'PATCH',
    body: { quantity: String(quantity) },
  });
  keep(result.success ? result : { success: false, message: result.message });
}

async function remove(productId: string): Promise<void> {
  const result = await shop.call<CartView>(`/cart/items/${productId}`, {
    method: 'DELETE',
  });
  keep(result.success ? result : { success: false, message: result.message });
}

async function saveForLater(productId: string): Promise<void> {
  const result = await shop.call<WishlistItem[]>(
    `/cart/items/${productId}/move-to-wishlist`,
    { method: 'POST' },
  );
  if (!result.success) {
    error.value = result.message;
    return;
  }
  await load();
}

await load();
useHead({ title: 'Cart' });
</script>

<template>
  <h1 class="text-3xl font-medium">Your cart</h1>
  <p v-if="error" class="text-danger" role="alert">{{ error }}</p>
  <p v-if="cart && cart.items.length === 0" class="text-ink-secondary">
    Your cart is empty.
    <NuxtLink to="/" class="underline">Keep shopping</NuxtLink>
  </p>
  <template v-else-if="cart">
    <p
      v-if="changed"
      class="border-warning bg-warning-soft rounded-3xl border px-5 py-4 text-sm"
      role="status"
    >
      Some items changed since you added them. Check the prices and stock below
      before you check out.
    </p>
    <ul class="flex flex-col gap-3">
      <li
        v-for="item in cart.items"
        :key="item.product_id"
        class="border-border-soft flex flex-col gap-3 rounded-3xl border p-5 sm:flex-row sm:items-center sm:justify-between"
        data-test="cart-item"
      >
        <div class="flex flex-col gap-1">
          <NuxtLink
            :to="`/products/${item.product_id}`"
            class="font-semibold"
            >{{ item.name }}</NuxtLink
          >
          <span class="text-ink-secondary text-sm"
            >{{ formatMoney(item.unit_price_minor, currency) }} each</span
          >
          <span v-if="item.price_changed" class="text-warning text-sm">
            Was {{ formatMoney(item.added_unit_price_minor, currency) }} when
            you added it
          </span>
          <span v-if="!item.in_stock" class="text-danger text-sm"
            >Out of stock</span
          >
        </div>
        <div class="flex items-center gap-3">
          <input
            type="number"
            min="0"
            :value="Number(item.quantity)"
            class="border-border-soft w-20 rounded-full border px-3 py-2"
            :aria-label="`Quantity of ${item.name}`"
            @change="
              setQuantity(
                item.product_id,
                Number(($event.target as HTMLInputElement).value),
              )
            "
          />
          <span class="w-28 text-right font-semibold">{{
            formatMoney(item.line_total_minor, currency)
          }}</span>
          <button
            v-if="shop.account.value"
            type="button"
            class="text-sm underline"
            @click="saveForLater(item.product_id)"
          >
            Save for later
          </button>
          <button
            type="button"
            class="text-danger text-sm underline"
            @click="remove(item.product_id)"
          >
            Remove
          </button>
        </div>
      </li>
    </ul>
    <div class="flex flex-col items-end gap-3">
      <p class="text-lg font-semibold">
        Subtotal {{ formatMoney(cart.subtotal_minor, currency) }}
      </p>
      <p class="text-ink-muted text-sm">
        Discounts, shipping and tax are worked out at checkout.
      </p>
      <NuxtLink
        to="/checkout"
        class="bg-brand text-ink-inverse rounded-full px-6 py-3 text-sm font-semibold"
        >Check out</NuxtLink
      >
    </div>
  </template>
</template>
