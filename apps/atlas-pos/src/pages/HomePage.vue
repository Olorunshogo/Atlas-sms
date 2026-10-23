<script setup lang="ts">
import {
  IonButton,
  IonContent,
  IonHeader,
  IonPage,
  IonToolbar,
} from '@ionic/vue';
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { AppLogo } from '@atlas/ui';
import { SaleSession } from '@atlas/ui-app';
import { usePageSeo } from '../composables/use-page-seo';
import {
  isSuccess,
  posRequest,
  type SessionProfile,
  type StoreRecord,
  type TenantChoice,
} from '../auth/api';
import { clearPosSession, posSession, rememberSession } from '../auth/session';

const router = useRouter();
usePageSeo({
  title: 'POS',
  description: 'AtlasSMS point of sale session.',
  path: '/',
  noIndex: true,
});
const profile = ref<SessionProfile | null>(null);
const stores = ref<StoreRecord[]>([]);
const platformTenants = ref<TenantChoice[]>([]);
const inviteEmail = ref<string>('');
const message = ref<string>('');
const errorMessage = ref<string>('');

const tenantOptions = ref<TenantChoice[]>([]);

const selectedStoreName = computed((): string => {
  const selected = stores.value.find((store) => {
    return store.id === posSession.storeId;
  });
  return selected?.name ?? 'Main store';
});

onMounted(async () => {
  await loadHome();
});

async function loadHome(): Promise<void> {
  const session = await posRequest<SessionProfile>('auth/me');
  if (!isSuccess(session.body)) {
    clearPosSession();
    await router.replace('/auth/login');
    return;
  }
  profile.value = session.body.data;

  const storeList = await posRequest<StoreRecord[]>('stores');
  stores.value = isSuccess(storeList.body) ? storeList.body.data : [];

  if (profile.value.is_platform_admin) {
    const allTenants = await posRequest<TenantChoice[]>(
      'auth/platform/tenants',
    );
    platformTenants.value = isSuccess(allTenants.body)
      ? allTenants.body.data
      : [];
    tenantOptions.value = platformTenants.value;
    return;
  }
  tenantOptions.value = profile.value.tenants;
}

async function onTenantChange(event: Event): Promise<void> {
  const target = event.target as HTMLSelectElement;
  errorMessage.value = '';
  const result = await posRequest<{
    access_token?: string;
    refresh_token?: string;
    tenant_id: string;
    user: { full_name: string };
  }>('auth/switch-tenant', {
    method: 'POST',
    body: { tenant_id: target.value },
  });
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
  posSession.storeId = null;
  await loadHome();
}

function onStoreChange(event: Event): void {
  const target = event.target as HTMLSelectElement;
  posSession.storeId = target.value || null;
}

async function sendInvite(): Promise<void> {
  message.value = '';
  errorMessage.value = '';
  const result = await posRequest<unknown>('auth/platform/invite', {
    method: 'POST',
    body: { email: inviteEmail.value },
  });
  if (!isSuccess(result.body)) {
    errorMessage.value = result.body.message;
    return;
  }
  message.value = result.body.message;
  inviteEmail.value = '';
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
    <IonHeader>
      <IonToolbar>
        <AppLogo />
      </IonToolbar>
    </IonHeader>
    <IonContent class="ion-padding">
      <div class="mx-auto flex w-full max-w-xl flex-col gap-6 py-4">
        <SaleSession :store-name="selectedStoreName" />

        <div class="flex flex-col gap-1">
          <h1 class="text-ink-primary text-2xl font-medium">
            {{ profile?.user.full_name }}
          </h1>
          <p class="text-ink-secondary text-sm">{{ profile?.user.email }}</p>
        </div>

        <label class="flex flex-col gap-2">
          <span class="text-ink-secondary text-sm font-semibold">Tenant</span>
          <select
            class="border-border-soft bg-surface-canvas rounded-full border px-4 py-3"
            :value="posSession.tenantId ?? ''"
            @change="onTenantChange"
          >
            <option
              v-for="tenant in tenantOptions"
              :key="tenant.id"
              :value="tenant.id"
            >
              {{ tenant.name }}
            </option>
          </select>
        </label>

        <label class="flex flex-col gap-2">
          <span class="text-ink-secondary text-sm font-semibold">Store</span>
          <select
            class="border-border-soft bg-surface-canvas rounded-full border px-4 py-3"
            :value="posSession.storeId ?? ''"
            @change="onStoreChange"
          >
            <option value="">No store selected</option>
            <option v-for="store in stores" :key="store.id" :value="store.id">
              {{ store.name }}
            </option>
          </select>
        </label>

        <form
          v-if="profile?.is_platform_admin"
          class="flex flex-col gap-3"
          @submit.prevent="sendInvite"
        >
          <h2 class="text-ink-primary text-lg font-semibold">
            Invite platform staff
          </h2>
          <input
            v-model="inviteEmail"
            type="email"
            required
            placeholder="Email"
            class="border-border-soft rounded-full border px-4 py-3"
          />
          <button
            type="submit"
            class="bg-brand text-ink-inverse rounded-full px-4 py-3 text-sm font-semibold"
          >
            Send invite
          </button>
        </form>

        <p v-if="errorMessage" class="text-danger text-sm">
          {{ errorMessage }}
        </p>
        <p v-if="message" class="text-success text-sm">{{ message }}</p>

        <IonButton fill="outline" @click="logout">Log out</IonButton>
      </div>
    </IonContent>
  </IonPage>
</template>
