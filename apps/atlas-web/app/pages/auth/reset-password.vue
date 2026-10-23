<script setup lang="ts">
import { AuthPanel, ResetPasswordForm } from '@atlas/ui-auth';
import { usePageSeo } from '../../composables/use-page-seo';

usePageSeo({
  title: 'Reset password',
  description: 'Choose a new AtlasSMS password.',
  path: '/auth/reset-password',
  noIndex: true,
});

const route = useRoute();
const initialEmail = computed((): string => {
  return typeof route.query.email === 'string' ? route.query.email : '';
});

async function onReset(email: string): Promise<void> {
  await navigateTo({ path: '/auth/login', query: { email } });
}
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <AuthPanel
      title="Reset password"
      description="This signs you out of every device. Log in again with the new password."
    >
      <ResetPasswordForm :initial-email="initialEmail" @reset="onReset" />
    </AuthPanel>
  </main>
</template>
