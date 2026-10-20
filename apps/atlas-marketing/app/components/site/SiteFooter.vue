<script setup lang="ts">
interface FooterLink {
  label: string;
  to: string;
}

const quickLinks: FooterLink[] = [
  { label: 'Product', to: '/#product' },
  { label: 'Pricing', to: '/#pricing' },
  { label: 'Sign up', to: '/signup' },
  { label: 'Log in', to: '/login' },
];

const supportEmail = 'support@atlas.app';

const year = computed((): number => {
  return new Date().getFullYear();
});
</script>

<template>
  <footer class="relative overflow-hidden bg-surface-inked text-ink-inverse">
    <p
      aria-hidden="true"
      class="pointer-events-none absolute inset-x-0 top-0 text-center text-7xl font-extrabold tracking-normal text-ink-inverse/10 select-none lg:text-9xl"
    >
      Atlas
    </p>

    <div class="relative z-10 mx-auto flex w-full max-w-5xl flex-col gap-10 px-4 py-16 lg:px-8">
      <div class="grid grid-cols-1 gap-10 lg:grid-cols-3">
        <h2 class="text-4xl leading-tight font-medium lg:text-5xl">
          Every store.
          <span class="block">One set of books.</span>
        </h2>

        <nav class="flex flex-col gap-4">
          <h3 class="text-sm font-semibold tracking-widest uppercase">Quick links</h3>
          <NuxtLink
            v-for="link in quickLinks"
            :key="link.to"
            :to="link.to"
            class="w-fit text-base text-ink-inverse hover:text-ink-inverse/80"
          >
            {{ link.label }}
          </NuxtLink>
        </nav>

        <div class="flex flex-col gap-4">
          <h3 class="text-sm font-semibold tracking-widest uppercase">Need help?</h3>
          <a :href="`mailto:${supportEmail}`" class="w-fit text-base underline decoration-ink-inverse/40 underline-offset-4">
            {{ supportEmail }}
          </a>
          <p class="text-sm text-ink-inverse/80">Weekdays, 9:00 to 17:00 WAT.</p>
        </div>
      </div>

      <div class="flex flex-col gap-3 border-t border-ink-inverse/10 pt-6 text-sm lg:flex-row lg:items-center lg:justify-between">
        <p>© {{ year }} Atlas. Store management for businesses with more than one branch.</p>
        <div class="flex items-center gap-4">
          <NuxtLink to="/login" class="hover:text-ink-inverse/80">Log in</NuxtLink>
          <NuxtLink to="/signup" class="hover:text-ink-inverse/80">Sign up</NuxtLink>
        </div>
      </div>
    </div>
  </footer>
</template>
