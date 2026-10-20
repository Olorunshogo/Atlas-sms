<script setup lang="ts">
import { useSeoMetaConfig } from '@atlas/composables';
import { apiRequest, isSuccess } from '../utils/api';

definePageMeta({ middleware: 'auth' });

useSeoMetaConfig({
  title: 'Dashboard - Atlas',
  description: 'Owner and manager dashboard.',
  ogUrl: 'https://app.atlas.com/dashboard',
});

const session = useSession();
const switchError = ref<string>('');
const inviteEmail = ref<string>('');
const inviteMessage = ref<string>('');
const inviteError = ref<string>('');

const selectedTenant = computed((): string => {
  return session.profile.value?.tenant_id ?? '';
});

async function onTenantChange(event: Event): Promise<void> {
  const target = event.target as HTMLSelectElement;
  switchError.value = '';
  const message = await session.switchTenant(target.value);
  if (message) {
    switchError.value = message;
  }
}

async function onStoreChange(event: Event): Promise<void> {
  const target = event.target as HTMLSelectElement;
  if (!target.value) {
    await session.clearStore();
    return;
  }
  await session.selectStore(target.value);
}

async function sendPlatformInvite(): Promise<void> {
  inviteMessage.value = '';
  inviteError.value = '';
  const result = await apiRequest<unknown>('v1/auth/platform/invite', {
    method: 'POST',
    body: { email: inviteEmail.value },
  });
  if (!isSuccess(result.body)) {
    inviteError.value = result.body.message;
    return;
  }
  inviteMessage.value = result.body.message;
  inviteEmail.value = '';
}
</script>

<template>
  <main class="mx-auto flex w-full max-w-5xl flex-col gap-8 px-4 py-16 lg:px-8">
    <div class="flex flex-col gap-2">
      <p class="text-sm font-semibold tracking-widest text-ink-muted uppercase">Dashboard</p>
      <h1 class="text-4xl font-medium">{{ session.currentTenant.value?.name ?? 'Your business' }}</h1>
      <p class="text-ink-secondary">{{ session.profile.value?.user.full_name }} · {{ session.profile.value?.user.email }}</p>
    </div>

    <section class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <label class="flex flex-col gap-2 rounded-3xl border border-border-soft bg-surface-raised p-6">
        <span class="text-sm font-semibold text-ink-secondary">Tenant</span>
        <select
          class="rounded-full border border-border-soft bg-surface-canvas px-4 py-3"
          :value="selectedTenant"
          @change="onTenantChange"
        >
          <option v-for="tenant in session.tenantOptions.value" :key="tenant.id" :value="tenant.id">
            {{ tenant.name }}
          </option>
        </select>
        <p v-if="switchError" class="text-sm text-danger">{{ switchError }}</p>
        <p v-if="session.profile.value?.is_platform_admin" class="text-sm text-ink-muted">
          Platform staff can enter any tenant.
        </p>
      </label>

      <label class="flex flex-col gap-2 rounded-3xl border border-border-soft bg-surface-raised p-6">
        <span class="text-sm font-semibold text-ink-secondary">Store</span>
        <select
          class="rounded-full border border-border-soft bg-surface-canvas px-4 py-3"
          :value="session.storeId.value ?? ''"
          @change="onStoreChange"
        >
          <option value="">All stores in view</option>
          <option v-for="store in session.stores.value" :key="store.id" :value="store.id">
            {{ store.name }}
          </option>
        </select>
        <p class="text-sm text-ink-muted">This selection is sent as x-store-id on later requests.</p>
      </label>
    </section>

    <section
      v-if="session.profile.value?.is_platform_admin"
      class="flex flex-col gap-4 rounded-3xl border border-border-soft p-6"
    >
      <h2 class="text-xl font-semibold">Invite platform staff</h2>
      <p class="text-ink-secondary">Only platform admins can add another. A new email gets an invite link. An existing account gets the grant immediately.</p>
      <form class="flex flex-col gap-3 sm:flex-row" @submit.prevent="sendPlatformInvite">
        <input
          v-model="inviteEmail"
          type="email"
          required
          placeholder="Email"
          class="flex-1 rounded-full border border-border-soft px-4 py-3"
        />
        <button type="submit" class="rounded-full bg-brand px-4 py-3 text-sm font-semibold text-ink-inverse">
          Send invite
        </button>
      </form>
      <p v-if="inviteError" class="text-sm text-danger">{{ inviteError }}</p>
      <p v-if="inviteMessage" class="text-sm text-success">{{ inviteMessage }}</p>
    </section>

    <button type="button" class="w-fit rounded-full border border-border-inked px-4 py-3 text-sm font-semibold" @click="session.logout()">
      Log out
    </button>
  </main>
</template>
