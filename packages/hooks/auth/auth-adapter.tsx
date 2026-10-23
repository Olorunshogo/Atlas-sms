import { createContext, useContext, type ReactNode } from 'react';

/*
 * Dependency injection boundary for the shared auth hooks.
 *
 * The hooks live here so any consumer can import them, but this package must never import the API client, a router, or an app's session context - doing so would drag transport and routing into a UI package and undo the boundary the auth refactor exists to create.
 *
 * So the app supplies these once via AuthAdapterProvider, and the hooks stay pure: form state, validation, error mapping, and nothing else.
 */

export type LoginVariant = 'public' | 'admin';

/*
 * `accepted_terms`, `terms_document` and `terms_version` record which document the account holder agreed to, and when it was current.
 * Sent rather than assumed: each role has its own terms, so the role alone does not identify the text, and a version-less record cannot show whether a later revision was ever seen.
 */
export interface RegisterPayload {
  first_name: string;
  last_name: string;
  email: string;
  password: string;
  role: string;
  phone?: string;
  referred_by_agent_code?: string;
  accepted_terms?: boolean;
  terms_document?: string;
  terms_version?: string;
}

export interface VerifiedSession {
  accessToken?: string;
  refreshToken?: string;
}

/*
 * `login` resolves to the authenticated user's role.
 * `updatePassword` is for a user who knows their current password and wants to set a new one.
 * `storeSession` persists a session returned by email verification, if one was issued.
 * `navigate` navigates, so this package never imports a router.
 * `redirectAfterAuth` sends the user to the right place for their role after authenticating.
 * `splitFullName` splits a single full-name field into the first/last the API expects.
 */
export interface AuthAdapter {
  login: (
    email: string,
    password: string,
    variant: LoginVariant,
  ) => Promise<string>;
  register: (payload: RegisterPayload) => Promise<unknown>;
  forgotPassword: (email: string) => Promise<unknown>;
  resetPassword: (token: string, password: string) => Promise<unknown>;
  updatePassword: (
    currentPassword: string,
    newPassword: string,
  ) => Promise<unknown>;
  verifyEmail: (email: string, otp: string) => Promise<VerifiedSession>;
  resendOtp: (email: string) => Promise<unknown>;
  storeSession: (session: VerifiedSession) => string | null;
  navigate: (path: string, state?: Record<string, unknown>) => void;
  redirectAfterAuth: (role: string) => void;
  splitFullName: (fullName: string) => {
    first_name: string;
    last_name: string;
  };
}

const AuthAdapterContext = createContext<AuthAdapter | null>(null);

export interface AuthAdapterProviderProps {
  adapter: AuthAdapter;
  children: ReactNode;
}

export function AuthAdapterProvider({
  adapter,
  children,
}: AuthAdapterProviderProps) {
  return (
    <AuthAdapterContext.Provider value={adapter}>
      {children}
    </AuthAdapterContext.Provider>
  );
}

export function useAuthAdapter(): AuthAdapter {
  const adapter = useContext(AuthAdapterContext);
  if (!adapter) {
    throw new Error(
      'Auth hooks require <AuthAdapterProvider>. Wrap your app in it and supply an adapter.',
    );
  }
  return adapter;
}
