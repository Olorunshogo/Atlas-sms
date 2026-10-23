/*
 * "3h ago" for anything recent, an absolute date once that stops being useful.
 *
 * A week is the cutoff: past that, "12d ago" makes a reader do arithmetic to
 * work out a date they could simply have been told.
 */
export function formatRelativeTime(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return '';

  const diff = Date.now() - at.getTime();

  /* A clock skew between server and client can put a timestamp slightly in the
     future; "in 2 minutes" for a notification that just arrived reads as a bug. */
  if (diff < 60_000) return 'Just now';

  const minutes = Math.floor(diff / 60_000);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(diff / 3_600_000);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(diff / 86_400_000);
  if (days < 7) return `${days}d ago`;

  return at.toLocaleDateString('en-NG', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}
