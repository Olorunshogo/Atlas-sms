<script setup lang="ts">
import type {
  CartView,
  CatalogProduct,
  Recommendations,
  WishlistItem,
} from '../../utils/types';

const shop = useShop();
const route = useRoute();
const productId = computed((): string => String(route.params.id));

const product = ref<CatalogProduct | null>(null);
const recommendations = ref<Recommendations | null>(null);
const quantity = ref<number>(1);
const message = ref<string>('');
const error = ref<string>('');
const adding = ref<boolean>(false);
const wished = ref<boolean>(false);

const currency = computed((): string => shop.info.value?.currency ?? 'NGN');

async function load(): Promise<void> {
  const result = await shop.call<CatalogProduct>(
    `/products/${productId.value}`,
  );
  product.value = result.success ? result.data : null;
  error.value = result.success ? '' : result.message;
  const recs = await shop.call<Recommendations>(
    `/products/${productId.value}/recommendations`,
  );
  recommendations.value = recs.success ? recs.data : null;
}

async function addToCart(): Promise<void> {
  message.value = '';
  error.value = '';
  adding.value = true;
  const result = await shop.call<CartView>('/cart/items', {
    method: 'POST',
    body: { product_id: productId.value, quantity: String(quantity.value) },
  });
  adding.value = false;
  if (!result.success) {
    error.value = result.message;
    return;
  }
  shop.cartCount.value = result.data.items.length;
  message.value = 'Added to your cart.';
}

async function toggleWish(): Promise<void> {
  const result = await shop.call<WishlistItem[]>(
    `/wishlist/${productId.value}`,
    { method: wished.value ? 'DELETE' : 'PUT' },
  );
  if (result.success) {
    wished.value = result.data.some(
      (item) => item.product_id === productId.value,
    );
  }
}

await load();
watch(productId, load);

onMounted(async () => {
  if (shop.account.value) {
    const list = await shop.call<WishlistItem[]>('/wishlist');
    wished.value =
      list.success &&
      list.data.some((item) => item.product_id === productId.value);
  }
});

useHead({ title: computed((): string => product.value?.name ?? 'Product') });
</script>

<template>
  <p v-if="!product" class="text-danger">{{ error || 'Product not found.' }}</p>
  <template v-else>
    <section class="grid grid-cols-1 gap-8 md:grid-cols-2">
      <div
        class="bg-surface-sunken text-ink-muted flex aspect-square items-center justify-center rounded-3xl text-4xl"
      >
        <img
          v-if="typeof product.attributes.image_url === 'string'"
          :src="product.attributes.image_url"
          :alt="product.name"
          class="h-full w-full rounded-3xl object-cover"
        />
        <span v-else>{{ product.name.slice(0, 1) }}</span>
      </div>
      <div class="flex flex-col gap-4">
        <h1 class="text-3xl font-medium">{{ product.name }}</h1>
        <p v-if="product.sku" class="text-ink-muted text-sm">
          SKU {{ product.sku }}
        </p>
        <p class="flex items-baseline gap-3 text-2xl font-semibold">
          {{ formatMoney(shownPrice(product), currency) }}
          <span
            v-if="product.sale_price_minor !== null"
            class="text-ink-muted text-base line-through"
          >
            {{ formatMoney(product.price_minor, currency) }}
          </span>
        </p>
        <p
          v-if="typeof product.attributes.description === 'string'"
          class="text-ink-secondary"
        >
          {{ product.attributes.description }}
        </p>
        <p
          :class="product.in_stock ? 'text-success' : 'text-danger'"
          class="text-sm font-semibold"
        >
          {{
            product.in_stock
              ? `${Number(product.available)} available`
              : 'Out of stock'
          }}
        </p>
        <form
          v-if="product.in_stock"
          class="flex items-center gap-3"
          @submit.prevent="addToCart"
        >
          <input
            v-model.number="quantity"
            type="number"
            min="1"
            :max="Number(product.available)"
            class="border-border-soft bg-surface-canvas w-24 rounded-full border px-4 py-3"
            aria-label="Quantity"
          />
          <button
            type="submit"
            class="bg-brand text-ink-inverse rounded-full px-5 py-3 text-sm font-semibold"
            :disabled="adding"
          >
            {{ adding ? 'Adding...' : 'Add to cart' }}
          </button>
        </form>
        <button
          v-if="shop.account.value"
          type="button"
          class="w-fit text-sm font-semibold underline"
          @click="toggleWish"
        >
          {{ wished ? 'Remove from wishlist' : 'Save to wishlist' }}
        </button>
        <p v-if="message" class="text-success text-sm">
          {{ message }}
          <NuxtLink to="/cart" class="underline">View cart</NuxtLink>
        </p>
        <p v-if="error" class="text-danger text-sm" role="alert">{{ error }}</p>
      </div>
    </section>

    <template v-if="recommendations">
      <ProductRow
        title="Frequently bought together"
        :products="recommendations.frequently_bought_together"
        :currency="currency"
      />
      <ProductRow
        title="You might also like"
        :products="recommendations.related"
        :currency="currency"
      />
      <ProductRow
        title="Recently viewed"
        :products="recommendations.recently_viewed"
        :currency="currency"
      />
    </template>
  </template>
</template>
