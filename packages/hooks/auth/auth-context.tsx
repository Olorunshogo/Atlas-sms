import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from 'react';
import {
  getAccessToken,
  decodeJwtPayload,
  storeTokens,
  logout as apiLogout,
  login as loginRequest,
  adminLogin as adminLoginRequest,
} from '@atlas/reference-api-client';
import type { JwtPayload } from '@atlas/reference-api-client';

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

export interface AuthContextType {
  user: AuthUser | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (
    email: string,
    password: string,
    variant?: 'public' | 'admin',
  ) => Promise<AuthUser['role']>;
  logout: () => Promise<void>;
  syncUserFromToken: () => AuthUser | null;
  dashboardPath: string;
}

const AuthContext = createContext<AuthContextType | null>(null);

function readUserFromToken(): AuthUser | null {
  const token = getAccessToken();
  if (!token) return null;
  const payload = decodeJwtPayload<
    JwtPayload & {
      exp?: number;
      admin_tier?: 'super' | 'sub';
      admin_desk?: 'buyer' | 'agent' | 'hr' | 'tech';
    }
  >(token);
  if (!payload) return null;
  if (payload.exp && payload.exp * 1000 < Date.now()) return null;
  return {
    sub: payload.sub as unknown as number,
    email: payload.email ?? '',
    role: (payload.role as AuthUser['role']) ?? 'buyer',
    admin_tier: payload.admin_tier,
    admin_desk: payload.admin_desk,
  };
}

function dashboardForRole(role: string, _adminTier?: string): string {
  switch (role) {
    case 'admin':
      /*
       * One admin dashboard for both tiers; the nav narrows for a sub-admin rather than sending them somewhere else.
       * A second top-level dashboard meant two places disagreed about what a sub-admin is.
       * `tiers` in use-dashboard-nav is the single answer, and a domain surface lives at /admin-dashboard/<domain> inside this one.
       */
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

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  const syncUserFromToken = useCallback((): AuthUser | null => {
    const nextUser = readUserFromToken();
    setUser(nextUser);
    return nextUser;
  }, []);

  useEffect(() => {
    syncUserFromToken();
    setIsLoading(false);
  }, [syncUserFromToken]);

  const login = useCallback(
    async (
      email: string,
      password: string,
      variant: 'public' | 'admin' = 'public',
    ): Promise<AuthUser['role']> => {
      /*
       * The transport lives in the reference API client.
       * This context only stores the tokens and derives the session, so moving auth to HttpOnly cookies later is a change inside api-client, not here.
       *
       * ApiError already carries the server's message verbatim, including on 401.
       * That matters because the backend returns 401 for five different conditions: bad credentials, missing password, unverified email, and agent-approval pending or rejected.
       * This previously overrode all of them with "Invalid email or password", which told users with a real, valid account that their password was wrong.
       *
       * Passing the server's wording through does not leak account existence: the backend deliberately returns the same "Invalid credentials" string for both an unknown email and a wrong password.
       */
      const request = variant === 'admin' ? adminLoginRequest : loginRequest;
      const session = await request({ email, password });

      storeTokens(session.accessToken, session.refreshToken);
      const nextUser = syncUserFromToken();
      return nextUser?.role ?? 'buyer';
    },
    [syncUserFromToken],
  );

  const logout = useCallback(async () => {
    await apiLogout();
    setUser(null);
  }, []);

  const dashboardPath = user
    ? dashboardForRole(user.role, user.admin_tier)
    : '/buyer-dashboard';

  return (
    <AuthContext.Provider
      value={{
        user,
        isAuthenticated: user !== null,
        isLoading,
        login,
        logout,
        syncUserFromToken,
        dashboardPath,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextType {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used inside <AuthProvider>');
  return ctx;
}
