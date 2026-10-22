<script setup lang="ts">
import {
  IonButton,
  IonContent,
  IonInput,
  IonItem,
  IonList,
  IonPage,
  IonSelect,
  IonSelectOption,
} from '@ionic/vue';
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { AppLogo } from '@atlas/ui';
import { isSuccess, posRequest, type TenantChoice } from '../auth/api';
import { rememberSession } from '../auth/session';
import { usePageSeo } from '../composables/use-page-seo';

usePageSeo({
  title: 'Log in',
  description: 'Log in to the AtlasSMS point of sale.',
  path: '/auth/login',
  noIndex: true,
});

const router = useRouter();
const email = ref<string>('');
const password = ref<string>('');
const tenantId = ref<string>('');
const tenants = ref<TenantChoice[]>([]);
const needsTenant = ref<boolean>(false);
const errorMessage = ref<string>('');
const submitting = ref<boolean>(false);

async function submitLogin(): Promise<void> {
  errorMessage.value = '';
  submitting.value = true;
  const result = await posRequest<{
    access_token?: string;
    refresh_token?: string;
    tenant_id: string;
    user: { full_name: string };
  }>('auth/login', {
    method: 'POST',
    auth: false,
    body: {
      email: email.value,
      password: password.value,
      ...(tenantId.value ? { tenant_id: tenantId.value } : {}),
    },
  });
  submitting.value = false;

  if (
    result.status === 409 &&
    !isSuccess(result.body) &&
    result.body.data?.tenants
  ) {
    tenants.value = result.body.data.tenants;
    needsTenant.value = true;
    errorMessage.value = result.body.message;
    return;
  }

  if (
    !isSuccess(result.body) ||
    !result.body.data.access_token ||
    !result.body.data.refresh_token
  ) {
    errorMessage.value = result.body.message;
    return;
  }

  rememberSession({
    accessToken: result.body.data.access_token,
    refreshToken: result.body.data.refresh_token,
    tenantId: result.body.data.tenant_id,
    userName: result.body.data.user.full_name,
  });
  await router.replace('/');
}
</script>

<template>
  <IonPage>
    <IonContent class="ion-padding">
      <form
        class="mx-auto flex w-full max-w-md flex-col gap-4 py-8"
        @submit.prevent="submitLogin"
      >
        <AppLogo show-catchphrase />
        <p class="text-ink-secondary">
          Log in on this till. The access token stays in memory.
        </p>
        <IonList class="flex flex-col gap-3">
          <IonItem>
            <IonInput
              v-model="email"
              type="email"
              label="Email"
              label-placement="stacked"
              required
            />
          </IonItem>
          <IonItem>
            <IonInput
              v-model="password"
              type="password"
              label="Password"
              label-placement="stacked"
              required
            />
          </IonItem>
          <IonItem v-if="needsTenant">
            <IonSelect
              v-model="tenantId"
              label="Tenant"
              label-placement="stacked"
              interface="popover"
            >
              <IonSelectOption
                v-for="tenant in tenants"
                :key="tenant.id"
                :value="tenant.id"
              >
                {{ tenant.name }}
              </IonSelectOption>
            </IonSelect>
          </IonItem>
        </IonList>
        <p v-if="errorMessage" class="text-danger text-sm">
          {{ errorMessage }}
        </p>
        <IonButton type="submit" expand="block" :disabled="submitting">
          {{ submitting ? 'Signing in...' : 'Log in' }}
        </IonButton>
      </form>
    </IonContent>
  </IonPage>
</template>
