import {
  apiRequest,
  isSuccess,
  SESSION_STATE_KEY,
  type SessionProfile,
  type StoreRecord,
  type TenantChoice,
  type TenantRecord,
} from '../utils/api';

/**
 * One session for the header, the dashboard, and the route guard.
 * Tokens stay in HttpOnly cookies. This only keeps the profile the API is willing to return.
 */
export function useSession() {
  const profile = useState<SessionProfile | null>(
    SESSION_STATE_KEY,
    () => null,
  );
  const loaded = useState<boolean>('atlas-session-loaded', () => false);
  const storeId = useState<string | null>('atlas-store-id', () => null);
  const stores = useState<StoreRecord[]>('atlas-stores', () => []);
  const platformTenants = useState<TenantChoice[]>(
    'atlas-platform-tenants',
    () => [],
  );
  const currentTenant = useState<TenantRecord | null>(
    'atlas-current-tenant',
    () => null,
  );

  async function loadSession(): Promise<void> {
    const session = await apiRequest<SessionProfile>('v1/auth/me');
    if (session.status === 200 && isSuccess(session.body)) {
      profile.value = session.body.data;
    } else {
      profile.value = null;
      stores.value = [];
      platformTenants.value = [];
      currentTenant.value = null;
    }

    try {
      const selected = await $fetch<{ data: { store_id: string | null } }>(
        '/session/store',
      );
      storeId.value = selected.data.store_id;
    } catch {
      storeId.value = null;
    }
    loaded.value = true;

    // Tenant data is closed to an unverified account; the dashboard gate asks for the code instead
    if (profile.value?.user.email_verified) {
      await loadWorkspace();
    }
  }

  async function loadWorkspace(): Promise<void> {
    const tenant = await apiRequest<TenantRecord>('v1/tenants/me');
    currentTenant.value =
      tenant.status === 200 && isSuccess(tenant.body) ? tenant.body.data : null;

    const storeList = await apiRequest<StoreRecord[]>('v1/stores');
    stores.value =
      storeList.status === 200 && isSuccess(storeList.body)
        ? storeList.body.data
        : [];

    if (profile.value?.is_platform_admin) {
      const allTenants = await apiRequest<TenantChoice[]>(
        'v1/auth/platform/tenants',
      );
      platformTenants.value =
        allTenants.status === 200 && isSuccess(allTenants.body)
          ? allTenants.body.data
          : [];
    } else {
      platformTenants.value = [];
    }
  }

  async function switchTenant(tenantId: string): Promise<string | null> {
    const result = await apiRequest<SessionProfile>('v1/auth/switch-tenant', {
      method: 'POST',
      body: { tenant_id: tenantId },
    });
    if (!isSuccess(result.body)) {
      return result.body.message;
    }
    await loadSession();
    return null;
  }

  async function selectStore(nextStoreId: string): Promise<void> {
    await $fetch('/session/store', {
      method: 'POST',
      body: { store_id: nextStoreId },
    });
    storeId.value = nextStoreId;
  }

  async function clearStore(): Promise<void> {
    await $fetch('/session/store', { method: 'DELETE' });
    storeId.value = null;
  }

  async function logout(): Promise<void> {
    await apiRequest('v1/auth/logout', { method: 'POST', body: {} });
    await clearStore();
    profile.value = null;
    stores.value = [];
    platformTenants.value = [];
    currentTenant.value = null;
    loaded.value = true;
    await navigateTo('/auth/login');
  }

  const emailVerified = computed((): boolean => {
    return profile.value?.user.email_verified ?? false;
  });

  const tenantOptions = computed((): TenantChoice[] => {
    if (profile.value?.is_platform_admin && platformTenants.value.length > 0) {
      return platformTenants.value;
    }
    return profile.value?.tenants ?? [];
  });

  return {
    profile,
    loaded,
    storeId,
    stores,
    currentTenant,
    emailVerified,
    tenantOptions,
    loadSession,
    switchTenant,
    selectStore,
    clearStore,
    logout,
  };
}
