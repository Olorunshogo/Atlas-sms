<script setup lang="ts">
import { usePageSeo } from '../../composables/use-page-seo';
import {
  AppLogo,
  DataTable,
  type RowAction,
  type TableColumn,
} from '@atlas/ui';
import { DashboardFrame } from '@atlas/ui-web';
import type { StoreRecord } from '../../utils/api';

definePageMeta({ layout: 'dashboard', middleware: 'auth' });

usePageSeo({
  title: 'Dashboard',
  description: 'Owner and manager dashboard.',
  path: '/dashboard',
  noIndex: true,
});

const session = useSession();

const storeLabel = computed((): string => {
  const selected = session.stores.value.find((store) => {
    return store.id === session.storeId.value;
  });
  return selected?.name ?? 'Main store';
});

const storeColumns: TableColumn<StoreRecord>[] = [
  {
    id: 'name',
    header: 'Store',
    cell: (row) => row.name,
    sortable: true,
    sortValue: (row) => row.name,
    searchValue: (row) => row.name,
  },
  {
    id: 'slug',
    header: 'Slug',
    cell: (row) => row.slug,
    searchValue: (row) => row.slug,
  },
  {
    id: 'currency',
    header: 'Currency',
    cell: (row) => row.currency,
  },
  {
    id: 'status',
    header: 'Status',
    cell: (row) => (row.is_active ? 'Active' : 'Inactive'),
    sortable: true,
    sortValue: (row) => (row.is_active ? 'Active' : 'Inactive'),
  },
];

const storeActions: RowAction<StoreRecord>[] = [
  {
    id: 'select',
    label: 'Use this store',
    confirm: {
      dialogKey: 'CONFIRM',
      props: (row) => {
        return {
          title: 'Use this store',
          body: row.name + ' will be sent as x-store-id on later requests.',
          confirmLabel: 'Use store',
          onConfirm: async () => {
            await session.selectStore(row.id);
          },
        };
      },
    },
  },
];
const switchError = ref<string>('');
const inviteOpen = ref<boolean>(false);
const inviteMessage = ref<string>('');

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

function openInvite(): void {
  inviteMessage.value = '';
  inviteOpen.value = true;
}
</script>

<template>
  <main
    class="section-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg flex w-full flex-col gap-8"
  >
    <DashboardFrame :store-name="storeLabel">
      <template #logo>
        <AppLogo />
      </template>
    </DashboardFrame>

    <div class="flex flex-col gap-2">
      <p class="text-ink-muted text-sm font-semibold tracking-widest uppercase">
        Dashboard
      </p>
      <h1 class="text-4xl font-medium">
        {{ session.currentTenant.value?.name ?? 'Your business' }}
      </h1>
      <p class="text-ink-secondary">
        {{ session.profile.value?.user.full_name }} ·
        {{ session.profile.value?.user.email }}
      </p>
    </div>

    <section class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <label
        class="border-border-soft bg-surface-raised flex flex-col gap-2 rounded-3xl border p-6"
      >
        <span class="text-ink-secondary text-sm font-semibold">Tenant</span>
        <select
          class="border-border-soft bg-surface-canvas rounded-full border px-4 py-3"
          :value="selectedTenant"
          @change="onTenantChange"
        >
          <option
            v-for="tenant in session.tenantOptions.value"
            :key="tenant.id"
            :value="tenant.id"
          >
            {{ tenant.name }}
          </option>
        </select>
        <p v-if="switchError" class="text-danger text-sm">{{ switchError }}</p>
        <p
          v-if="session.profile.value?.is_platform_admin"
          class="text-ink-muted text-sm"
        >
          Platform staff can enter any tenant.
        </p>
      </label>

      <label
        class="border-border-soft bg-surface-raised flex flex-col gap-2 rounded-3xl border p-6"
      >
        <span class="text-ink-secondary text-sm font-semibold">Store</span>
        <select
          class="border-border-soft bg-surface-canvas rounded-full border px-4 py-3"
          :value="session.storeId.value ?? ''"
          @change="onStoreChange"
        >
          <option value="">All stores in view</option>
          <option
            v-for="store in session.stores.value"
            :key="store.id"
            :value="store.id"
          >
            {{ store.name }}
          </option>
        </select>
        <p class="text-ink-muted text-sm">
          This selection is sent as x-store-id on later requests.
        </p>
      </label>
    </section>

    <section
      v-if="session.profile.value?.is_platform_admin"
      class="border-border-soft flex flex-col gap-4 rounded-3xl border p-6"
    >
      <h2 class="text-xl font-semibold">Invite platform staff</h2>
      <p class="text-ink-secondary">Only platform admins can add another.</p>
      <button
        type="button"
        class="bg-brand text-ink-inverse w-fit rounded-full px-4 py-3 text-sm font-semibold"
        @click="openInvite"
      >
        Invite platform staff
      </button>
      <p v-if="inviteMessage" class="text-success text-sm">
        {{ inviteMessage }}
      </p>
      <InvitePlatformStaffModal
        v-model:open="inviteOpen"
        @invited="inviteMessage = $event"
      />
    </section>

    <section class="flex flex-col gap-4">
      <h2 class="text-xl font-semibold">Stores</h2>
      <DataTable
        :rows="session.stores.value"
        :columns="storeColumns"
        :actions="storeActions"
        caption="Stores"
        empty-label="No stores yet."
      />
    </section>

    <button
      type="button"
      class="border-border-inked w-fit border px-4 py-3 text-sm font-semibold"
      @click="session.logout()"
    >
      Log out
    </button>
  </main>
</template>
