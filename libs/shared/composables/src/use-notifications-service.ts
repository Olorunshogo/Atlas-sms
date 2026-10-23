import {
  computed,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
  type ComputedRef,
  type Ref,
} from 'vue';
import { useRouter } from 'vue-router';

/*
 * The one place a dashboard talks to the notifications API - ported from
 * the reference source's React use-notifications-service.ts. Every role gets the same
 * shape back, so a notifications page never learns who is looking at it.
 *
 * NotificationItem/NotificationStatus did not exist anywhere in the source
 * tree (they came from the original app's own components/notifications/types,
 * outside this reference dump) - defined locally below. Role-specific
 * dashboard routes (admin-dashboard, buyer-dashboard, etc.) are that
 * source's own routing, not Atlas's - verify or replace ROLE_CONFIG before use.
 */

export type NotificationRole = 'admin' | 'buyer' | 'agent' | 'employee';
export type NotificationStatus = 'unread' | 'read' | 'done';

export interface NotificationItem {
  id: number;
  type: string;
  title: string;
  description: string;
  created_at: string;
  status: NotificationStatus;
  actionUrl?: string;
}

export interface NotificationsViewProps {
  notifications: Ref<NotificationItem[]>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
  onRetry: () => void;
  onMarkAsRead: (id: number) => void;
  onMarkAsDone: (id: number) => void;
  onMarkAllRead: () => void;
  onItemClick: (item: NotificationItem) => void;
  hasMore: Ref<boolean>;
  loadingMore: Ref<boolean>;
  onLoadMore: () => void;
}

interface RoleConfig {
  base: string;
  listPath: string;
}

const ROLE_CONFIG: Record<NotificationRole, RoleConfig> = {
  admin: {
    base: '/admin/notifications',
    listPath: '/admin-dashboard/notifications',
  },
  buyer: {
    base: '/buyer/notifications',
    listPath: '/buyer-dashboard/notifications',
  },
  agent: {
    base: '/agent/notifications',
    listPath: '/agent-dashboard/notifications',
  },
  employee: {
    base: '/careers/employees/me/notifications',
    listPath: '/employee-dashboard/notifications',
  },
};

/** Rating notifications deep-link to the page that can open the rating sheet. */
const RATING_ACTION_URL: Partial<Record<NotificationRole, string>> = {
  buyer: '/buyer-dashboard/orders?rate=1',
  agent: '/agent-dashboard/deliveries?rate=1',
};

interface ApiNotification {
  id: number;
  type: string;
  title: string;
  description: string;
  created_at: string;
  status: NotificationStatus;
  actionUrl?: string;
}

function mapNotification(
  row: ApiNotification,
  role: NotificationRole,
): NotificationItem {
  const ratingUrl = row.type === 'rating' ? RATING_ACTION_URL[role] : undefined;
  return {
    id: row.id,
    type: row.type,
    title: row.title,
    description: row.description,
    created_at: row.created_at,
    status: row.status,
    ...(row.actionUrl || ratingUrl
      ? { actionUrl: row.actionUrl ?? ratingUrl }
      : {}),
  };
}

/** The bell's dot is read from localStorage on every navigation, so it stays in step with whatever this service last saw. */
const HAS_UNREAD_KEY = 'atlas_has_unread';

function setHasUnreadFlag(unreadCount: number): void {
  try {
    localStorage.setItem(HAS_UNREAD_KEY, unreadCount > 0 ? 'true' : 'false');
  } catch {
    /* Private mode or blocked storage. The dot is a convenience, not state. */
  }
}

export interface UseNotificationsServiceResult extends NotificationsViewProps {
  unreadCount: ComputedRef<number>;
  recent: ComputedRef<NotificationItem[]>;
  listPath: string;
  reload: () => void;
}

/** One page per request; the backend caps `limit` and pages by offset. */
const PAGE_SIZE = 20;

/** How often the badge re-checks the server for a count that changed without this tab doing it. */
const POLL_INTERVAL_MS = 2 * 60 * 1000;

