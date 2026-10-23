<script setup lang="ts">
import { IonApp, IonRouterOutlet } from '@ionic/vue';
import { watch } from 'vue';
import { useRouter } from 'vue-router';
import { posSession, signedIn } from './auth/session';

const router = useRouter();

// A tenant call refused with EMAIL_NOT_VERIFIED flips this mid-session
watch(
  () => posSession.emailVerified,
  async (verified: boolean) => {
    if (!verified && signedIn()) {
      await router.replace('/auth/verify-email');
    }
  },
);
</script>

<template>
  <IonApp>
    <IonRouterOutlet />
  </IonApp>
</template>
