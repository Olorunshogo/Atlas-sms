/*
 * RBAC model from the reference source - ported structurally, not semantically. These
 * roles (employee/customer_care/tech/admin, agent/tech/hr desks) are
 * that source's own org chart, nothing like Atlas's actual roles (Owner, Admin,
 * Store Manager, Cashier, Inventory Officer, Finance Officer - see
 * docs/RolesAndPermissions.md and the role_grants/roles tables). Atlas
 * authorization also lives server-side via assert_permission and RLS, not a
 * client-side permission table like this one. Prune or rewrite before use -
 * kept here only so the pattern (permission union + role-to-permission maps
 * + a permissionsForRole/hasPermission pair) is available as reference.
 */

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

const ROLE_PERMISSIONS: Readonly<
  Record<Exclude<PermissionRole, 'admin'>, readonly Permission[]>
> = {
  employee: EMPLOYEE_PERMISSIONS,
  customer_care: [...EMPLOYEE_PERMISSIONS, 'outreach:read', 'outreach:report'],
  tech: EMPLOYEE_PERMISSIONS,
};

const DESK_PERMISSIONS: Readonly<
  Record<'agent' | 'tech' | 'hr', readonly Permission[]>
> = {
  agent: ['directory:read', 'reports:submit', 'performance:read'],
  tech: ['directory:read', 'reports:submit', 'performance:read'],
  hr: ['directory:read'],
};

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
