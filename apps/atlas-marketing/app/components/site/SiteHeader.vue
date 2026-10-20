<script setup lang="ts">
interface NavLinkItem {
  label: string;
  href: string;
}

const navLinks: NavLinkItem[] = [
  { label: 'Product', href: '/#product' },
  { label: 'Pricing', href: '/#pricing' },
];

const menuOpen = ref<boolean>(false);
const session = useSession();

onMounted(async () => {
  if (!session.loaded.value) {
    await session.loadSession();
  }
});

const signedIn = computed((): boolean => {
  return session.profile.value !== null;
});

function closeMenu(): void {
  menuOpen.value = false;
}

function toggleMenu(): void {
  menuOpen.value = !menuOpen.value;
}

watch(menuOpen, (open: boolean) => {
  if (!import.meta.client) {
    return;
  }
  document.body.style.overflow = open ? 'hidden' : '';
});

function onKeyDown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    closeMenu();
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKeyDown);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeyDown);
  document.body.style.overflow = '';
});
</script>

<template>
  <div class="sticky top-3 z-50 px-4">
    <header
      class="mx-auto flex h-16 w-full max-w-5xl items-center justify-between rounded-full border border-border-soft bg-surface-canvas py-3 pr-3 pl-6 shadow-sm"
    >
      <NuxtLink to="/" class="font-outfit text-lg font-semibold text-ink-primary" @click="closeMenu">
        Atlas
      </NuxtLink>

      <nav class="hidden items-center gap-1 lg:flex">
        <NuxtLink
          v-for="link in navLinks"
          :key="link.href"
          :to="link.href"
          class="rounded-full px-3 py-2 text-sm font-semibold text-ink-primary hover:bg-surface-sunken"
        >
          {{ link.label }}
        </NuxtLink>
      </nav>

      <div class="hidden items-center gap-2 lg:flex">
        <NuxtLink
          v-if="signedIn"
          to="/dashboard"
          class="rounded-full bg-brand px-4 py-2 text-sm font-semibold text-ink-inverse"
        >
          Dashboard
        </NuxtLink>
        <template v-else>
          <NuxtLink
            to="/login"
            class="rounded-full border border-border-inked px-4 py-2 text-sm font-semibold text-ink-primary"
          >
            Log in
          </NuxtLink>
          <NuxtLink
            to="/signup"
            class="rounded-full bg-brand px-4 py-2 text-sm font-semibold text-ink-inverse"
          >
            Sign up
          </NuxtLink>
        </template>
      </div>

      <button
        type="button"
        class="rounded-full p-2 text-ink-primary lg:hidden"
        :aria-expanded="menuOpen"
        aria-controls="site-menu"
        :aria-label="menuOpen ? 'Close menu' : 'Open menu'"
        @click="toggleMenu"
      >
        <span aria-hidden="true">{{ menuOpen ? 'Close' : 'Menu' }}</span>
      </button>
    </header>

    <div
      v-if="menuOpen"
      id="site-menu"
      role="dialog"
      aria-modal="true"
      aria-label="Site menu"
      class="fixed inset-0 z-50 lg:hidden"
    >
      <button
        type="button"
        class="absolute inset-0 bg-surface-inked/60"
        aria-label="Close menu"
        @click="closeMenu"
      />
      <div class="relative flex h-full w-full max-w-sm flex-col gap-6 bg-surface-canvas px-6 py-6">
        <div class="flex items-center justify-between">
          <NuxtLink to="/" class="text-xl font-semibold text-ink-primary" @click="closeMenu">Atlas</NuxtLink>
          <button type="button" class="rounded-full px-3 py-2 text-ink-primary" @click="closeMenu">
            Close
          </button>
        </div>
        <nav class="flex flex-col gap-4">
          <NuxtLink
            v-for="link in navLinks"
            :key="link.href"
            :to="link.href"
            class="text-base font-semibold text-ink-primary"
            @click="closeMenu"
          >
            {{ link.label }}
          </NuxtLink>
        </nav>
        <div class="flex flex-col gap-3">
          <NuxtLink
            v-if="signedIn"
            to="/dashboard"
            class="rounded-full bg-brand px-4 py-3 text-center text-sm font-semibold text-ink-inverse"
            @click="closeMenu"
          >
            Dashboard
          </NuxtLink>
          <template v-else>
            <NuxtLink
              to="/login"
              class="rounded-full border border-border-inked px-4 py-3 text-center text-sm font-semibold text-ink-primary"
              @click="closeMenu"
            >
              Log in
            </NuxtLink>
            <NuxtLink
              to="/signup"
              class="rounded-full bg-brand px-4 py-3 text-center text-sm font-semibold text-ink-inverse"
              @click="closeMenu"
            >
              Sign up
            </NuxtLink>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>
