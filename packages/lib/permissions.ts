// === Types

export type Permission =
  | 'profile:read'
  | 'policies:read'
  | 'contracts:read'
  | 'reports:submit'
  | 'performance:read'
  | 'directory:read'
  | 'outreach:read'
  | 'outreach:report';

export type PermissionRole = 'employee' | 'customer_care' | 'tech' | 'admin';

// === Role defaults

const EMPLOYEE_PERMISSIONS: readonly Permission[] = [
  'profile:read',
  'policies:read',
  'contracts:read',
  'reports:submit',
  'performance:read',
];

// Track roles are employees first; extras are what each track adds.
const ROLE_PERMISSIONS: Readonly<
  Record<Exclude<PermissionRole, 'admin'>, readonly Permission[]>
> = {
  employee: EMPLOYEE_PERMISSIONS,
  customer_care: [...EMPLOYEE_PERMISSIONS, 'outreach:read', 'outreach:report'],
  tech: EMPLOYEE_PERMISSIONS,
};

/*
 * A sub-admin's desk is which department they oversee in this app - "agent"
 * doubles as the customer-care head (same desk as the admin app's
 * field-agent desk, reused rather than inventing a second one for the same
 * person), "tech" is engineering. Each gets read access to their own
 * department's directory; a desk with no employee-app meaning (buyer) gets
 * nothing here.
 *
 * "hr" gets directory:read only - the roster (now including date of birth)
 * is HR's job to see, but HR doesn't review reports or performance for
 * departments it doesn't run.
 */
const DESK_PERMISSIONS: Readonly<
  Record<'agent' | 'tech' | 'hr', readonly Permission[]>
> = {
  /*
   * reports:submit / performance:read also gate the Reports and Performance
   * nav items for a desk admin, who reviews rather than submits - reusing
   * the existing keys rather than adding reports:review/performance:review
   * since a NavItem only ever checks one permission, and the pages
   * themselves branch into review UI for an admin role regardless of which
   * permission let them in.
   */
  agent: ['directory:read', 'reports:submit', 'performance:read'],
  tech: ['directory:read', 'reports:submit', 'performance:read'],
  hr: ['directory:read'],
};

// Super admin sees everything, so it needs no list.
// Role-derived for now; moves server-side once admins need custom mixes.
export function permissionsForRole(
  role: string | undefined | null,
  adminTier?: 'super' | 'sub' | null,
  adminDesk?: 'buyer' | 'agent' | 'hr' | 'tech' | null,
): readonly Permission[] | 'all' {
  if (role === 'admin') {
    if (adminTier === 'super') return 'all';
    if (adminDesk === 'agent' || adminDesk === 'tech' || adminDesk === 'hr') {
      return DESK_PERMISSIONS[adminDesk];
    }
    return [];
  }
  if (role === 'employee' || role === 'customer_care' || role === 'tech') {
    return ROLE_PERMISSIONS[role];
  }
  return [];
}

export function hasPermission(
  granted: readonly Permission[] | 'all',
  needed: Permission,
): boolean {
  return granted === 'all' || granted.includes(needed);
}
