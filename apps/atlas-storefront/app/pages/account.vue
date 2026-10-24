<script setup lang="ts">
import type { LoyaltyAccount, Order, Shopper } from '../utils/types';

const shop = useShop();
const route = useRoute();
const router = useRouter();
const config = useRuntimeConfig();

const mode = ref<'login' | 'signup'>('login');
const email = ref<string>('');
const password = ref<string>('');
const fullName = ref<string>('');
const phone = ref<string>('');
const error = ref<string>('');
const working = ref<boolean>(false);
const orders = ref<Order[]>([]);
const loyalty = ref<LoyaltyAccount | null>(null);

const currency = computed((): string => shop.info.value?.currency ?? 'NGN');

async function loadAccountData(): Promise<void> {
  const list = await shop.call<Order[]>('/orders');
  orders.value = list.success ? list.data : [];
  const points = await shop.call<LoyaltyAccount>('/account/loyalty');
  loyalty.value = points.success ? points.data : null;
}

async function signedIn(): Promise<void> {
  await shop.loadAccount();
  await shop.refreshCartCount();
  const next =
    typeof route.query.next === 'string' && route.query.next.startsWith('/')
      ? route.query.next
      : null;
  if (next) {
    await router.push(next);
    return;
  }
  await loadAccountData();
}

async function submit(): Promise<void> {
  error.value = '';
  working.value = true;
  const path = mode.value === 'login' ? '/auth/login' : '/auth/signup';
  const body =
    mode.value === 'login'
      ? { email: email.value, password: password.value }
      : {
          email: email.value,
          password: password.value,
          full_name: fullName.value,
          phone: phone.value || null,
        };
  const result = await shop.call<{ shopper: Shopper }>(path, {
    method: 'POST',
    body,
  });
  working.value = false;
  if (!result.success) {
    error.value = result.message;
    return;
  }
  await signedIn();
}

/**
 * The mock identity provider accepts `mock:{subject}:{email}:{name}`. A real Google or Facebook button
 * would put the provider's ID token here instead.
 */
async function social(provider: 'google' | 'facebook'): Promise<void> {
  error.value = '';
  if (!email.value) {
    error.value =
      'Enter an email first; the mock provider signs you in as that address.';
    return;
  }
  const token = `mock:${provider}-${email.value}:${email.value}:${fullName.value || email.value.split('@')[0]}`;
  const result = await shop.call<{ shopper: Shopper }>('/auth/social', {
    method: 'POST',
    body: { provider, token },
  });
  if (!result.success) {
    error.value = result.message;
    return;
  }
  await signedIn();
}

if (shop.account.value) {
  await loadAccountData();
}
useHead({ title: 'Account' });
</script>

<template>
  <template v-if="shop.account.value">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-medium">{{ shop.account.value.full_name }}</h1>
        <p class="text-ink-secondary">{{ shop.account.value.email }}</p>
      </div>
      <button
        type="button"
        class="border-border-inked rounded-full border px-4 py-2 text-sm font-semibold"
        @click="shop.signOut()"
      >
        Sign out
      </button>
    </div>

    <section
      v-if="loyalty?.enabled"
      class="border-border-soft flex flex-col gap-1 rounded-3xl border p-6"
    >
      <h2 class="text-lg font-semibold">Points</h2>
      <p>
        <span class="text-2xl font-semibold">{{
          loyalty.available_points
        }}</span>
        available (worth
        {{
          formatMoney(
            loyalty.available_points * loyalty.point_value_minor,
            currency,
          )
        }})
      </p>
      <p v-if="loyalty.pending_points" class="text-ink-muted text-sm">
        {{ loyalty.pending_points }} more become available once their orders are
        past the return window.
      </p>
    </section>

    <section class="flex flex-col gap-3">
      <h2 class="text-lg font-semibold">Orders</h2>
      <p v-if="orders.length === 0" class="text-ink-secondary">
        No orders yet.
      </p>
      <ul class="flex flex-col gap-2">
        <li v-for="order in orders" :key="order.id">
          <NuxtLink
            :to="`/orders/${order.id}`"
            class="border-border-soft flex justify-between rounded-3xl border p-4 text-sm"
          >
            <span class="font-semibold">{{ order.order_number }}</span>
            <span>{{ order.status.replace('_', ' ') }}</span>
            <span>{{ formatMoney(order.total_minor, currency) }}</span>
          </NuxtLink>
        </li>
      </ul>
    </section>
  </template>

  <div v-else class="mx-auto flex w-full max-w-112 flex-col gap-6">
    <h1 class="text-3xl font-medium">
      {{ mode === 'login' ? 'Sign in' : 'Create an account' }}
    </h1>
    <form class="flex flex-col gap-3" @submit.prevent="submit">
      <template v-if="mode === 'signup'">
        <input
          v-model="fullName"
          required
          placeholder="Full name"
          autocomplete="name"
          class="border-border-soft rounded-full border px-4 py-3"
        />
        <input
          v-model="phone"
          type="tel"
          placeholder="Phone (optional)"
          autocomplete="tel"
          class="border-border-soft rounded-full border px-4 py-3"
        />
      </template>
      <input
        v-model="email"
        required
        type="email"
        placeholder="Email"
        autocomplete="email"
        class="border-border-soft rounded-full border px-4 py-3"
      />
      <input
        v-model="password"
        required
        type="password"
        minlength="8"
        placeholder="Password"
        :autocomplete="mode === 'login' ? 'current-password' : 'new-password'"
        class="border-border-soft rounded-full border px-4 py-3"
      />
      <p v-if="error" class="text-danger text-sm" role="alert">{{ error }}</p>
      <button
        type="submit"
        class="bg-brand text-ink-inverse rounded-full px-5 py-3 text-sm font-semibold"
        :disabled="working"
      >
        {{ mode === 'login' ? 'Sign in' : 'Create account' }}
      </button>
    </form>
    <div v-if="config.public.mockIntegrations" class="flex flex-col gap-2">
      <p class="text-ink-muted text-sm">Or continue with (mock providers)</p>
      <div class="flex gap-3">
        <button
          type="button"
          class="border-border-inked flex-1 rounded-full border px-4 py-3 text-sm font-semibold"
          @click="social('google')"
        >
          Google
        </button>
        <button
          type="button"
          class="border-border-inked flex-1 rounded-full border px-4 py-3 text-sm font-semibold"
          @click="social('facebook')"
        >
          Facebook
        </button>
      </div>
    </div>
    <button
      type="button"
      class="text-ink-secondary w-fit text-sm underline"
      @click="mode = mode === 'login' ? 'signup' : 'login'"
    >
      {{
        mode === 'login'
          ? 'New here? Create an account'
          : 'Already have an account? Sign in'
      }}
    </button>
  </div>
</template>
