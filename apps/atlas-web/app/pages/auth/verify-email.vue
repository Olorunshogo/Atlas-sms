<script setup lang="ts">
import { AuthPanel, VerifyEmailForm } from '@atlas/ui-auth';
import { usePageSeo } from '../../composables/use-page-seo';

usePageSeo({
  title: 'Verify email',
  description: 'Confirm an AtlasSMS email address.',
  path: '/auth/verify-email',
  noIndex: true,
});

const route = useRoute();
const email = computed((): string => {
  return typeof route.query.email === 'string' ? route.query.email : '';
});

async function onVerified(verifiedEmail: string): Promise<void> {
  await navigateTo({ path: '/auth/login', query: { email: verifiedEmail } });
}
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <AuthPanel
      title="Verify email"
      :description="`Enter the code sent to ${email || 'your email'}.`"
    >
      <VerifyEmailForm :email="email" @verified="onVerified" />
      <template #footer>
        <NuxtLink
          :to="{ path: '/auth/login', query: email ? { email } : {} }"
          class="w-fit underline"
          >Verify later and log in</NuxtLink
        >
      </template>
    </AuthPanel>
  </main>
</template>
