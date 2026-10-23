import {
  computed,
  inject,
  provide,
  ref,
  type ComputedRef,
  type InjectionKey,
  type Ref,
} from 'vue';

/*
 * Ported from the reference source's React auth-context.tsx (useState/useEffect/useContext
 * -> ref/provide/inject). NEEDS RECONCILING WITH ATLAS'S ACTUAL SESSION MODEL
 * before use: this file assumes a client-readable JWT access token, which is
 * how the reference API client worked. Atlas does not work that way -
 * apps/atlas-web/app/composables/use-session.ts relies on an HttpOnly
 * cookie refreshed through the Nitro proxy (no client-side token at all), and
 * apps/atlas-pos/src/auth/session.ts keeps the access token in memory, not a
 * decoded JWT claims object. None of Atlas's real auth composables
 * (use-login.ts etc. in this folder) depend on this file - they go through
 * useAuthAdapter() instead, matching how the reference source's own hooks already bypassed
 * this file. Kept for parity with the source tree; likely to be deleted
 * once the auth-pages extraction work reconciles session handling for real.
 */

export interface AuthUser {
  sub: number;
  email: string;
  role:
    | 'buyer'
    | 'agent'
    | 'admin'
    | 'applicant'
    | 'employee'
    | 'customer_care'
    | 'tech';
  admin_tier?: 'super' | 'sub';
  admin_desk?: 'buyer' | 'agent' | 'hr' | 'tech';
}

export interface AuthSessionTransport {
  getAccessToken: () => string | null;
  decodeJwtPayload: <T>(token: string) => T | null;
  storeTokens: (accessToken: string, refreshToken: string) => void;
  login: (
    email: string,
    password: string,
  ) => Promise<{ accessToken: string; refreshToken: string }>;
  adminLogin: (
    email: string,
    password: string,
  ) => Promise<{ accessToken: string; refreshToken: string }>;
  logout: () => Promise<void>;
}

export interface AuthContextValue {
  user: Ref<AuthUser | null>;
  isAuthenticated: ComputedRef<boolean>;
  isLoading: Ref<boolean>;
  login: (
    email: string,
    password: string,
    variant?: 'public' | 'admin',
  ) => Promise<AuthUser['role']>;
  logout: () => Promise<void>;
  syncUserFromToken: () => AuthUser | null;
  dashboardPath: ComputedRef<string>;
}

interface DecodedJwtClaims {
  sub?: unknown;
  email?: string;
  role?: AuthUser['role'];
  exp?: number;
  admin_tier?: AuthUser['admin_tier'];
  admin_desk?: AuthUser['admin_desk'];
}

const AUTH_CONTEXT_KEY: InjectionKey<AuthContextValue> =
  Symbol('AtlasAuthContext');

function dashboardForRole(role: string): string {
  switch (role) {
    case 'admin':
      return '/admin-dashboard';
    case 'agent':
      return '/agent-dashboard';
    case 'applicant':
    case 'employee':
    case 'customer_care':
      return '/careers-dashboard';
    default:
      return '/buyer-dashboard';
  }
}

export function provideAuthContext(
  transport: AuthSessionTransport,
): AuthContextValue {
  const user = ref<AuthUser | null>(null);
  const isLoading = ref<boolean>(true);

  function readUserFromToken(): AuthUser | null {
    const token = transport.getAccessToken();
    if (!token) return null;

    const payload = transport.decodeJwtPayload<DecodedJwtClaims>(token);
    if (!payload) return null;
    if (payload.exp && payload.exp * 1000 < Date.now()) return null;

    return {
      sub: payload.sub as number,
      email: payload.email ?? '',
      role: payload.role ?? 'buyer',
      admin_tier: payload.admin_tier,
      admin_desk: payload.admin_desk,
    };
  }

  function syncUserFromToken(): AuthUser | null {
    const nextUser = readUserFromToken();
    user.value = nextUser;
    return nextUser;
  }

  syncUserFromToken();
  isLoading.value = false;

  async function login(
    email: string,
    password: string,
    variant: 'public' | 'admin' = 'public',
  ): Promise<AuthUser['role']> {
    const request =
      variant === 'admin' ? transport.adminLogin : transport.login;
    const session = await request(email, password);
    transport.storeTokens(session.accessToken, session.refreshToken);
    const nextUser = syncUserFromToken();
    return nextUser?.role ?? 'buyer';
  }

  async function logout(): Promise<void> {
    await transport.logout();
    user.value = null;
  }

  const isAuthenticated = computed<boolean>(() => user.value !== null);
  const dashboardPath = computed<string>(() =>
    user.value ? dashboardForRole(user.value.role) : '/buyer-dashboard',
  );

  const context: AuthContextValue = {
    user,
    isAuthenticated,
    isLoading,
    login,
    logout,
    syncUserFromToken,
    dashboardPath,
  };

  provide(AUTH_CONTEXT_KEY, context);
  return context;
}

export function useAuthContext(): AuthContextValue {
  const context = inject(AUTH_CONTEXT_KEY);
  if (!context) {
    throw new Error(
      'useAuthContext() must run within a component tree that called provideAuthContext().',
    );
  }
  return context;
}
