<script setup lang="ts">
import { AuthPanel, LoginForm } from '@atlas/ui-auth';
import { usePageSeo } from '../../composables/use-page-seo';

usePageSeo({
  title: 'Log in',
  description: 'Log in to AtlasSMS.',
  path: '/auth/login',
  noIndex: true,
});

const route = useRoute();
const session = useSession();
const initialEmail = computed((): string => {
  return typeof route.query.email === 'string' ? route.query.email : '';
});

// An unverified account still lands on the dashboard; its layout asks for the code
async function onSuccess(): Promise<void> {
  await session.loadSession();
  await navigateTo('/dashboard');
}
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <AuthPanel title="Log in">
      <LoginForm :initial-email="initialEmail" @success="onSuccess" />
      <template #footer>
        <NuxtLink to="/auth/forgot-password" class="w-fit underline"
          >Forgot password</NuxtLink
        >
        <NuxtLink to="/auth/signup" class="w-fit underline"
          >Create a business</NuxtLink
        >
      </template>
    </AuthPanel>
  </main>
</template>
