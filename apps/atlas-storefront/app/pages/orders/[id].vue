<script setup lang="ts">
import type { OrderView, PaymentStart } from '../../utils/types';

const shop = useShop();
const route = useRoute();
const config = useRuntimeConfig();
const requestFetch = useRequestFetch();

const orderId = computed((): string => String(route.params.id));
const accessToken = computed((): string | undefined =>
  typeof route.query.access_token === 'string'
    ? route.query.access_token
    : undefined,
);
const order = ref<OrderView | null>(null);
const error = ref<string>('');
const notice = ref<string>(
  route.query.placed ? 'Thanks, your order is in.' : '',
);
const retryMethod = ref<string>('card');
const working = ref<boolean>(false);

const currency = computed(
  (): string => order.value?.currency ?? shop.info.value?.currency ?? 'NGN',
);
const pendingPayment = computed(
  () => order.value?.payments.find((p) => p.status === 'pending') ?? null,
);
const lastFailed = computed(
  () =>
    [...(order.value?.payments ?? [])]
      .reverse()
      .find((p) => p.status === 'failed') ?? null,
);

async function load(): Promise<void> {
  const result = await shop.call<OrderView>(`/orders/${orderId.value}`, {
    query: { access_token: accessToken.value },
  });
  order.value = result.success ? result.data : null;
  error.value = result.success ? '' : result.message;
}

async function retry(): Promise<void> {
  working.value = true;
  const result = await shop.call<PaymentStart>(
    `/orders/${orderId.value}/payments`,
    {
      method: 'POST',
      body: {
        method: retryMethod.value,
        idempotency_key: crypto.randomUUID(),
        access_token: accessToken.value ?? null,
      },
    },
  );
  working.value = false;
  if (!result.success) {
    error.value = result.message;
    return;
  }
  await load();
}

// Development only: plays the gateway's confirmation the way the real one would, server to server
async function simulate(status: 'succeeded' | 'failed'): Promise<void> {
  const payment = pendingPayment.value;
  if (!payment?.gateway_ref) {
    return;
  }
  working.value = true;
  await requestFetch('/api/dev/pay', {
    method: 'POST',
    body: { gateway_ref: payment.gateway_ref, status },
    ignoreResponseError: true,
  });
  working.value = false;
  await load();
}

await load();
useHead({
  title: computed((): string =>
    order.value ? `Order ${order.value.order_number}` : 'Order',
  ),
});
</script>

