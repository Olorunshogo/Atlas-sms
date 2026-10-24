<script setup lang="ts">
import type { CheckoutQuote, PlacedOrder } from '../utils/types';

const shop = useShop();
const router = useRouter();

const zoneId = ref<string>(shop.info.value?.shipping_zones[0]?.id ?? '');
const coupon = ref<string>('');
const points = ref<number>(0);
const email = ref<string>('');
const phone = ref<string>('');
const fullName = ref<string>('');
const line1 = ref<string>('');
const city = ref<string>('');
const state = ref<string>('');
const method = ref<string>('card');
const quote = ref<CheckoutQuote | null>(null);
const error = ref<string>('');
const changedNotice = ref<string>('');
const placing = ref<boolean>(false);
// One key per checkout attempt, so a double click or a retry can't place two orders
const idempotencyKey = ref<string>(crypto.randomUUID());

const currency = computed((): string => shop.info.value?.currency ?? 'NGN');
const guest = computed((): boolean => !shop.account.value);
const zone = computed(
  () =>
    shop.info.value?.shipping_zones.find((z) => z.id === zoneId.value) ?? null,
);
const payOnDeliveryOffered = computed((): boolean => {
  const limit = shop.info.value?.pay_on_delivery_max_minor;
  return (
    Boolean(zone.value?.allows_pay_on_delivery) &&
    limit !== null &&
    limit !== undefined &&
    (quote.value?.total_minor ?? 0) <= limit
  );
});

async function refreshQuote(): Promise<void> {
  if (!zoneId.value) {
    return;
  }
  const result = await shop.call<CheckoutQuote>('/checkout/quote', {
    method: 'POST',
    body: {
      shipping_zone_id: zoneId.value,
      coupon_code: coupon.value || null,
      points: points.value,
      email: email.value || null,
    },
  });
  if (!result.success) {
    error.value = result.message;
    quote.value = null;
    return;
  }
  error.value = '';
  quote.value = result.data;
}

async function placeOrder(): Promise<void> {
  if (!quote.value) {
    return;
  }
  error.value = '';
  changedNotice.value = '';
  placing.value = true;
  const result = await shop.call<PlacedOrder>('/checkout', {
    method: 'POST',
    body: {
      idempotency_key: idempotencyKey.value,
      email: email.value || null,
      phone: phone.value || null,
      full_name: fullName.value || null,
      delivery_address: {
        line1: line1.value,
        city: city.value,
        state: state.value,
      },
      shipping_zone_id: zoneId.value,
      payment_method: method.value,
      coupon_code: coupon.value || null,
      points: points.value,
      expected_total_minor: quote.value.total_minor,
    },
  });
  placing.value = false;
  if (!result.success) {
    const changed =
      result.code === 'TOTAL_CHANGED'
        ? (result.data as unknown as { quote?: CheckoutQuote } | undefined)
            ?.quote
        : undefined;
    if (changed) {
      quote.value = changed;
      changedNotice.value = result.message;
      return;
    }
    error.value = result.message;
    return;
  }
  shop.cartCount.value = 0;
  const query: Record<string, string> = { placed: '1' };
  if (result.data.access_token) {
    query.access_token = result.data.access_token;
  }
  await router.push({ path: `/orders/${result.data.order.id}`, query });
}

watch([zoneId, points], refreshQuote);
await refreshQuote();
useHead({ title: 'Checkout' });
</script>

