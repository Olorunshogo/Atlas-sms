<script setup lang="ts">
import { AppLogo } from '@atlas/ui';
import { useSidebar } from '@atlas/composables';

interface NavLinkItem {
  label: string;
  href: string;
}

const navLinks: NavLinkItem[] = [
  { label: 'Product', href: '/#product' },
  { label: 'Pricing', href: '/#pricing' },
];

const session = useSession();
const { isSidebarOpen, openSidebar, closeSidebar, handleSidebarKeydown } =
  useSidebar('site-menu-close');

onMounted(async () => {
  if (!session.loaded.value) {
    await session.loadSession();
  }
});

watch(isSidebarOpen, (open) => {
  document.body.style.overflow = open ? 'hidden' : '';
});

onBeforeUnmount(() => {
  document.body.style.overflow = '';
});

const signedIn = computed((): boolean => {
  return session.profile.value !== null;
});
</script>

<template>
  <header
    class="border-border-soft bg-surface-raised sticky top-0 z-40 w-full border-b"
  >
    <div
      class="section-max-width px-section-px sm:px-section-px-sm lg:px-section-px-lg flex h-16 items-center justify-between gap-6"
    >
      <NuxtLink to="/" class="w-fit" @click="closeSidebar">
        <AppLogo />
      </NuxtLink>

      <nav class="hidden items-center gap-6 lg:flex">
        <NuxtLink
          v-for="link in navLinks"
          :key="link.href"
          :to="link.href"
          class="text-ink-secondary hover:text-ink-primary text-sm font-semibold"
        >
          {{ link.label }}
        </NuxtLink>
      </nav>

      <div class="hidden items-center gap-3 lg:flex">
        <NuxtLink
          v-if="signedIn"
          to="/dashboard"
          class="bg-brand text-ink-inverse px-4 py-2 text-sm font-semibold"
        >
          Dashboard
        </NuxtLink>
        <template v-else>
          <NuxtLink
            to="/auth/login"
            class="text-ink-primary px-3 py-2 text-sm font-semibold"
          >
            Log in
          </NuxtLink>
          <NuxtLink
            to="/auth/signup"
            class="bg-brand text-ink-inverse px-4 py-2 text-sm font-semibold"
          >
            Sign up
          </NuxtLink>
        </template>
      </div>

      <button
        type="button"
        class="text-ink-primary cursor-pointer p-2 lg:hidden"
        :aria-expanded="isSidebarOpen"
        aria-controls="site-menu"
        :aria-label="isSidebarOpen ? 'Close menu' : 'Open menu'"
        @click="isSidebarOpen ? closeSidebar() : openSidebar()"
      >
        <svg
          v-if="isSidebarOpen"
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          aria-hidden="true"
        >
          <path d="M18 6 6 18" />
          <path d="m6 6 12 12" />
        </svg>
        <svg
          v-else
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          aria-hidden="true"
        >
          <path d="M4 6h16" />
          <path d="M4 12h16" />
          <path d="M4 18h16" />
        </svg>
      </button>
    </div>
  </header>

  <Teleport to="body">
    <div
      class="fixed inset-0 z-50 lg:hidden"
      :class="isSidebarOpen ? '' : 'pointer-events-none'"
    >
      <button
        type="button"
        class="absolute inset-0 cursor-pointer bg-black/60 transition-opacity duration-300"
        :class="isSidebarOpen ? 'opacity-100' : 'opacity-0'"
        aria-label="Close menu"
        tabindex="-1"
        @click="closeSidebar"
      />
      <div
        id="site-menu"
        role="dialog"
        aria-modal="true"
        aria-label="Site menu"
        class="bg-surface-raised absolute top-0 left-0 flex h-full w-full max-w-sm flex-col gap-6 px-6 py-6 shadow-xl transition-transform duration-300 ease-out"
        :class="isSidebarOpen ? 'translate-x-0' : '-translate-x-full'"
        :inert="isSidebarOpen ? undefined : true"
        @keydown="handleSidebarKeydown"
      >
        <div class="flex items-center justify-between">
          <NuxtLink to="/" @click="closeSidebar">
            <AppLogo />
          </NuxtLink>
          <button
            id="site-menu-close"
            type="button"
            class="text-ink-primary cursor-pointer rounded-full p-2"
            aria-label="Close menu"
            @click="closeSidebar"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              aria-hidden="true"
            >
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
        <nav class="flex flex-col gap-4">
          <NuxtLink
            v-for="link in navLinks"
            :key="link.href"
            :to="link.href"
            class="text-ink-primary text-base font-semibold"
            @click="closeSidebar"
          >
            {{ link.label }}
          </NuxtLink>
        </nav>
        <div class="mt-auto flex flex-col gap-3">
          <NuxtLink
            v-if="signedIn"
            to="/dashboard"
            class="bg-brand text-ink-inverse px-4 py-3 text-center text-sm font-semibold"
            @click="closeSidebar"
          >
            Dashboard
          </NuxtLink>
          <template v-else>
            <NuxtLink
              to="/auth/login"
              class="border-border-inked text-ink-primary border px-4 py-3 text-center text-sm font-semibold"
              @click="closeSidebar"
            >
              Log in
            </NuxtLink>
            <NuxtLink
              to="/auth/signup"
              class="bg-brand text-ink-inverse px-4 py-3 text-center text-sm font-semibold"
              @click="closeSidebar"
            >
              Sign up
            </NuxtLink>
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>
