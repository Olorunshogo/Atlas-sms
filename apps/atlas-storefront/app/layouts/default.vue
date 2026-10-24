<script setup lang="ts">
const shop = useShop();

await shop.loadInfo();

onMounted(async () => {
  if (!shop.accountLoaded.value) {
    await shop.loadAccount();
  }
  await shop.refreshCartCount();
});

useHead({
  title: computed((): string => shop.info.value?.store_name ?? 'Shop'),
});
</script>

<template>
  <div
    class="bg-surface-canvas font-outfit text-ink-primary flex min-h-screen flex-col"
  >
    <header
      class="border-border-soft bg-surface-raised sticky top-0 z-40 w-full border-b"
    >
      <div
        class="section-max-width px-section-px sm:px-section-px-sm lg:px-section-px-lg flex h-16 items-center justify-between gap-6"
      >
        <NuxtLink to="/" class="text-lg font-semibold">
          {{ shop.info.value?.store_name ?? 'Shop' }}
        </NuxtLink>
        <nav class="flex items-center gap-5 text-sm font-semibold">
          <NuxtLink to="/" class="text-ink-secondary hover:text-ink-primary"
            >Shop</NuxtLink
          >
          <NuxtLink
            v-if="shop.account.value"
            to="/wishlist"
            class="text-ink-secondary hover:text-ink-primary"
            >Wishlist</NuxtLink
          >
          <NuxtLink
            to="/cart"
            class="text-ink-secondary hover:text-ink-primary"
          >
            Cart<span v-if="shop.cartCount.value > 0" data-test="cart-count">
              ({{ shop.cartCount.value }})</span
            >
          </NuxtLink>
          <NuxtLink
            to="/account"
            class="text-ink-secondary hover:text-ink-primary"
          >
            {{
              shop.account.value
                ? shop.account.value.full_name.split(' ')[0]
                : 'Sign in'
            }}
          </NuxtLink>
        </nav>
      </div>
    </header>
    <main
      class="section-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg flex w-full flex-1 flex-col gap-8"
    >
      <p v-if="!shop.info.value" class="text-danger">
        This shop isn't available right now.
      </p>
      <slot v-else />
    </main>
    <footer
      class="border-border-soft text-ink-muted border-t px-6 py-6 text-center text-sm"
    >
      {{ shop.info.value?.store_name }} · Prices in
      {{ shop.info.value?.currency ?? 'NGN' }}
      <span v-if="shop.info.value?.prices_include_tax">, tax included</span>
    </footer>
  </div>
</template>
