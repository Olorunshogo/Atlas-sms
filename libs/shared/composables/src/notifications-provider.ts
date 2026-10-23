import { inject, provide, type InjectionKey } from 'vue';
import {
  useNotificationsService,
  type NotificationRole,
  type UseNotificationsServiceResult,
} from './use-notifications-service';

/*
 * Ported from the reference source's React notifications-provider.tsx. Without this, a
 * bell dropdown and a notifications page would each run their own
 * useNotificationsService - one provider above both means one fetch, one
 * reactive state, both surfaces agree.
 */

const NOTIFICATIONS_CONTEXT_KEY: InjectionKey<UseNotificationsServiceResult> =
  Symbol('AtlasNotificationsContext');

/**
 * `role` is nullable because not every signed-in user gets a notification
 * feed. Call this from a top-level layout component's setup(); when role is
 * null, nothing is provided and useNotificationsContext() will throw if a
 * descendant calls it - guard with a v-if in the layout instead.
 */
export function provideNotifications(
  role: NotificationRole | null,
): UseNotificationsServiceResult | null {
  if (!role) return null;
  const value = useNotificationsService({ role });
  provide(NOTIFICATIONS_CONTEXT_KEY, value);
  return value;
}

export function useNotificationsContext(): UseNotificationsServiceResult {
  const context = inject(NOTIFICATIONS_CONTEXT_KEY);
  if (!context) {
    throw new Error(
      'useNotificationsContext() must run within a component tree that called provideNotifications().',
    );
  }
  return context;
}