<template>
  <h1 class="text-3xl font-medium">Checkout</h1>
  <div class="grid grid-cols-1 gap-8 lg:grid-cols-[1fr_22rem]">
    <form class="flex flex-col gap-6" @submit.prevent="placeOrder">
      <fieldset v-if="guest" class="flex flex-col gap-3">
        <legend class="mb-2 font-semibold">Your details</legend>
        <p class="text-ink-secondary text-sm">
          Checking out as a guest.
          <NuxtLink to="/account?next=/checkout" class="underline"
            >Sign in</NuxtLink
          >
          to earn points.
        </p>
        <input
          v-model="fullName"
          required
          placeholder="Full name"
          autocomplete="name"
          class="border-border-soft rounded-full border px-4 py-3"
        />
        <input
          v-model="email"
          required
          type="email"
          placeholder="Email"
          autocomplete="email"
          class="border-border-soft rounded-full border px-4 py-3"
        />
        <input
          v-model="phone"
          required
          type="tel"
          placeholder="Phone"
          autocomplete="tel"
          class="border-border-soft rounded-full border px-4 py-3"
        />
      </fieldset>

      <fieldset class="flex flex-col gap-3">
        <legend class="mb-2 font-semibold">Delivery</legend>
        <input
          v-model="line1"
          required
          placeholder="Street address"
          autocomplete="address-line1"
          class="border-border-soft rounded-full border px-4 py-3"
        />
        <div class="grid grid-cols-2 gap-3">
          <input
            v-model="city"
            required
            placeholder="City"
            autocomplete="address-level2"
            class="border-border-soft rounded-full border px-4 py-3"
          />
          <input
            v-model="state"
            required
            placeholder="State"
            autocomplete="address-level1"
            class="border-border-soft rounded-full border px-4 py-3"
          />
        </div>
        <select
          v-model="zoneId"
          required
          class="border-border-soft rounded-full border px-4 py-3"
          aria-label="Delivery zone"
        >
          <option
            v-for="z in shop.info.value?.shipping_zones ?? []"
            :key="z.id"
            :value="z.id"
          >
            {{ z.name }} ({{ formatMoney(z.base_fee_minor, currency)
            }}<template v-if="z.per_kg_fee_minor">
              + {{ formatMoney(z.per_kg_fee_minor, currency) }}/kg</template
            >)
          </option>
        </select>
      </fieldset>

      <fieldset class="flex flex-col gap-3">
        <legend class="mb-2 font-semibold">Payment</legend>
        <label class="flex items-center gap-2"
          ><input v-model="method" type="radio" value="card" /> Card</label
        >
        <label class="flex items-center gap-2"
          ><input v-model="method" type="radio" value="bank_transfer" /> Bank
          transfer</label
        >
        <label class="flex items-center gap-2"
          ><input v-model="method" type="radio" value="ussd" /> USSD</label
        >
        <label class="flex items-center gap-2"
          ><input v-model="method" type="radio" value="mobile_wallet" /> Mobile
          wallet</label
        >
        <label v-if="payOnDeliveryOffered" class="flex items-center gap-2"
          ><input v-model="method" type="radio" value="pay_on_delivery" /> Pay
          on delivery</label
        >
      </fieldset>

      <fieldset class="flex flex-col gap-3">
        <legend class="mb-2 font-semibold">Discounts</legend>
        <div class="flex gap-3">
          <input
            v-model="coupon"
            placeholder="Coupon code"
            class="border-border-soft flex-1 rounded-full border px-4 py-3"
          />
          <button
            type="button"
            class="border-border-inked rounded-full border px-4 py-3 text-sm font-semibold"
            @click="refreshQuote"
          >
            Apply
          </button>
        </div>
        <label
          v-if="shop.account.value && shop.info.value?.loyalty_enabled"
          class="flex items-center gap-3 text-sm"
        >
          Use points (you have {{ shop.account.value.loyalty_points }})
          <input
            v-model.number="points"
            type="number"
            min="0"
            :max="shop.account.value.loyalty_points"
            class="border-border-soft w-24 rounded-full border px-3 py-2"
          />
        </label>
      </fieldset>

      <p
        v-if="changedNotice"
        class="border-warning bg-warning-soft rounded-3xl border px-5 py-4 text-sm"
        role="status"
      >
        {{ changedNotice }} The new total is shown on the right; place the order
        again to accept it.
      </p>
      <p v-if="error" class="text-danger" role="alert">{{ error }}</p>
      <button
        type="submit"
        class="bg-brand text-ink-inverse rounded-full px-6 py-3 font-semibold"
        :disabled="placing || !quote"
      >
        {{
          placing
            ? 'Placing order...'
            : quote
              ? `Pay ${formatMoney(quote.total_minor, currency)}`
              : 'Place order'
        }}
      </button>
    </form>

    <aside
      v-if="quote"
      class="border-border-soft bg-surface-raised flex h-fit flex-col gap-3 rounded-3xl border p-6"
      data-test="order-summary"
    >
      <h2 class="text-lg font-semibold">Order summary</h2>
      <dl class="flex flex-col gap-2 text-sm">
        <div class="flex justify-between">
          <dt>Items</dt>
          <dd>{{ formatMoney(quote.subtotal_minor, currency) }}</dd>
        </div>
        <div
          v-for="promotion in quote.promotions"
          :key="promotion.promotion_id"
          class="text-success flex justify-between"
        >
          <dt>{{ promotion.name }}</dt>
          <dd>-{{ formatMoney(promotion.discount_minor, currency) }}</dd>
        </div>
        <div
          v-if="quote.points_value_minor"
          class="text-success flex justify-between"
        >
          <dt>{{ quote.points_redeemed }} points</dt>
          <dd>-{{ formatMoney(quote.points_value_minor, currency) }}</dd>
        </div>
        <div class="flex justify-between">
          <dt>Shipping</dt>
          <dd>{{ formatMoney(quote.shipping_minor, currency) }}</dd>
        </div>
        <div class="flex justify-between">
          <dt>Tax</dt>
          <dd>{{ formatMoney(quote.tax_minor, currency) }}</dd>
        </div>
        <div
          class="border-border-soft flex justify-between border-t pt-2 text-base font-semibold"
        >
          <dt>Total</dt>
          <dd>{{ formatMoney(quote.total_minor, currency) }}</dd>
        </div>
      </dl>
      <p v-if="quote.coupon_error" class="text-danger text-sm">
        {{ quote.coupon_error }}
      </p>
    </aside>
  </div>
</template>
