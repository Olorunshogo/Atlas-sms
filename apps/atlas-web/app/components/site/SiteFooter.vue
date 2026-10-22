<script setup lang="ts">
import { AppLogo } from '@atlas/ui';
interface FooterLink {
  label: string;
  to: string;
}

const productLinks: FooterLink[] = [
  { label: 'Product', to: '/#product' },
  { label: 'Pricing', to: '/#pricing' },
];

const accountLinks: FooterLink[] = [
  { label: 'Sign up', to: '/auth/signup' },
  { label: 'Log in', to: '/auth/login' },
  { label: 'Forgot password', to: '/auth/forgot-password' },
];

const supportEmail = 'support@atlas.app';

const year = computed((): number => {
  return new Date().getFullYear();
});
</script>

<template>
  <footer
    class="border-border-soft bg-surface-raised text-ink-primary w-full border-t"
  >
    <div
      class="section-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg grid grid-cols-1 gap-10 lg:grid-cols-4"
    >
      <div class="flex flex-col gap-3">
        <AppLogo show-catchphrase />
        <p class="text-ink-secondary max-w-xs text-sm">
          Store management for a business with one to twenty branches. Stock,
          staff, and prices stay with the store. Reporting stays shared.
        </p>
      </div>

      <nav class="flex flex-col gap-3">
        <h2 class="text-sm font-semibold">Product</h2>
        <NuxtLink
          v-for="link in productLinks"
          :key="link.to"
          :to="link.to"
          class="text-ink-secondary hover:text-ink-primary w-fit text-sm"
        >
          {{ link.label }}
        </NuxtLink>
      </nav>

      <nav class="flex flex-col gap-3">
        <h2 class="text-sm font-semibold">Account</h2>
        <NuxtLink
          v-for="link in accountLinks"
          :key="link.to"
          :to="link.to"
          class="text-ink-secondary hover:text-ink-primary w-fit text-sm"
        >
          {{ link.label }}
        </NuxtLink>
      </nav>

      <div class="flex flex-col gap-3">
        <h2 class="text-sm font-semibold">Support</h2>
        <a
          :href="`mailto:${supportEmail}`"
          class="text-ink-secondary decoration-border-firm hover:text-ink-primary w-fit text-sm underline underline-offset-4"
        >
          {{ supportEmail }}
        </a>
        <p class="text-ink-secondary text-sm">Weekdays, 9:00 to 17:00 WAT.</p>
      </div>
    </div>

    <div
      class="section-max-width border-border-soft px-section-px text-ink-muted sm:px-section-px-sm lg:px-section-px-lg flex flex-col gap-2 border-t py-6 text-sm lg:flex-row lg:items-center lg:justify-between"
    >
      <p>© {{ year }} AtlasSMS.</p>
      <p>One catalogue across every store.</p>
    </div>
  </footer>
</template>
