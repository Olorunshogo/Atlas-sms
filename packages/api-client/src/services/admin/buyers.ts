import { apiFetch } from '../../apiFetch';

/*
 * Admin actions on a buyer account.
 * Blocking is permanent, suspension is a hold - they are separate fields on the user row, so each has its own pair of endpoints rather than one status enum.
 */

export function blockBuyer(id: number): Promise<null> {
  return apiFetch<null>(`/admin/buyers/${id}/block`, { method: 'PATCH' });
}

export function unblockBuyer(id: number): Promise<null> {
  return apiFetch<null>(`/admin/buyers/${id}/unblock`, { method: 'PATCH' });
}

export function suspendBuyer(id: number): Promise<null> {
  return apiFetch<null>(`/admin/buyers/${id}/suspend`, { method: 'PATCH' });
}

export function unsuspendBuyer(id: number): Promise<null> {
  return apiFetch<null>(`/admin/buyers/${id}/unsuspend`, { method: 'PATCH' });
}
