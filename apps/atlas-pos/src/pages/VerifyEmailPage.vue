<script setup lang="ts">
import { IonButton, IonContent, IonPage } from '@ionic/vue';
import { useRouter } from 'vue-router';
import { AppLogo } from '@atlas/ui';
import {
  AuthPanel,
  EmailVerificationBanner,
  VerifyEmailForm,
} from '@atlas/ui-auth';
import { posRequest } from '../auth/api';
import { clearPosSession, posSession } from '../auth/session';
import { usePageSeo } from '../composables/use-page-seo';

usePageSeo({
  title: 'Verify email',
  description: 'Verify the email behind this till session.',
  path: '/auth/verify-email',
  noIndex: true,
});

const router = useRouter();

async function onVerified(): Promise<void> {
  posSession.emailVerified = true;
  await router.replace('/');
}

async function logout(): Promise<void> {
  await posRequest<unknown>('auth/logout', {
    method: 'POST',
    body: { refresh_token: posSession.refreshToken ?? '' },
  });
  clearPosSession();
  await router.replace('/auth/login');
}
</script>

<template>
  <IonPage>
    <IonContent class="ion-padding">
      <div class="flex flex-col gap-6 py-8">
        <AppLogo />
        <EmailVerificationBanner :email="posSession.email" />
        <AuthPanel
          title="Verify your email"
          :description="`Enter the 6-digit code sent to ${posSession.email}. The till opens as soon as it's confirmed.`"
        >
          <VerifyEmailForm :email="posSession.email" @verified="onVerified" />
          <template #footer>
            <IonButton fill="clear" class="w-fit" @click="logout">
              Log out
            </IonButton>
          </template>
        </AuthPanel>
      </div>
    </IonContent>
  </IonPage>
</template>
