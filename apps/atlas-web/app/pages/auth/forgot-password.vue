<script setup lang="ts">
import { AuthPanel, ForgotPasswordForm } from '@atlas/ui-auth';
import { usePageSeo } from '../../composables/use-page-seo';

usePageSeo({
  title: 'Forgot password',
  description: 'Request a password reset for an AtlasSMS account.',
  path: '/auth/forgot-password',
  noIndex: true,
});

async function onSent(email: string): Promise<void> {
  await navigateTo({ path: '/auth/reset-password', query: { email } });
}
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <AuthPanel
      title="Forgot password"
      description="If the email has an account, we send a reset code. The response is the same either way."
    >
      <ForgotPasswordForm @sent="onSent" />
    </AuthPanel>
  </main>
</template>