<template>
  <p v-if="!order" class="text-danger">{{ error || 'Order not found.' }}</p>
  <template v-else>
    <div class="flex flex-col gap-1">
      <p v-if="notice" class="text-success font-semibold">{{ notice }}</p>
      <h1 class="text-3xl font-medium">Order {{ order.order_number }}</h1>
      <p class="text-ink-secondary">
        Status:
        <span class="font-semibold" data-test="order-status">{{
          order.status.replace('_', ' ')
        }}</span>
        · Invoice {{ order.invoice_number
        }}<template v-if="order.receipt_number">
          · Receipt {{ order.receipt_number }}</template
        >
      </p>
      <p v-if="accessToken" class="text-ink-muted text-sm">
        Keep this page's address to come back to your order.
      </p>
    </div>

    <section
      v-if="order.status === 'pending_payment'"
      class="border-border-soft flex flex-col gap-3 rounded-3xl border p-6"
    >
      <h2 class="text-lg font-semibold">Payment</h2>
      <template v-if="pendingPayment">
        <p v-if="pendingPayment.method === 'pay_on_delivery'">
          You'll pay {{ formatMoney(order.total_minor, currency) }} when it
          arrives.
        </p>
        <p v-else>
          Waiting for the payment provider to confirm
          {{ formatMoney(pendingPayment.amount_minor, currency) }}.
        </p>
        <a
          v-if="pendingPayment.redirect_url"
          :href="pendingPayment.redirect_url"
          class="w-fit underline"
          target="_blank"
          rel="noopener"
          >Continue to payment</a
        >
        <p v-if="order.payment_expires_at" class="text-ink-muted text-sm">
          Held until {{ new Date(order.payment_expires_at).toLocaleString() }}.
        </p>
        <div
          v-if="
            config.public.mockIntegrations &&
            pendingPayment.method !== 'pay_on_delivery'
          "
          class="flex gap-3 text-sm"
        >
          <span class="text-ink-muted">Mock gateway:</span>
          <button
            type="button"
            class="underline"
            :disabled="working"
            @click="simulate('succeeded')"
          >
            Confirm payment
          </button>
          <button
            type="button"
            class="text-danger underline"
            :disabled="working"
            @click="simulate('failed')"
          >
            Fail payment
          </button>
        </div>
      </template>
      <form v-else class="flex flex-col gap-3" @submit.prevent="retry">
        <p v-if="lastFailed" class="text-danger text-sm">
          The last payment didn't go through{{
            lastFailed.failure_reason ? `: ${lastFailed.failure_reason}` : ''
          }}.
        </p>
        <label class="flex items-center gap-3 text-sm">
          Try again with
          <select
            v-model="retryMethod"
            class="border-border-soft rounded-full border px-3 py-2"
          >
            <option value="card">Card</option>
            <option value="bank_transfer">Bank transfer</option>
            <option value="ussd">USSD</option>
            <option value="mobile_wallet">Mobile wallet</option>
          </select>
        </label>
        <button
          type="submit"
          class="bg-brand text-ink-inverse w-fit rounded-full px-5 py-3 text-sm font-semibold"
          :disabled="working"
        >
          Retry payment
        </button>
      </form>
    </section>

    <section class="flex flex-col gap-3">
      <h2 class="text-lg font-semibold">Items</h2>
      <ul class="flex flex-col gap-2">
        <li
          v-for="line in order.lines"
          :key="line.product_id"
          class="flex justify-between text-sm"
        >
          <span>{{ Number(line.quantity) }} × {{ line.product_name }}</span>
          <span>{{ formatMoney(line.total_minor, currency) }}</span>
        </li>
      </ul>
      <dl class="border-border-soft flex flex-col gap-1 border-t pt-3 text-sm">
        <div class="flex justify-between">
          <dt>Items</dt>
          <dd>{{ formatMoney(order.subtotal_minor, currency) }}</dd>
        </div>
        <div
          v-if="order.discount_minor"
          class="text-success flex justify-between"
        >
          <dt>Discounts</dt>
          <dd>-{{ formatMoney(order.discount_minor, currency) }}</dd>
        </div>
        <div class="flex justify-between">
          <dt>Shipping</dt>
          <dd>{{ formatMoney(order.shipping_minor, currency) }}</dd>
        </div>
        <div class="flex justify-between">
          <dt>Tax</dt>
          <dd>{{ formatMoney(order.tax_minor, currency) }}</dd>
        </div>
        <div class="flex justify-between font-semibold">
          <dt>Total</dt>
          <dd>{{ formatMoney(order.total_minor, currency) }}</dd>
        </div>
      </dl>
      <a
        :href="`/api/shop/orders/${order.id}/invoice?format=text${accessToken ? `&access_token=${accessToken}` : ''}`"
        class="w-fit text-sm underline"
        target="_blank"
        >View invoice</a
      >
    </section>

    <section class="flex flex-col gap-3">
      <h2 class="text-lg font-semibold">Tracking</h2>
      <p v-if="order.shipment" class="text-sm">
        {{ order.shipment.courier }} tracking number
        <span class="font-semibold">{{ order.shipment.tracking_number }}</span>
        ({{ order.shipment.status.replace('_', ' ') }})
      </p>
      <ol class="flex flex-col gap-2 text-sm">
        <li
          v-for="(event, index) in order.events"
          :key="index"
          class="flex gap-3"
        >
          <span class="text-ink-muted w-44 shrink-0">{{
            new Date(event.created_at).toLocaleString()
          }}</span>
          <span
            ><span class="font-semibold">{{
              event.status.replace('_', ' ')
            }}</span
            ><template v-if="event.note"> · {{ event.note }}</template></span
          >
        </li>
      </ol>
    </section>
  </template>
</template>
