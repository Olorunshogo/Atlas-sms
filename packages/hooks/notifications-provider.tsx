import { createContext, useContext, type ReactNode } from 'react';
import {
  useNotificationsService,
  type NotificationRole,
  type UseNotificationsServiceResult,
} from './use-notifications-service';

/*
 * Without this, the bell dropdown and the notifications page each ran their
 * own useNotificationsService, so marking an item read in the bell updated
 * the server but left the page's separate copy showing it as unread until a
 * full remount. One provider above both means one fetch, one array, both
 * surfaces agree.
 */
const NotificationsContext =
  createContext<UseNotificationsServiceResult | null>(null);

function NotificationsProviderInner({
  role,
  children,
}: {
  role: NotificationRole;
  children: ReactNode;
}) {
  const value = useNotificationsService({ role });
  return (
    <NotificationsContext.Provider value={value}>
      {children}
    </NotificationsContext.Provider>
  );
}

/*
 * `role` is nullable because not every signed-in user gets a notification
 * feed (buyer-admin has no feed of its own). useNotificationsService cannot
 * be called conditionally, so the branch lives here, one level up, instead
 * of inside the hook.
 */
export function NotificationsProvider({
  role,
  children,
}: {
  role: NotificationRole | null;
  children: ReactNode;
}) {
  if (!role) return <>{children}</>;
  return (
    <NotificationsProviderInner role={role}>
      {children}
    </NotificationsProviderInner>
  );
}

export function useNotificationsContext(): UseNotificationsServiceResult {
  const ctx = useContext(NotificationsContext);
  if (!ctx) {
    throw new Error(
      'useNotificationsContext must be used within a NotificationsProvider',
    );
  }
  return ctx;
}