export function useNotificationsService({
  role,
}: {
  role: NotificationRole;
}): UseNotificationsServiceResult {
  const router = useRouter();
  const { base, listPath } = ROLE_CONFIG[role];

  const notifications = ref<NotificationItem[]>([]);
  const loading = ref<boolean>(true);
  const error = ref<string | null>(null);
  const page = ref<number>(1);
  const hasMore = ref<boolean>(false);
  const loadingMore = ref<boolean>(false);
  /** A real server-side count, not derived from `notifications` (that list is capped at one page). */
  const unreadCountValue = ref<number>(0);

  async function load(): Promise<void> {
    loading.value = true;
    try {
      // Replace apiFetch with an @atlas/api atlasRequest call once wired up.
      const rows: ApiNotification[] = await apiFetch(
        `${base}?page=1&limit=${PAGE_SIZE}`,
      );
      notifications.value = rows.map((row) => mapNotification(row, role));
      page.value = 1;
      hasMore.value = rows.length === PAGE_SIZE;
      error.value = null;
    } catch (err) {
      notifications.value = [];
      hasMore.value = false;
      error.value =
        err instanceof Error
          ? err.message
          : 'Could not load notifications. Check your connection and retry.';
    } finally {
      loading.value = false;
    }
  }

  async function loadUnreadCount(): Promise<void> {
    try {
      const { total }: { total: number } = await apiFetch(
        `${base}/unread-count`,
      );
      unreadCountValue.value = total;
    } catch {
      /* Keep the last known count; a failed poll should not blank the badge. */
    }
  }

  async function onLoadMore(): Promise<void> {
    if (loadingMore.value || !hasMore.value) return;
    loadingMore.value = true;
    try {
      const next = page.value + 1;
      const rows: ApiNotification[] = await apiFetch(
        `${base}?page=${next}&limit=${PAGE_SIZE}`,
      );
      const seen = new Set(notifications.value.map((n) => n.id));
      const mapped = rows
        .filter((r) => !seen.has(r.id))
        .map((row) => mapNotification(row, role));
      notifications.value = [...notifications.value, ...mapped];
      page.value = next;
      hasMore.value = rows.length === PAGE_SIZE;
    } catch {
      /* Keep what is already shown; the button stays for another try. */
    } finally {
      loadingMore.value = false;
    }
  }

  function mutate(
    apply: (rows: NotificationItem[]) => NotificationItem[],
    request: () => Promise<unknown>,
  ): void {
    const previous = notifications.value;
    notifications.value = apply(previous);
    request().catch(() => {
      notifications.value = previous;
    });
  }

  function onMarkAsRead(id: number): void {
    const wasUnread = notifications.value.some(
      (n) => n.id === id && n.status === 'unread',
    );
    if (wasUnread)
      unreadCountValue.value = Math.max(0, unreadCountValue.value - 1);
    mutate(
      (rows) =>
        rows.map((n) =>
          n.id === id && n.status === 'unread' ? { ...n, status: 'read' } : n,
        ),
      () =>
        apiFetch(`${base}/${id}/read`, { method: 'PATCH' }).catch(
          (err: unknown) => {
            if (wasUnread) void loadUnreadCount();
            throw err;
          },
        ),
    );
  }

  function onMarkAsDone(id: number): void {
    const wasUnread = notifications.value.some(
      (n) => n.id === id && n.status === 'unread',
    );
    if (wasUnread)
      unreadCountValue.value = Math.max(0, unreadCountValue.value - 1);
    mutate(
      (rows) => rows.map((n) => (n.id === id ? { ...n, status: 'done' } : n)),
      () =>
        apiFetch(`${base}/${id}/done`, { method: 'PATCH' }).catch(
          (err: unknown) => {
            if (wasUnread) void loadUnreadCount();
            throw err;
          },
        ),
    );
  }

  function onMarkAllRead(): void {
    const previousCount = unreadCountValue.value;
    unreadCountValue.value = 0;
    mutate(
      (rows) =>
        rows.map((n) => (n.status === 'unread' ? { ...n, status: 'read' } : n)),
      () =>
        apiFetch(`${base}/mark-all/read`, { method: 'PATCH' }).catch(
          (err: unknown) => {
            unreadCountValue.value = previousCount;
            throw err;
          },
        ),
    );
  }

  function onItemClick(item: NotificationItem): void {
    if (item.actionUrl) router.push(item.actionUrl);
  }

  const recent = computed<NotificationItem[]>(() =>
    notifications.value.filter((n) => n.status !== 'done').slice(0, 5),
  );
  const unreadCount = computed<number>(() => unreadCountValue.value);

  onMounted(() => {
    void load();
    void loadUnreadCount();
  });

  const pollTimer = setInterval(() => {
    void loadUnreadCount();
  }, POLL_INTERVAL_MS);

  onBeforeUnmount(() => clearInterval(pollTimer));

  // Mirrors the source's "sync the badge flag whenever the count settles" effect.
  watch([loading, error, unreadCountValue], () => {
    if (!loading.value && !error.value)
      setHasUnreadFlag(unreadCountValue.value);
  });

  return {
    notifications,
    loading,
    error,
    onRetry: () => void load(),
    onMarkAsRead,
    onMarkAsDone,
    onMarkAllRead,
    onItemClick,
    unreadCount,
    recent,
    listPath,
    reload: () => void load(),
    hasMore,
    loadingMore,
    onLoadMore: () => void onLoadMore(),
  };
}

/**
 * Placeholder transport - replace with `atlasRequest` from `@atlas/api`
 * before this service is wired into an actual Atlas app. Kept as a typed
 * stub so the composable above compiles standalone during this conversion
 * pass.
 */
async function apiFetch<T>(
  _path: string,
  _init?: { method?: string },
): Promise<T> {
  throw new Error(
    'useNotificationsService: wire apiFetch to @atlas/api before use.',
  );
}
