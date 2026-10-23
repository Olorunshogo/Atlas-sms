<script setup lang="ts">
import { AcceptInviteForm, AuthPanel } from '@atlas/ui-auth';
import { usePageSeo } from '../../../composables/use-page-seo';

usePageSeo({
  title: 'Accept invite',
  description: 'Accept an AtlasSMS staff invite.',
  path: '/auth/invites',
  noIndex: true,
});

const route = useRoute();
const token = computed((): string => {
  return typeof route.params.token === 'string' ? route.params.token : '';
});

async function onAccepted(): Promise<void> {
  await navigateTo('/auth/login');
}
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <AuthPanel
      title="Accept invite"
      description="Set a password for this invite. You will log in next. Accepting does not start a session."
    >
      <AcceptInviteForm :token="token" @accepted="onAccepted" />
    </AuthPanel>
  </main>
</template>
