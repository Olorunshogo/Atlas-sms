<script setup lang="ts">
import { AuthPanel, SignupForm } from '@atlas/ui-auth';
import { usePageSeo } from '../../composables/use-page-seo';

usePageSeo({
  title: 'Create your business',
  description: 'Create an AtlasSMS business and the first store.',
  path: '/auth/signup',
  noIndex: true,
});

async function onSuccess(email: string): Promise<void> {
  await navigateTo({ path: '/auth/verify-email', query: { email } });
}
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <AuthPanel
      title="Create your business"
      description="Signup opens a new business and makes you the owner. We'll email you a code to verify your address."
    >
      <SignupForm @success="onSuccess" />
      <template #footer>
        <NuxtLink to="/auth/login" class="w-fit underline"
          >Already have an account? Log in</NuxtLink
        >
      </template>
    </AuthPanel>
  </main>
</template>
