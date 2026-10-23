<script setup lang="ts">
import { IonContent, IonPage } from '@ionic/vue';
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { AppLogo } from '@atlas/ui';
import { AuthPanel, LoginForm } from '@atlas/ui-auth';
import type { IssuedSession } from '../auth/api';
import { rememberSession } from '../auth/session';
import { usePageSeo } from '../composables/use-page-seo';

usePageSeo({
  title: 'Log in',
  description: 'Log in to the AtlasSMS point of sale.',
  path: '/auth/login',
  noIndex: true,
});

const router = useRouter();
const errorMessage = ref<string>('');

async function onSuccess(issued: IssuedSession): Promise<void> {
  errorMessage.value = '';
  if (!rememberSession(issued)) {
    errorMessage.value = 'The server did not return a till session. Try again.';
    return;
  }
  await router.replace(issued.user.email_verified ? '/' : '/auth/verify-email');
}
</script>

<template>
  <IonPage>
    <IonContent class="ion-padding">
      <div class="flex flex-col gap-6 py-8">
        <AppLogo show-catchphrase />
        <AuthPanel
          title="Log in"
          description="Log in on this till. The access token stays in memory."
        >
          <LoginForm @success="onSuccess" />
          <p v-if="errorMessage" class="text-danger text-sm">
            {{ errorMessage }}
          </p>
        </AuthPanel>
      </div>
    </IonContent>
  </IonPage>
</template>
